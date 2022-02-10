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

/// [nus3audio] has AudioFile::filename to do exactly this, but
/// VGAudioCli seems to create lopus files without the header
/// that nus3audio expects
///
/// Therefore, we rewrite that function here minus the fallback
/// to .bin. Yes, this is a hack.
pub fn extension_of_encoded(encoded: &Vec<u8>) -> Result<String, String> {
	Ok(
		if encoded.len() < 4 {
			return Err("Not a valid file".to_owned())
		} else if encoded[..4].eq(b"IDSP") {
			"idsp"
		} else {
			"lopus"
		}.to_owned()
	)
}

/// A particular list.
pub struct List {
	/// The name of this nus3audio file.
	pub name: String,
	/// The path of this list's original nus3audio file.
	pub path: Option<PathBuf>,
	/// Items in this nus3audio file.
	pub items: Vec<ListItem>,
	/// The browser widget representing the file.
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
	pub fn save_nus3audio(&mut self, path: Option<&Path>, settings: &crate::settings::Settings) -> Result<(), String> {
		let path = if let Some(path) = path { path } else { self.path.as_ref().expect("No path has been set to save.") };
		let name = path.file_name().unwrap().to_string_lossy().to_string();
		let mut nus3audio = Nus3audioFile::new();

		for (index, list_item) in self.items.iter_mut().enumerate() {
			let file_name = self.widget.text(index as i32 + 1).expect("Failed to get list item");
			match list_item.get_nus3_encoded_raw(&name, &file_name, settings) {
				Ok(data) => {
					nus3audio.files.push(
						nus3audio::AudioFile {
							id: list_item.id,
							name: list_item.name.to_owned(),
							data
						}
					)
				},
				Err(error) => {
					return Err(format!("Error converting:\n{}", error))
				}
			}
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
	pub name: String,
	/// Raw audio, in wav format.
	pub audio_raw: Option<Vec<u8>>,
	/// Currently unused.
	pub loop_points: Option<(usize, usize)>,
	/// Currently unused.
	pub bytes_per_sample: u16
}

impl ListItem {
	/// Return a new [ListItem].
	pub fn new(id: u32, name: String) -> Self {
		Self {
			id,
			name,
			audio_raw: None,
			loop_points: None,
			bytes_per_sample: 0
		}
	}

	/// Attach a new raw value to this item.
	pub fn set_audio_raw(&mut self, raw: Vec<u8>) -> Result<(), String> {
		let cursor = Cursor::new(raw);
		let decoder = rodio::Decoder::new(cursor);
		if let Err(error) = decoder {
			return Err(error.to_string())
		};
		let decoder = decoder.unwrap();

		let decoder_sample_rate = decoder.sample_rate();
		// The lopus format only supports these sample rates
		let sample_rate = if decoder_sample_rate <= 8_000 {8_000}
		else if decoder_sample_rate <= 12_000 {12_000}
		else if decoder_sample_rate <= 16_000 {16_000}
		else if decoder_sample_rate <= 24_000 {24_000}
		else {48_000};

		let header = wav::Header::new(wav::WAV_FORMAT_PCM, decoder.channels(), sample_rate, 16);

		let mut decoded: Vec<i16> = decoder.collect();

		println!("src rate: {}, target rate: {}", decoder_sample_rate, sample_rate);
		if decoder_sample_rate != sample_rate {
			// Need to resample
			if header.channel_count == 1 {
				let input = fon::Audio::<fon::chan::Ch16, 1>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 1>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			} else {
				let input = fon::Audio::<fon::chan::Ch16, 2>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 2>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			}
		}

		self.bytes_per_sample = header.bytes_per_sample;

		let mut written: Vec<u8> = Vec::new();
		let mut cursor = Cursor::new(&mut written);

		wav::write(header, &wav::BitDepth::Sixteen(decoded), &mut cursor).unwrap();

		self.audio_raw = Some(written);
		self.loop_points = None;
		Ok(())
	}

	/// Gets the sound from an encoded file from a nus3audio file.
	pub fn from_encoded<P>(&mut self, nus3audio_name: &str, encoded: Vec<u8>, sound_name: P, settings: &crate::settings::Settings) -> Result<(), String>
	where P: Into<PathBuf> {
		let target_dir = CACHEDIR.join(nus3audio_name);

		
		let src_file = target_dir.join(sound_name.into()).with_extension(extension_of_encoded(&encoded)?);

		let dest_file = src_file.with_extension("wav");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, encoded) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		let raw = self.run_vgaudio_cli(&src_file, &dest_file, settings)?;
		self.audio_raw = Some(raw);
		self.loop_points = None;

		Ok(())
	}

	/// Return the raw audio from this item. Returns an error if it is [None].
	pub fn get_audio_raw(&mut self) -> Result<Vec<u8>, String> {
		if let Some(raw) = &self.audio_raw {
			Ok(raw.clone())
		} else {
			Err("Audio of selected item is empty".to_owned())
		}
	}

	/// Return the nus3audio-encoded sound from this item. Converts the raw audio.
	pub fn get_nus3_encoded_raw<P>(&mut self, nus3audio_name: &str, sound_name: P, settings: &crate::settings::Settings) -> Result<Vec<u8>, String>
		where P: Into<PathBuf> {
		if self.audio_raw.is_none() { return Err("Audio of selected item is empty".to_owned()) }

		// Need to convert the wav
		let target_dir = CACHEDIR.join(nus3audio_name);
		let dest_file = target_dir.join(sound_name.into());
		let src_file = dest_file.with_extension("wav");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, self.audio_raw.as_ref().unwrap()) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		let nus3_encoded_raw = self.run_vgaudio_cli(&src_file, &dest_file, settings)?;

		Ok(nus3_encoded_raw)
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
	fn run_vgaudio_cli(&self, src_file: &Path, dest_file: &Path, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		if settings.vgaudio_cli_path.is_empty() {
			return Err("VGAudiCli path is empty".to_owned())
		}

		let mut command: Command;
		if !settings.vgaudio_cli_prepath.is_empty() {
			// Add the prepath if it isn't empty
			command = Command::new(&settings.vgaudio_cli_prepath);
			command.arg(&settings.vgaudio_cli_path);
		} else {
			command = Command::new(&settings.vgaudio_cli_path);
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
				let mut error = format!("Attempted running VGAudioCli, found exit code {}\n", code);

				let stdout = String::from_utf8(output.stdout);
				let stderr = String::from_utf8(output.stderr);

				if let Ok(out) = stdout {
					if out.is_empty() {
						error.push_str("stdout is empty\n")
					} else {
						error.push_str(&format!("stdout is:\n{}\n", out))
					}
				} else {
					error.push_str("stdout couldn't be read\n")
				}
				if let Ok(err) = stderr {
					if err.is_empty() {
						error.push_str("stderr is empty")
					} else {
						error.push_str(&format!("stderr is:\n{}", err))
					}
				} else {
					error.push_str("stderr couldn't be read")
				}

				return Err(error)
			}
		} else {
			return Err("Attempted running VGAudioCli, didn't get any exit code".to_string())
		}

		match fs::read(dest_file) {
			Ok(bytes) => Ok(bytes),
			Err(error) => Err(format!("Error reading destination file {:?}\n{}", dest_file, error))
		}
	}
}
