use std::{
	fs,
	io::Cursor,
	path::{ Path, PathBuf },
	process::Command
};
use nus3audio::Nus3audioFile;
use fltk::{
	prelude::{
		BrowserExt,
		WidgetBase,
		WidgetExt
	},
	browser::Browser
};
use rodio::Source;
use crate::settings::CACHEDIR;

/// A particular list.
pub struct List {
	pub name: String,
	pub path: Option<PathBuf>,
	pub items: Vec<ListItem>,
	widget: Browser
}

impl List {
	// pub fn new(sender: fltk::app::Sender<crate::Message>) -> Self {
	pub fn new() -> Self {
		let mut widget = Browser::new(0, 0, 0, 0, "");
		widget.set_type(fltk::browser::BrowserType::Hold);
		// widget.set_callback(move |c| c.emit(sender, crate::Message::ListInteracted));
		Self {
			name: String::new(),
			path: None,
			items: Vec::new(),
			widget
		}
	}

	/// Clear the items in this list.
	pub fn clear(&mut self) {
		self.items.clear();
		self.widget.clear()
	}

	/// Save this nus3audio to the file at `self.path`.
	pub fn save_nus3audio(&mut self, path: Option<&Path>, vgaudio_cli: &str) -> Result<(), String> {
		let path = if let Some(path) = path { path } else { self.path.as_ref().expect("No path has been set to save.") };
		let name = path.file_name().unwrap().to_string_lossy().to_string();
		let mut nus3audio = Nus3audioFile::new();

		let mut index: usize = 0;
		while let Some(sound_name) = self.get_label_of(index) {
				let list_item = self.items.get_mut(index).expect("Failed to find internal list item");

				match list_item.get_idsp_raw(&name, &sound_name, vgaudio_cli) {
					Ok(data) => {
						nus3audio.files.push(
							nus3audio::AudioFile {
								id: list_item.id,
								name: sound_name,
								data
							}
						)
					},
					Err(error) => {
						return Err(format!("Error converting idsp:\n{}", error))
					}
				}
				index += 1
		}

		let mut export: Vec<u8> = Vec::new();
		nus3audio.write(&mut export);

		if let Err(error) = fs::write(path.with_extension("nus3audio"), &export) {
			Err(error.to_string())
		} else {
			Ok(())
		}
	}

	/// Redraw the widget of this list.
	pub fn redraw(&mut self) {
		self.widget.redraw()
	}

	/// Returns the selected value of this list, if one is selected.
	pub fn selected(&mut self) -> Option<(usize, String)> {
		let value = self.widget.value();
		// 0 is returned if there is no value selected, but
		// I'm not sure if this value is ever negative
		if value != 0 { Some((value as usize - 1, self.widget.text(value).unwrap())) } else { None }
	}

	pub fn get_label_of(&self, line: usize) -> Option<String> {
		self.widget.text(line as i32 + 1)
	}

	/// Returns the [&mut Browser] widget of this List.
	pub fn get_widget_mut(&mut self) -> &mut Browser {
		&mut self.widget
	}

	/// Adds an item to the list.
	pub fn add_item(&mut self, item: ListItem, name: &str) {
		self.items.push(item);
		self.widget.add(name)
	}
}

/// An item in a [List].
pub struct ListItem {
	pub id: u32,
	pub idsp_raw: Option<Vec<u8>>,
	pub raw: Option<Vec<u8>>,
	pub loop_points: Option<(usize, usize)>,
	pub bytes_per_sample: u16
}

impl ListItem {
	/// Return a new [ListItem].
	pub fn new(id: u32) -> Self {
		Self {
			id,
			idsp_raw: None,
			raw: None,
			loop_points: None,
			bytes_per_sample: 0
		}
	}

	/// Attach a new raw value to this item.
	pub fn set_raw(&mut self, raw: Vec<u8>) -> Result<(), String> {
		let cursor = Cursor::new(raw);
		let decoder = rodio::Decoder::new(cursor);
		if let Err(error) = decoder {
			return Err(error.to_string())
		};
		let decoder = decoder.unwrap();

		let header = wav::Header::new(wav::WAV_FORMAT_PCM, decoder.channels(), decoder.sample_rate(), 16);

		let decoded: Vec<i16> = decoder.collect();

		self.bytes_per_sample = header.bytes_per_sample;

		let mut written: Vec<u8> = Vec::new();
		let mut cursor = Cursor::new(&mut written);

		wav::write(header, &wav::BitDepth::Sixteen(decoded), &mut cursor).unwrap();

		self.raw = Some(written);
		self.idsp_raw = None;
		self.loop_points = None;
		Ok(())
	}

