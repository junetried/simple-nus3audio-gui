#[cfg(not(target_os = "windows"))]
use which::which;
use serde::{ Serialize, Deserialize };
use lazy_static::lazy_static;
use directories::BaseDirs;
use std::{
	fs,
	path::PathBuf
};
use crate::layout::{
	choice2,
	input
};
use fltk::window::Window;
use fltk::dialog::message_title;

lazy_static! {
	pub static ref CONFIGDIR: PathBuf = {
		let base = BaseDirs::new().expect("Failed to get home directory");
		base.config_dir().join(crate::NAME)
	};
	pub static ref CACHEDIR: PathBuf = {
		let base = BaseDirs::new().expect("Failed to get home directory");
		base.cache_dir().join(crate::NAME)
	};
	pub static ref CONFIG: PathBuf = CONFIGDIR.join("settings.toml");
}

#[cfg(target_os = "windows")]
const CONFIGURE_MESSAGE: &str = "Please set the path to the VGAudioCli executable.";
#[cfg(not(target_os = "windows"))]
const CONFIGURE_MESSAGE: &str = "Please set the path to the VGAudioCli executable.\nIt is recommended to use mono or dotnet over wine.";

#[derive(Serialize, Deserialize)]
pub struct Settings {
	pub vgaudio_cli_path: String,
	pub first_time: bool
}

impl Default for Settings {
	fn default() -> Self {
		Self::new()
	}
}

impl Settings {
	pub fn new() -> Self {
		#[cfg(target_os = "windows")]
		let vgaudio_cli_path = r".\VGAudioCli.exe".to_owned();
		#[cfg(not(target_os = "windows"))]
		let vgaudio_cli_path = {
			if let Ok(_) = which("mono") {
				"mono ./VGAudioCli.exe".to_owned()
			} else if let Ok(_) = which("dotnet") {
				"dotnet ./VGAudioCli.exe".to_owned()
			} else {
				"wine ./VGAudioCli.exe".to_owned()
			}
		};

		Self {
			vgaudio_cli_path,
			first_time: true
		}
	}

	/// Return a deserialized settings file, or the default.
	pub fn new_default() -> Self {
		match std::fs::read(CONFIG.as_path()) {
			Ok(bytes) => match toml::from_slice::<Self>(&bytes) {
				Ok(settings) => return settings,
				Err(error) => println!("couldn't read settings, skipping: {}", error)
			},
			Err(error) => println!("couldn't read settings, skipping: {}", error)
		}

		Self::new()
	}

	/// Save these settings. Never returns an error, but prints errors to stderr.
	pub fn save(&self) {
		if let Err(error) = Self::create_settings() {
			eprintln!("Error creating settings: {}", error)
		} else {
			match toml::to_string(self) {
				Ok(string) => {
					let _ = fs::write(CONFIG.as_path(), string);
				},
				Err(error) => {
					eprintln!("Error serializing settings: {}", error)
				}
			}
		}
	}

	/// Shows the first-time greeting if it hasn't already been shown.
	pub fn first_time_greeting(&mut self, window: &Window, sender: fltk::app::Sender<crate::Message>) {
		if self.first_time {
			message_title("Welcome");
			let response = choice2(window, "To get started, please download a release of
https://github.com/Thealexbarney/VGAudio/releases
and extract it.
Then, visit \"File → Configure VGAudioCli\" to set this location.", "Dismiss", "Show me", "");

			if let Some(1) = response {
				let _ = open::that("https://github.com/Thealexbarney/VGAudio/releases");
				sender.send(crate::Message::ConfigurePath)
			}

			self.first_time = false
		}
	}

	/// Open an input dialog that allows changing the VGAudioCli path.
	pub fn configure_vgaudio_cli_path(&mut self, window: &Window) {
		message_title("VGAudioCli Path");
		if let Some(string) = input(window, CONFIGURE_MESSAGE, &self.vgaudio_cli_path) {
			self.vgaudio_cli_path = string
		}
	}

	/// Function that tries to create the config directory.
	pub fn create_settings() -> Result<(), std::io::Error> {
		if !CONFIGDIR.exists() {
			fs::create_dir(CONFIGDIR.as_path())?
		} else if CONFIGDIR.is_file() {
			fs::remove_file(CONFIGDIR.as_path())?;
			fs::create_dir(CONFIGDIR.as_path())?
		}

		Ok(())
	}

	/// Function that will reset the cache dir to an empty state.
	pub fn reset_cache() -> Result<(), std::io::Error> {
		if CACHEDIR.exists() {
			if CACHEDIR.is_dir() {
				let contents = CACHEDIR.read_dir()?;
				for item in contents {
					let item_path = item?.path();
					if item_path.is_dir() {
						fs::remove_dir_all(item_path)?
					} else {
						fs::remove_file(item_path)?
					}
				}
				fs::remove_dir_all(CACHEDIR.as_path())?;
			} else {
				fs::remove_file(CACHEDIR.as_path())?;
			}
		}

		fs::create_dir(CACHEDIR.as_path())?;
		Ok(())
	}
}
