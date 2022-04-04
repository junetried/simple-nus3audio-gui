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
pub fn extension_of_encoded(encoded: &[u8]) -> Result<AudioExtension, String> {
	Ok(
		if encoded.len() < 4 {
			return Err("Not a valid file".to_owned())
		} else if encoded[..4].eq(b"IDSP") {
			AudioExtension::Idsp
		} else {
			AudioExtension::Lopus
		}
	)
}

/// Possible (valid) formats for audio in a nus3audio file.
#[derive(Clone, PartialEq, Eq)]
pub enum AudioExtension {
	Idsp,
	Lopus
}

impl std::fmt::Display for AudioExtension {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			AudioExtension::Idsp => write!(f, "idsp"),
			AudioExtension::Lopus => write!(f, "lopus"),
		}
	}
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

	/// Remove an item from this list by index.
	pub fn remove(&mut self, index: usize) {
		self.items.remove(index);
		self.widget.remove(index as i32 + 1)
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
			match list_item.get_nus3_encoded_raw(&name, settings) {
				Ok(data) => {
					nus3audio.files.push(
						nus3audio::AudioFile {
							id: index as u32,
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

	pub fn set_label_of(&mut self, line: usize, text: &str) {
		self.widget.set_text(line as i32 + 1, text)
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
	/// The name of this audio.
	pub name: String,
	/// The extension of the audio in this nus3audio file.
	pub extension: AudioExtension,
	/// Raw audio, in wav format.
	pub audio_raw: Option<Vec<i16>>,
	/// Loop points of this sound in samples.
	loop_points_samples: Option<(usize, usize)>,
	/// Loop points of this sound in seconds.
	loop_points_seconds: Option<(f64, f64)>,
	/// Length in samples of the sound.
	pub length_in_samples: usize,
	/// Sample rate of the sound.
	sample_rate: u32,
	/// Number of channels
	channels: u16
}

impl ListItem {
	/// Return a new [ListItem].
	pub fn new(name: String) -> Self {
		Self {
			name,
			extension: AudioExtension::Idsp,
			audio_raw: None,
			loop_points_samples: None,
			loop_points_seconds: None,
			length_in_samples: 0,
			sample_rate: 12_000,
			channels: 1
		}
	}

	/// Return the loop points in samples.
	pub fn loop_points(&self) -> &Option<(usize, usize)> {
		&self.loop_points_samples
	}

	/// Return the ending loop point
	pub fn loop_end(&self) -> Option<usize> {
		self.loop_points_samples.map(|(_, end)| end)
	}

	/// Return the loop points in seconds.
	pub fn loop_points_seconds(&self) -> &Option<(f64, f64)> {
		&self.loop_points_seconds
	}

	/// Set the loop points in samples.
	pub fn set_loop_points(&mut self, loop_points: Option<(usize, usize)>) {
		if let Some((begin, end)) = loop_points {
			self.loop_points_seconds = Some((
				begin as f64 / self.sample_rate as f64,
				end as f64 / self.sample_rate as f64
			));
		} else {
			self.loop_points_seconds = None;
		}
		self.loop_points_samples = loop_points;
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

		let channel_count = if decoder.channels() == 1 { 1 } else { 2 };

		let mut decoded: Vec<i16> = decoder.collect();

		if decoder_sample_rate != sample_rate {
			// Need to resample
			if channel_count == 1 {
				let input = fon::Audio::<fon::chan::Ch16, 1>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 1>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			} else {
				let input = fon::Audio::<fon::chan::Ch16, 2>::with_i16_buffer(decoder_sample_rate, decoded);

				let mut output = fon::Audio::<fon::chan::Ch16, 2>::with_audio(sample_rate, &input);

				decoded = output.as_i16_slice().to_vec()
			}
		}

		self.length_in_samples = decoded.len() / channel_count as usize;
		self.sample_rate = sample_rate;
		self.channels = channel_count;

		self.audio_raw = Some(decoded);
		self.set_loop_points(None);
		Ok(())
	}

	/// Gets the sound from an encoded file from a nus3audio file.
	pub fn from_encoded(&mut self, nus3audio_name: &str, encoded: Vec<u8>, settings: &crate::settings::Settings) -> Result<(), String> {
		let target_dir = CACHEDIR.join(nus3audio_name);

		
		let src_file = target_dir.join(&self.name).with_extension(extension_of_encoded(&encoded)?.to_string());

		let dest_file = src_file.with_extension("wav");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, encoded) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		// This should be in wav format now
		let raw = self.run_vgaudio_cli(&src_file, &dest_file, settings)?;

		let wav_result = wav::read(&mut Cursor::new(&raw));

		// Check that the wav could be read
		match wav_result {
			Ok((header, bitdepth)) => {
				let raw = match bitdepth.try_into_sixteen() {
					Ok(raw) => raw,
					Err(bitdepth) => return Err(format!("Error reading returned wav\nWrong bit depth found: {:?}", bitdepth))
				};
				self.audio_raw = Some(raw);
				self.set_loop_points(None);
				self.channels = header.channel_count;
				self.sample_rate = header.sampling_rate;

				Ok(())
			},
			Err(error) => Err(format!("Error reading returned wav\n{}", error))
		}
	}

	/// Return the audio from this item in WAV format. Yes, this is hacky.
	/// 
	/// Optionally take the length in samples that should be used.
	pub fn get_audio_wav(&self, end: Option<usize>) -> Result<Vec<u8>, String> {
		if let Some(raw) = &self.audio_raw {
			// Make the header
			let header = wav::Header::new(wav::WAV_FORMAT_PCM, self.channels, self.sample_rate, 16);
			// Create the empty vec
			let mut wav_file: Vec<u8> = Vec::new();
			// And the cursor to write to it
			let mut wav_cursor = Cursor::new(&mut wav_file);
			// Get the raw slice if there is a specific sample limit
			let raw = &raw[0..if let Some(end) = end {
				let sample_length = end * self.channels as usize;
				println!("raw.len() = {}, sample_length = {}", raw.len(), sample_length);
				if raw.len() < sample_length { raw.len() } else { sample_length }
			} else {
				raw.len()
			}];
			// Finally, write the wav file
			// I don't honestly know when this can fail...
			if let Err(error) = wav::write(header, &wav::BitDepth::Sixteen(raw.to_vec()), &mut wav_cursor) { return Err(error.to_string())};

			Ok(wav_file)
		} else {
			Err("Audio of selected item is empty".to_owned())
		}
	}

	/// Return the nus3audio-encoded sound from this item. Converts the raw audio.
	pub fn get_nus3_encoded_raw(&mut self, nus3audio_name: &str, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		if self.audio_raw.is_none() { return Err("Audio of selected item is empty".to_owned()) }

		// Need to convert the wav
		let target_dir = CACHEDIR.join(nus3audio_name);
		let dest_file = target_dir.join(&self.name).with_extension(self.extension.to_string());
		let src_file = dest_file.with_extension("wav");

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, self.get_audio_wav(self.loop_end()).unwrap()) {
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

		command.arg("-c")
			.arg(src_file.as_os_str())
			.arg(dest_file.as_os_str());
		
		// Add loop points if they exist
		if let Some((from, to)) = self.loop_points_samples {
			command.arg("-l").arg(format!("{}-{}", from, to)).arg("--cbr").arg("--opusheader").arg("namco");
		}

		let output = command.output();

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
