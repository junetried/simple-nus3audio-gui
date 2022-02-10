mod layout;
mod list;
mod playback;
mod rect;
mod settings;

use fltk::{
	prelude::*,
	app,
	dialog::{
		NativeFileChooser, FileDialogType
	},
	enums::{
		Event, FrameType, Shortcut
	},
	menu::{
		MenuBar, MenuFlag
	},
	window::Window
};
use nus3audio::Nus3audioFile;
use std::fs;
use crate::{
	layout::alert,
	list::{
		List,
		ListItem
	},
	playback::Playback,
	settings::Settings
};

#[derive(Clone, Copy)]
pub enum Message {
	/// The window will re-lay itself out.
	ReLay,
	/// Open a nus3audio.
	Open,
	/// Play.
	PlayPause,
	/// Stop the currently playing sound.
	Stop,
	/// Update the seek bar.
	Update,
	// Seek,
	/// Save the working nus3audio.
	Save,
	/// Save the nus3audio to a new location.
	SaveAs,
	/// Export a single sound.
	ExportSingle,
	/// Export everything.
	ExportAll,
	/// Replace a single sound.
	Replace,
	/// Configure the VGAudioCli path.
	ConfigurePath,
	/// Show the welcome message again.
	WelcomeGreeting,
	/// Quit the application.
	Quit(i32)
}

const NAME: &str = "simple-nus3audio-gui";