	/// Attach a new raw idsp value to this item.
	pub fn set_idsp_raw(&mut self, idsp_raw: Vec<u8>) {
		self.idsp_raw = Some(idsp_raw);
		self.raw = None;
		self.loop_points = None
	}

	/// Return a reference to the raw sound from this item. Converts the idsp first if it needs to.
	fn get_raw_internal(&mut self, nus3audio_name: &str, sound_name: &str, vgaudio_cli: &str) -> Result<&Vec<u8>, String> {
		if self.raw.is_some() { return Ok(self.raw.as_ref().unwrap()) }
		if self.idsp_raw.is_none() { unreachable!() }

		// Need to convert the idsp to wav
		let target_dir = CACHEDIR.join(nus3audio_name);
		let src_file = target_dir.join(&format!("{}.idsp", sound_name));
		let dest_file = src_file.with_extension("wav");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, self.idsp_raw.as_ref().unwrap()) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		let raw = self.run_vgaudio_cli(&src_file, &dest_file, vgaudio_cli)?;
		self.raw = Some(raw);

		Ok(self.raw.as_ref().unwrap())
	}

	/// Return the raw sound from this item. Converts the idsp first if it needs to.
	pub fn get_raw(&mut self, nus3audio_name: &str, sound_name: &str, vgaudio_cli: &str) -> Result<Vec<u8>, String> {
		Ok(self.get_raw_internal(nus3audio_name, sound_name, vgaudio_cli)?.clone())
	}

	/// Return the idsp-format sound from this item. Converts the sound first if it needs to.
	pub fn get_idsp_raw(&mut self, nus3audio_name: &str, sound_name: &str, vgaudio_cli: &str) -> Result<Vec<u8>, String> {
		if self.idsp_raw.is_some() { return Ok(self.idsp_raw.as_ref().unwrap().clone()) }
		if self.raw.is_none() { unreachable!() }

		// Need to convert the wav to idsp
		let target_dir = CACHEDIR.join(nus3audio_name);
		let src_file = target_dir.join(&format!("{}.wav", sound_name));
		let dest_file = src_file.with_extension("idsp");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, self.raw.as_ref().unwrap()) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		let idsp_raw = self.run_vgaudio_cli(&src_file, &dest_file, vgaudio_cli)?;
		self.idsp_raw = Some(idsp_raw);

		Ok(self.idsp_raw.as_ref().unwrap().clone())
	}

	/// Try to empty and create the target directory. This should be in the cache directory,
	/// to avoid deleting something we shouldn't.
	pub fn create_target_dir(target_dir: &Path) -> Result<(), std::io::Error> {
		if target_dir.exists() {
			if target_dir.is_dir() {
				let contents = target_dir.read_dir()?;
				for item in contents {
					let item_path = item?.path();
					if item_path.is_dir() {
						fs::remove_dir_all(item_path)?
					} else {
						fs::remove_file(item_path)?
					}
				}
			} else {
				fs::remove_file(target_dir)?;
				fs::create_dir(target_dir)?
			}
		} else {
			fs::create_dir(target_dir)?
		}
		Ok(())
	}

	/// Run VGAudioCli, convert `src_file` to `dest_file` and return it as bytes.
	fn run_vgaudio_cli(&self, src_file: &Path, dest_file: &Path, vgaudio_cli: &str) -> Result<Vec<u8>, String> {
		let mut arg_iter = vgaudio_cli.split(' ');
		let mut command: Command;
		if let Some(arg) = arg_iter.next() {
			command = Command::new(arg)
		} else {
			return Err("VGAudiCli path is empty".to_owned())
		}

		for arg in arg_iter {
			command.arg(arg);
		}

		let output = command
			.arg("-c")
			.arg(src_file.as_os_str())
			.arg(dest_file.as_os_str())
			.output();

		let output = if let Err(error) = output {
			return Err(format!("Error running VGAudioCli\n{}", error))
		} else {
			output.unwrap()
		};

		if let Some(code) = output.status.code() {
			if code != 0 {
				let stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| String::new());
				return Err(format!("VGAudioCli returned exit code {}\n{}", code, stdout))
			}
		} else {
			return Err("VGAudio didn't return any exit code".to_string())
		}

		match fs::read(dest_file) {
			Ok(bytes) => Ok(bytes),
			Err(error) => Err(format!("Error reading destination file {:?}\n{}", dest_file, error))
		}
	}
}
