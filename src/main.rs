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
	/// Create a new nus3audio.
	New,
	/// Open a nus3audio.
	Open,
	/// Play.
	PlayPause,
	/// Update the seek bar.
	Update,
	// Seek,
	/// Save the nus3audio.
	Save,
	/// Export a single sound.
	ExportSingle,
	/// Export everything.
	ExportAll,
	/// Replace a single sound.
	Replace,
	/// Set the loop status of selected audio.
	LoopEdit,
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
	window.size_range(200, 150, i32::MAX, i32::MAX);

	// Menu
	let mut menu = MenuBar::default();
	menu.set_frame(FrameType::ThinUpBox);

	menu.add_emit(
		"&File/New\t",
		Shortcut::Ctrl | 'n',
		MenuFlag::Normal,
		s,
		Message::New,
	);
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
		"&File/Export single sound\t",
		Shortcut::Ctrl | 'e',
		MenuFlag::Normal,
		s,
		Message::ExportSingle,
	);
	menu.add_emit(
		"&File/Export all\t",
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
		"&Edit/Replace single sound\t",
		Shortcut::Ctrl | 'r',
		MenuFlag::Normal,
		s,
		Message::Replace,
	);
	menu.add_emit(
		"&Edit/Configure selected audio loop...\t",
		Shortcut::empty(),
		MenuFlag::Normal,
		s,
		Message::LoopEdit,
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
		layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut(), &mut start_input, &mut end_input)
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
					layout::lay_widgets(&mut window, &mut menu, play_widget, slider_widget, file_list.get_widget_mut(), &mut start_input, &mut end_input)
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
						let raw = list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path);
						if let Err(error) = raw {
							fltk::dialog::message_title("Error");
							alert(&window, &format!("{}", error));
							continue
						}

						let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
						save_dialog.set_filter("*.wav");
						save_dialog.show();

						if !save_dialog.filename().to_string_lossy().is_empty() {
							if let Err(error) = fs::write(save_dialog.filename().with_extension("wav"), &raw.unwrap()) {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("{}", error));
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
						loop {
							if let Some(sound_name) = file_list.get_label_of(index) {
								let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
								let raw = list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path);
								if let Err(error) = raw {
									alert(&window, &format!("{}", error));
									break
								}
								let target_file = save_dialog.filename().join(&format!("{}.wav", sound_name));
		
								if let Err(error) = fs::write(target_file, &raw.unwrap()) {
									fltk::dialog::message_title("Error");
									alert(&window, &format!("Error writing file:\n{}", error));
									break
								}
								index += 1
							} else {
								break
							}
						}
					}
				},
				Message::Replace => {
					if let Some((index, _)) = file_list.selected() {
						let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

						let mut open_dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
						open_dialog.set_filter("*.{ogg,flac,wav,mp3}\n*.ogg\n*.flac\n*.wav\n*.mp3");
						open_dialog.show();

						if open_dialog.filename().exists() {
							let raw = fs::read(open_dialog.filename());
							if let Err(error) = raw {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("Could not read file:\n{}", error));
								continue
							}

							if let Err(error) = list_item.set_raw(raw.unwrap()) {
								fltk::dialog::message_title("Error");
								alert(&window, &format!("Could not decode file:\n{}", error));
							}
						}
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.");
					}
				},
				Message::LoopEdit => {
					if let Some((index, _)) = file_list.selected() {
						let sound_name = file_list.get_label_of(index).unwrap();
						file_list.items[index].configure_loop(&window, &file_list.name, &sound_name, &settings.vgaudio_cli_path)
					} else {
						fltk::dialog::message_title("Alert");
						alert(&window, "Nothing is selected.")
					}
				},
				Message::Save => {
					let mut save_dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
					save_dialog.set_filter("*.nus3audio");
					save_dialog.show();

					if !save_dialog.filename().to_string_lossy().is_empty() {
						let name = save_dialog.filename().file_name().unwrap().to_string_lossy().to_string();
						let mut nus3audio = Nus3audioFile::new();

						let mut index: usize = 0;
						loop {
							if let Some(sound_name) = file_list.get_label_of(index) {
								let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");

								match list_item.get_idsp_raw(&name, &sound_name, &settings.vgaudio_cli_path) {
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
										fltk::dialog::message_title("Error");
										alert(&window, &format!("Error converting idsp:\n{}", error));
										break
									}
								}
								index += 1
							} else {
								break
							}
						}

						let mut export: Vec<u8> = Vec::new();
						nus3audio.write(&mut export);

						if let Err(error) = fs::write(save_dialog.filename().with_extension("nus3audio"), &export) {
							fltk::dialog::message_title("Error");
							alert(&window, &format!("Error writing file:\n{}", error));
						}
					}
				},
				Message::PlayPause => {
					if let Err(error) = playback.on_press(&mut file_list, &settings) {
						fltk::dialog::message_title("Error");
						alert(&window, &error);
					}
				},
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
				},
				_ => {}
			}
		}
	}

	settings.save();
	Settings::reset_cache().expect("Failed to reset the cache directory")
}