fn main() {
	let app = app::App::default();
	let (s, r) = app::channel();
	let mut window = Window::new(0, 0, 250, 200, NAME);
	window.size_range(200, 150, 0, 0);

	// Menu
	let mut menu = MenuBar::default();
	menu.set_frame(FrameType::ThinUpBox);

	menu.add_emit(
		"&File/Open nus3audio\t",
		Shortcut::Ctrl | 'o',
		MenuFlag::Normal,
		s,
		Message::Open,
	);
	menu.add_emit(
		"&File/Save nus3audio\t",
		Shortcut::Ctrl | 's',
		MenuFlag::Normal,
		s,
		Message::Save,
	);
	menu.add_emit(
		"&File/Save nus3audio as...\t",
		Shortcut::Ctrl | Shortcut::Shift | 's',
		MenuFlag::Normal,
		s,
		Message::SaveAs,
	);
	menu.add_emit(
		"&File/Export single sound...\t",
		Shortcut::Ctrl | 'e',
		MenuFlag::Normal,
		s,
		Message::ExportSingle,
	);
	menu.add_emit(
		"&File/Export all...\t",
		Shortcut::Ctrl | Shortcut::Shift | 'e',
		MenuFlag::Normal,
		s,
		Message::ExportAll,
	);
	menu.add_emit(
		"&File/Quit\t",
		Shortcut::Ctrl | 'q',
		MenuFlag::Normal,
		s,
		Message::Quit(0),
	);
	menu.add_emit(
		"&Edit/Play\t",
		Shortcut::from_char(' '),
		MenuFlag::Normal,
		s,
		Message::PlayPause,
	);
	menu.add_emit(
		"&Edit/Stop\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::Stop,
	);
	menu.add_emit(
		"&Edit/Replace single sound...\t",
		Shortcut::Ctrl | 'r',
		MenuFlag::Normal,
		s,
		Message::Replace,
	);
	menu.add_emit(
		"&Edit/Configure VGAudioCli path...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::ConfigurePath,
	);
	menu.add_emit(
		"&Help/VGAudioCli\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::WelcomeGreeting,
	);

	// Playback
	let mut playback = Playback::new(s);

	// This will contain all the list items
	let mut file_list: List = List::new();

	let mut start_input = fltk::input::IntInput::default();
	start_input.set_tooltip("Loop start position in samples");
	let mut end_input = fltk::input::IntInput::default();
	end_input.set_tooltip("Loop end position in samples");

	window.make_resizable(true);
	window.end();
	window.show();

	// Now we need to lay the window out!
	{
		let (play_widget, slider_widget) = playback.get_widgets_mut();
		layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut())
	}

	window.handle(move |_, event| match event {
		Event::Resize => {
			s.send(Message::ReLay);
			true
		}
		_ => { false }
	});

	let mut settings = Settings::new_default();

	// Show the first-time greeting if necessary
	settings.first_time_greeting(&window, s);

	// Create the settings if needed
	if let Err(error) = Settings::create_settings() {
		// We won't exit in this case, but we'll probably have issues later
		fltk::dialog::message_title("Error");
		alert(&window, &format!("Error creating the settings directory:\n{}", error))
	}

	// And reset the cache
	if let Err(error) = Settings::reset_cache() {
		fltk::dialog::message_title("Fatal Error");
		alert(&window, &format!("Error creating the cache directory:\n{}", error));
		std::process::exit(1)
	}
	
	// Main event loop
	while app.wait() {
		// Handle events
		if let Some(e) = r.recv() {
			match e {
				Message::ReLay => {
					let (play_widget, slider_widget) = playback.get_widgets_mut();
					layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut())
				},
				Message::Open => {
					let mut file_dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
					file_dialog.set_filter("*.nus3audio");
					// Get file selection
					file_dialog.show();

					if file_dialog.filename().exists() {
						// Attempt to read chosen file
						let raw = match std::fs::read(file_dialog.filename()) {
							Ok(r) => r,
							Err(e) => {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("Error reading file:\n{}", e));
								continue
							}
						};

						// Try to load the nus3audio file
						let nus3audio = match Nus3audioFile::try_from_bytes(&raw) {
							Some(f) => f,
							None => {
								fltk::dialog::message_title("Error");
								alert(&window, "Error parsing file");
								continue
							}
						};

						file_list.clear();
						file_list.name = file_dialog.filename().file_name().unwrap().to_string_lossy().to_string();
						file_list.path = Some(file_dialog.filename());

						// Add the files to the list
						for file in nus3audio.files {
							let mut item = ListItem::new(file.id);

							item.set_idsp_raw(file.data);

							file_list.add_item(item, &file.name)
						};

						file_list.redraw()
					}
				},
				Message::ExportSingle => {
					if let Some((index, sound_name)) = file_list.selected() {
						let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

						let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
						save_dialog.set_filter("*.wav\n*.idsp");
						save_dialog.show();

						let (extension, raw) = if let Some(extension) = save_dialog.filename().extension() {
							if extension == "idsp" {
								("idsp", list_item.get_idsp_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path))
							} else {
								("wav", list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path))
							}
						} else {
							("wav", list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path))
						};
						if let Err(error) = raw {
							fltk::dialog::message_title("Error");
							alert(&window, &error.to_string());
							continue
						}

						if !save_dialog.filename().to_string_lossy().is_empty() {
							if let Err(error) = fs::write(save_dialog.filename().with_extension(extension), &raw.unwrap()) {
								fltk::dialog::message_title("Error");
								alert(&window, &error.to_string());
							}
						}
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
					
				},
				Message::ExportAll => {
					let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveDir);
					save_dialog.set_filter("*.wav");
					save_dialog.show();

					if !save_dialog.filename().to_string_lossy().is_empty() {
						let mut index: usize = 0;
						while let Some(sound_name) = file_list.get_label_of(index) {
								let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
								let raw = list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path);
								if let Err(error) = raw {
									alert(&window, &error.to_string());
									break
								}
								let target_file = save_dialog.filename().join(&format!("{}.wav", sound_name));
		
								if let Err(error) = fs::write(target_file, &raw.unwrap()) {
									fltk::dialog::message_title("Error");
									alert(&window, &format!("Error writing file:\n{}", error));
									break
								}
								index += 1
						}
					}
				},
				Message::Replace => {
					if let Some((index, _)) = file_list.selected() {
						let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

						let mut open_dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
						open_dialog.set_filter("*.{ogg,flac,wav,mp3,idsp}\n*.ogg\n*.flac\n*.wav\n*.mp3\n*.idsp");
						open_dialog.show();

						if open_dialog.filename().exists() {
							let raw = fs::read(open_dialog.filename());
							if let Err(error) = raw {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("Could not read file:\n{}", error));
								continue
							}

							let result = if let Some(extension) = open_dialog.filename().extension() {
								match extension.to_str() {
									Some("idsp") => { list_item.set_idsp_raw(raw.unwrap()); Ok(()) },
									_ => list_item.set_raw(raw.unwrap())
								}
							} else { list_item.set_raw(raw.unwrap()) };

							if let Err(error) = result {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("Could not decode file:\n{}", error));
							}
						}
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
				},
				Message::Save => {
					if file_list.path.is_some() {
						if let Err(error) = file_list.save_nus3audio(None, &settings.vgaudio_cli_path) {
							fltk::dialog::message_title("Error");
							alert(&window, &format!("Error saving file:\n{}", error));
						}
					} else {
						// Nothing to save to.
						s.send(Message::SaveAs)
					}
				},
				Message::SaveAs => {
					let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
					save_dialog.set_filter("*.nus3audio");
					save_dialog.show();

					if !save_dialog.filename().to_string_lossy().is_empty() {
						if let Err(error) = file_list.save_nus3audio(Some(&save_dialog.filename()), &settings.vgaudio_cli_path) {
							fltk::dialog::message_title("Error");
							alert(&window, &format!("Error saving file:\n{}", error));
						}
					}
				},
				Message::PlayPause => {
					if let Err(error) = playback.on_press(&mut file_list, &settings) {
						fltk::dialog::message_title("Error");
						alert(&window, &error);
					}
				},
				Message::Stop => playback.stop_sink(),
				Message::Update => playback.on_update(),
				// Message::Seek => playback.on_seek(),
				Message::ConfigurePath => settings.configure_vgaudio_cli_path(&window),
				Message::WelcomeGreeting => {
					settings.first_time = true;
					settings.first_time_greeting(&window, s)
				},
				Message::Quit(code) => {
					settings.save();
					Settings::reset_cache().expect("Failed to reset the cache directory");
					std::process::exit(code)
				}
			}
		}
	}

	settings.save();
	Settings::reset_cache().expect("Failed to reset the cache directory")
}
