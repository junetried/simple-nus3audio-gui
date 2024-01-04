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
	browser::Browser,
	dialog::{ FileDialogType, NativeFileChooser }
};
use rodio::Source;
#[allow(unused_imports)]
use log::{ trace, debug, info, warn, error };
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
	Lopus,
	Bin
}

impl std::fmt::Display for AudioExtension {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			AudioExtension::Idsp => write!(f, "idsp"),
			AudioExtension::Lopus => write!(f, "lopus"),
			AudioExtension::Bin => write!(f, "bin")
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
	/// Whether or not this list has been modified. This is used to track unsaved changes.
	pub modified: bool,
	/// The browser widget representing the file.
	widget: Browser,
	/// The last browse directory of the replace dialog
	browser_path: Option<PathBuf>
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
			modified: false,
			widget,
			browser_path: None
		}
	}

	/// Remove an item from this list by index.
	/// 
	/// Marks this list as being modified.
	pub fn remove(&mut self, index: usize) {
		self.items.remove(index);
		self.widget.remove(index as i32 + 1);
		self.modified = true;
	}

	/// Clear the items in this list.
	/// 
	/// Marks this list as being unmodified.
	pub fn clear(&mut self) {
		self.items.clear();
		self.widget.clear();
		self.modified = false
	}

	/// Replace a sound at `index` via a file dialog.
	/// 
	/// If it doesn't fail, marks this list as being modified.
	pub fn replace(&mut self, index: usize, settings: &crate::Settings) -> Result<(), String> {
		let list_item = match self.items.get_mut(index) {
			Some(item) => item,
			None => return Err("Failed to find internal list item.\nYou shouldn't be seeing this during normal use.".to_owned())
		};

		let mut open_dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
		open_dialog.set_filter("*.{ogg,flac,wav,mp3,idsp,lopus}\n*.ogg\n*.flac\n*.wav\n*.mp3\n*.idsp\n*.lopus\n*");
		// Set the default path to the last path used
		if let Some(path) = &self.browser_path {
			let _ = open_dialog.set_directory(path);
		}
		open_dialog.show();

		if open_dialog.filename().exists() {
			// Set the last path used to the path we just used
			self.browser_path = open_dialog.filename().parent().map(|path| path.to_owned());

			let raw = fs::read(open_dialog.filename());
			if let Err(error) = raw {
				return Err(format!("Could not read file:\n{}", error))
			}
			let raw = raw.unwrap();

			let result = if let Some(extension) = open_dialog.filename().extension() {
				match extension.to_str() {
					Some("idsp") => { list_item.from_encoded(&self.name, raw, settings) },
					Some("lopus") => { list_item.from_encoded(&self.name, raw, settings) },
					_ => list_item.set_audio_raw(raw)
				}
			} else { list_item.set_audio_raw(raw) };

			if let Err(error) = result {
				return Err(format!("Could not decode file as audio:\n{}", error))
			}

			list_item.loop_points_samples = ListItem::loop_points_of(&open_dialog.filename(), settings);
			self.modified = true;

			Ok(())
		} else {
			Ok(())
		}
	}

	/// Save this nus3audio to the file at `self.path`.
	/// 
	/// Marks this list as being unmodified.
	pub fn save_nus3audio(&mut self, path: Option<PathBuf>, settings: &crate::settings::Settings) -> Result<(), String> {
		let path = if let Some(path) = path { path } else { self.path.clone().expect("No path has been set to save.") };
		let name = path.file_name().unwrap().to_string_lossy().to_string();
		let mut nus3audio = Nus3audioFile::new();

		for (index, list_item) in self.items.iter_mut().enumerate() {
			let data = list_item.get_nus3_encoded_raw(&name, &list_item.extension.to_string(), settings).unwrap_or_else(|_| Vec::new());
			nus3audio.files.push(
				nus3audio::AudioFile {
					id: index as u32,
					name: list_item.name.to_owned(),
					data
				}
			)
		}

		let mut export: Vec<u8> = Vec::new();
		nus3audio.write(&mut export);

		// Update label, after potentially encoding some items
		// that were empty previously
		for index in 0..self.items.len() {
			self.update_label_of(index)
		}

		if let Err(error) = fs::write(path.with_extension("nus3audio"), &export) {
			Err(error.to_string())
		} else {
			self.modified = false;
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
		let mut text = text.to_owned();
		// Append a status if the item isn't complete
		match (self.items[line].audio_raw.is_some(), self.items[line].bytes_raw.is_some()) {
			(true, true) => {},
			(true, false) => text.push_str(" (Not yet encoded)"),
			(false, true) => text.push_str(" (Could not decode)"),
			(false, false) => text.push_str(" (Empty)")
		}
		self.widget.set_text(line as i32 + 1, &text)
	}

	pub fn update_label_of(&mut self, line: usize) {
		self.set_label_of(line, &format!("{}.{}", self.items[line].name, self.items[line].extension))
	}

	/// Returns the [&mut Browser] widget of this List.
	pub fn get_widget_mut(&mut self) -> &mut Browser {
		&mut self.widget
	}

	/// Adds an item to the list.
	/// 
	/// Marks this list as being modified.
	pub fn add_item(&mut self, item: ListItem, name: &str) {
		self.items.push(item);
		self.widget.add(name);
		self.modified = true
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
	/// Raw, unconverted bytes.
	/// There is no guarantee that this data is in any particular format.
	bytes_raw: Option<Vec<u8>>,
	/// Loop points of this sound in samples.
	pub loop_points_samples: Option<(usize, usize)>,
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
			bytes_raw: None,
			loop_points_samples: None,
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

	/// Return the loop points in samples.
	pub fn loop_points_samples(&self) -> Option<(i64, i64)> {
		if let Some((begin, end)) = &self.loop_points_samples {
			Some((
				*begin as i64,
				*end as i64
			))
		} else {
			None
		}
	}

	/// Attach new bytes to this item and attempt to decode them.
	pub fn set_audio_raw(&mut self, raw: Vec<u8>) -> Result<(), String> {
		// If we can't decode these bytes, we'll just clear the audio data and set the bytes later
		let raw_copy = raw.clone();

		let cursor = Cursor::new(raw);

		// It would be nice to use Kira for this,
		// but it seems to coerce everything into dual channel,
		// which isn't ideal.
		// 
		// I've considered using the Symphonia crate, but it looks far
		// too complicated to use for something otherwise so simple.
		// For example, this is the basic "decoding audio" mock-up:
		// https://github.com/pdeljanov/Symphonia/blob/master/symphonia/examples/getting-started.rs#L53
		let decoder = rodio::Decoder::new(cursor);
		if let Err(error) = decoder {
			// This can't be decoded
			warn!("Error decoding file: {}
  This is not fatal, this file's bytes have been loaded directly. If this is not desired, make sure this file is a known format and is not corrupted.", error);
			self.loop_points_samples = None;
			self.extension = AudioExtension::Bin;
			self.audio_raw = None;
			self.bytes_raw = Some(raw_copy);
			return Ok(())
			
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

		debug!("Successfully decoded audio");
		if decoder_sample_rate != sample_rate {
			warn!("Sample rate has been changed! Input file was {} Hz and decoded file is {} Hz", decoder_sample_rate, sample_rate);
			warn!("If you plan to set loop points, export this item as a wav and check your loop points, otherwise they WILL be wrong!")
		}
		self.audio_raw = Some(decoded);
		self.loop_points_samples = None;
		self.bytes_raw = None;
		Ok(())
	}

	/// Gets the sound from an encoded IDSP or LOPUS file.
	/// 
	/// More specifically, it will attempt to decode bytes with VGAudio CLI or vgmstream.
	pub fn from_encoded(&mut self, nus3audio_name: &str, encoded: Vec<u8>, settings: &crate::settings::Settings) -> Result<(), String> {
		let target_dir = CACHEDIR.join(nus3audio_name);
		
		let src_file = target_dir.join(&self.name).with_extension(extension_of_encoded(&encoded)?.to_string());

		if let Err(error) = Self::create_target_dir(&target_dir) {
			return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
		};

		if let Err(error) = fs::write(&src_file, &encoded) {
			return Err(format!("Error writing source file {:?}\n{}", src_file, error))
		};

		match self.decode(&src_file, settings) {
			Ok(raw) => {
				// This should be in wav format now
				let loop_points = Self::loop_points_of(&src_file, settings);

				let wav_result = wav::read(&mut Cursor::new(&raw));

				// Check that the wav could be read
				match wav_result {
					Ok((header, bitdepth)) => {
						let decoded = match bitdepth.try_into_sixteen() {
							Ok(decoded) => decoded,
							Err(bitdepth) => return Err(format!("Error reading returned wav\nWrong bit depth found: {:?}", bitdepth))
						};
						self.bytes_raw = Some(raw);
						self.audio_raw = Some(decoded);
						self.channels = header.channel_count;
						self.sample_rate = header.sampling_rate;
						self.loop_points_samples = loop_points;

						Ok(())
					},
					Err(error) => Err(format!("Error reading returned wav\n{}", error))
				}
			},
			Err(error) => {
				// Could not be decoded, assume this is binary data
			warn!("Error decoding file: {}
  This is not fatal, this file's bytes have been loaded directly. If this is not desired, make sure this file is a known format and is not corrupted.", error);
				self.bytes_raw = Some(encoded);
				self.audio_raw = None;
				self.extension = AudioExtension::Bin;
				self.loop_points_samples = None;
				Ok(())
			}
		}
	}

	/// Removes the bytes from this item.
	pub fn clear_bytes(&mut self) {
		self.bytes_raw = None
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
				if raw.len() < sample_length { raw.len() } else { sample_length }
			} else {
				raw.len()
			}];
			// Finally, write the wav file
			// I don't honestly know when this can fail...
			if let Err(error) = wav::write(header, &wav::BitDepth::Sixteen(raw.to_vec()), &mut wav_cursor) { return Err(error.to_string())};

			Ok(wav_file)
		} else {
			if self.bytes_raw.is_none() {
				Err("Selected item is empty".to_owned())
			} else {
				Err("Selected item could not be decoded".to_owned())
			}
		}
	}

	/// Return the bytes associated with this item. If it has audio but no bytes, the audio is converted according to `extension`.
	pub fn get_nus3_encoded_raw(&mut self, nus3audio_name: &str, extension: &str, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		if self.audio_raw.is_none() { return Err("Audio of selected item is empty".to_owned()) }

		if let Some(bytes) = &self.bytes_raw {
			return Ok(bytes.clone())
		} else {
			// Need to convert the wav
			let target_dir = CACHEDIR.join(nus3audio_name);
			let dest_file = target_dir.join(&self.name).with_extension(extension);
			let src_file = dest_file.with_extension("wav");

			if let Err(error) = Self::create_target_dir(&target_dir) {
				return Err(format!("Error creating cache subdirectory {:?}\n{}", target_dir, error))
			};

			match self.get_audio_wav(self.loop_end()) {
				Ok(bytes) => {
					if let Err(error) = fs::write(&src_file, bytes) {
						return Err(format!("Error writing source file {:?}\n{}", src_file, error))
					}
				},
				Err(error) => return Err(format!("Error decoding audio\n{}", error))
			}

			self.bytes_raw = Some(self.vgaudio_cli_decode(&src_file, &dest_file, settings)?);

			debug!("Encoded {:?} to {:?}", src_file, dest_file);

			Ok(self.bytes_raw.as_ref().unwrap().clone())
		}
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

	/// Decode `src_file` to a WAV file as bytes.
	/// 
	/// Might use vgmstream or VGAudio Cli depending on which one is available to use.
	fn decode(&self, src_file: &Path, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		if settings.prefer_vgmstream_decode() {
			if !settings.vgmstream_path().is_empty() {
				Self::vgmstream_decode(src_file, settings)
			} else {
				self.vgaudio_cli_decode(src_file, &src_file.with_extension("wav"), settings)
			}
		} else {
			if !settings.vgaudio_cli_path().is_empty() {
				self.vgaudio_cli_decode(src_file, &src_file.with_extension("wav"), settings)
			} else {
				Self::vgmstream_decode(src_file, settings)
			}
		}
	}

	/// Return loop points associated with `src_file`.
	/// 
	/// Requires vgmstream to be present and working, and will silently fail otherwise.
	pub fn loop_points_of(src_file: &Path, settings: &crate::settings::Settings) -> Option<(usize, usize)> {
		// Check if we can get metadata from this file
		if let Ok(metadata) = Self::vgmstream_metadata(src_file, settings) {
			// Check if the metadata has the "loopingInfo" object
			if let json::JsonValue::Object(loop_info) = &metadata["loopingInfo"] {
				// Check that the "start" and "end" numbers can be read as usize
				if let (Some(start), Some(end)) = (loop_info["start"].as_usize(), loop_info["end"].as_usize()) {
					// Check that the end is placed after the start
					if end > start {
						return Some((start, end))
					}
				}
			}
		}

		None
	}

	/// Run VGAudioCli, convert `src_file` to `dest_file` and return it as bytes.
	fn vgaudio_cli_decode(&self, src_file: &Path, dest_file: &Path, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		let vgaudio_cli_path = settings.vgaudio_cli_path();
		if vgaudio_cli_path.is_empty() {
			return Err("VGAudiCli path is empty".to_owned())
		}

		let mut command: Command;
		match settings.vgaudio_cli_prepath() {
			vgaudio_cli_prepath if !vgaudio_cli_prepath.is_empty() => {
				// Add the prepath if it isn't empty
				command = Command::new(vgaudio_cli_prepath);
				command.arg(vgaudio_cli_path);
			},
			_ => {
				command = Command::new(vgaudio_cli_path);
			}
		}

		command.arg("-c")
			.arg(src_file.as_os_str())
			.arg(dest_file.as_os_str());
		
		// Add loop points if they exist
		if let Some((from, to)) = self.loop_points_samples {
			command.arg("-l").arg(format!("{}-{}", from, to)).arg("--cbr").arg("--opusheader").arg("namco");
		}

		debug!("Running {:?}", command);

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

			let stdout = String::from_utf8(output.stdout);
			let stderr = String::from_utf8(output.stderr);

			if let Ok(out) = stdout {
				if out.is_empty() {
					debug!("stdout is empty")
				} else {
					debug!("stdout is:\n{}", out)
				}
			} else {
				debug!("stdout couldn't be read")
			}
			if let Ok(err) = stderr {
				if err.is_empty() {
					debug!("stderr is empty")
				} else {
					debug!("stderr is:\n{}", err)
				}
			} else {
				debug!("stderr couldn't be read")
			}
		} else {
			return Err("Attempted running VGAudioCli, didn't get any exit code".to_string())
		}

		match fs::read(dest_file) {
			Ok(bytes) => Ok(bytes),
			Err(error) => Err(format!("Error reading destination file {:?}\n{}", dest_file, error))
		}
	}

	/// Run vgmstream, decode `src_file` and return it as bytes.
	fn vgmstream_decode(src_file: &Path, settings: &crate::settings::Settings) -> Result<Vec<u8>, String> {
		let vgmstream_path = settings.vgmstream_path();
		if vgmstream_path.is_empty() {
			return Err("vgmstream path is empty".to_owned())
		}

		// Create the command
		let mut command = Command::new(vgmstream_path);
		command.arg("-p")
		// -m: print metadata only, don't decode
		// -I: print requested file info as JSON
			.arg(src_file);

		debug!("Running {:?}", command);

		// Run the command
		let output = command.output();

		let output = if let Err(error) = output {
			return Err(format!("Error running vgmstream\n{}", error))
		} else {
			output.unwrap()
		};

		// Check the error code
		if let Some(code) = output.status.code() {
			if code != 0 {
				let mut error = format!("Attempted running vgmstream, found exit code {}\n", code);

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
		}

		Ok(output.stdout)
	}

	/// Run vgmstream, read metadata of `src_file` and return a [json::JsonValue].
	fn vgmstream_metadata(src_file: &Path, settings: &crate::settings::Settings) -> Result<json::JsonValue, String> {
		let vgmstream_path = settings.vgmstream_path();
		if vgmstream_path.is_empty() {
			return Err("vgmstream path is empty".to_owned())
		}

		// Create the command
		let mut command = Command::new(vgmstream_path);
		command.arg("-mI")
		// -m: print metadata only, don't decode
		// -I: print requested file info as JSON
			.arg(src_file);

		debug!("Running {:?}", command);

		// Run the command
		let output = command.output();

		let output = if let Err(error) = output {
			return Err(format!("Error running vgmstream\n{}", error))
		} else {
			output.unwrap()
		};

		// Check the error code
		if let Some(code) = output.status.code() {
			if code != 0 {
				let mut error = format!("Attempted running vgmstream, found exit code {}\n", code);

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
		}

		// Get string output
		let text_output = match std::str::from_utf8(&output.stdout) {
			Ok(output) => output,
			Err(error) => return Err(format!("Error reading vgmstream output\n{}", error))
		};
		// Parse output as JSON
		match json::parse(text_output) {
			Ok(output) => Ok(output),
			Err(error) => Err(format!("Error parsing vgmstream output\n{}", error))
		}
	}
}
