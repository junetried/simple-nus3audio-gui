#[cfg(not(target_os = "windows"))]
use which::which;
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
pub const VGAUDIO_CLI_PREPATH_DEFAULT: &'static str = "";

#[cfg(not(target_os = "windows"))]
lazy_static! {
	pub static ref VGAUDIO_CLI_PREPATH_DEFAULT: &'static str = {
		if which("mono").is_ok() {
			"mono"
		} else if which("dotnet").is_ok() {
			"dotnet"
		} else {
			"wine"
		}
	};
}

const VGAUDIO_CLI_PATH: &str = "vgaudio_cli_path";
const VGAUDIO_CLI_PREPATH: &str = "vgaudio_cli_prepath";
const FIRST_TIME: &str = "first_time";
#[cfg(target_os = "windows")]
const VGAUDIO_CLI_PATH_DEFAULT: &str = ".\\VGAudioCli.exe";
#[cfg(not(target_os = "windows"))]
const VGAUDIO_CLI_PATH_DEFAULT: &str = "./VGAudioCli.exe";

const FIRST_TIME_DEFAULT: bool = false;

const CONFIGURE_MESSAGE: &str = "Please set the path to the VGAudioCli executable.";
#[cfg(not(target_os = "windows"))]
const CONFIGURE_RUNTIME_MESSAGE: &str = "Please set the path to the executable used to run .NET applications.
This executable will be given the path to the VGAudioCli executable, immediately followed by arguments passed to it.
It is recommended to use mono or dotnet over wine.";

pub struct Settings (pub toml::map::Map<String, toml::Value>);

impl Default for Settings {
	fn default() -> Self {
		Self::new()
	}
}

impl Settings {
	pub fn new() -> Self {
		let map = toml::map::Map::new();

		Self::from_default(map)
			}

	/// Create new settings from the map provided, filling in missing values with defaults.
	pub fn from_default(mut map: toml::map::Map<String, toml::Value>) -> Self {
		if !map.contains_key(VGAUDIO_CLI_PATH) {
			map.insert(VGAUDIO_CLI_PATH.to_owned(), toml::Value::String(VGAUDIO_CLI_PATH_DEFAULT.to_owned()));
		}
		if !map.contains_key(VGAUDIO_CLI_PREPATH) {
			map.insert(VGAUDIO_CLI_PREPATH.to_owned(), toml::Value::String(VGAUDIO_CLI_PREPATH_DEFAULT.to_owned()));
		}
		if !map.contains_key(FIRST_TIME) {
			map.insert(FIRST_TIME.to_owned(), toml::Value::Boolean(FIRST_TIME_DEFAULT));
		}
	}

	/// Return a deserialized settings file, or the default.
	pub fn new_default() -> Self {
		match std::fs::read(CONFIG.as_path()) {
			Ok(bytes) => match toml::from_slice::<toml::map::Map<String, toml::Value>>(&bytes) {
				Ok(map) => return Self::from_default(map),
				Err(error) => println!("couldn't read settings, skipping: {}", error)
			},
			Err(error) => println!("couldn't read settings, skipping: {}", error)
		}

		Self::new()
	}

	/// Return the path to VGAudioCli's executable.
	pub fn vgaudio_cli_path(&self) -> &str {
		let value = self.0.get::<str>(VGAUDIO_CLI_PATH);
		if let Some(toml::Value::String(value)) = value {
			value
		} else {
			VGAUDIO_CLI_PATH_DEFAULT
		}
	}

	#[cfg(target_os = "windows")]
	/// Return the .NET runtime used to run VGAudioCli.
	/// 
	/// Though the .NET runtime is not configurable in Windows,
	/// this setting is still used there (although it defaults to an empty string).
	pub fn vgaudio_cli_prepath(&self) -> &str {
		let value = self.toml.get::<str>(VGAUDIO_CLI_PREPATH);
		if let Some(toml::Value::String(value)) = value {
			value
		} else {
			VGAUDIO_CLI_PREPATH_DEFAULT
		}
	}

	#[cfg(not(target_os = "windows"))]
	/// Return the .NET runtime used to run VGAudioCli.
	/// 
	/// Though the .NET runtime is not configurable in Windows,
	/// this setting is still used there (although it defaults to an empty string).
	pub fn vgaudio_cli_prepath(&self) -> &str {
		let value = self.0.get::<str>(VGAUDIO_CLI_PREPATH);
		if let Some(toml::Value::String(value)) = value {
			value
		} else {
			&VGAUDIO_CLI_PREPATH_DEFAULT
		}
	}

	/// Return the first time boolean. Whether or not the first-time message should be displayed.
	pub fn first_time(&self) -> bool {
		let value = self.0.get::<str>(FIRST_TIME);
		if let Some(toml::Value::Boolean(value)) = value {
			*value
		} else {
			FIRST_TIME_DEFAULT
		}
	}

	/// Set the first time boolean. Whether or not the first-time message should be displayed.
	pub fn set_first_time(&mut self, first_time: bool) {
		self.0.insert(FIRST_TIME.to_owned(), toml::Value::Boolean(first_time));
	}

	/// Save these settings. Never returns an error, but prints errors to stderr.
	pub fn save(&self) {
		if let Err(error) = Self::create_settings() {
			eprintln!("Error creating settings: {}", error)
		} else {
			match toml::to_string(&self.0) {
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
		if self.first_time() {
			message_title("Welcome");
			let response = choice2(window, "To get started, please download a release of
https://ci.appveyor.com/project/Thealexbarney/VGAudio/build/artifacts
Then, visit \"File → Configure VGAudioCli\" to set this location.", "Dismiss", "Show me", "");

			if let Some(1) = response {
				let _ = open::that("https://ci.appveyor.com/project/Thealexbarney/VGAudio/build/artifacts");
				sender.send(crate::Message::ConfigurePath)
			}

			self.set_first_time(false)
		}
	}

	/// Open an input dialog that allows changing the VGAudioCli path.
	pub fn configure_vgaudio_cli_path(&mut self, window: &Window) {
		message_title("VGAudioCli Path");
		if let Some(new_path) = input(window, CONFIGURE_MESSAGE, self.vgaudio_cli_path()) {
			self.0.insert(VGAUDIO_CLI_PATH.to_owned(), toml::Value::String(new_path));
		}
	}

	#[cfg(not(target_os = "windows"))]
	/// Open an input dialog that allows changing the .NET runtime path.
	/// 
	/// Though the .NET runtime is not configurable in Windows,
	/// this setting is still used there (although it defaults to an empty string).
	pub fn configure_vgaudio_cli_prepath(&mut self, window: &Window) {
		message_title(".NET Runtime Path");
		if let Some(new_path) = input(window, CONFIGURE_RUNTIME_MESSAGE, self.vgaudio_cli_prepath()) {
			self.0.insert(VGAUDIO_CLI_PREPATH.to_owned(), toml::Value::String(new_path));
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
