use fltk::{
	prelude::{ WidgetExt, ValuatorExt },
	button::Button,
	// valuator::HorNiceSlider
	valuator::HorFillSlider
};
use kira::{
	manager::{
		AudioManager,
		AudioManagerSettings,
		backend::cpal::{
			CpalBackend,
			Error as CpalError
		}
	},
	sound::static_sound::{
		PlaybackState,
		StaticSoundData,
		StaticSoundHandle,
		StaticSoundSettings
	},
	tween::{
		Easing,
		Tween
	}
};
use std::io::Cursor;

/// Play button text. FLTK gives us the option to use a nice, fancy icon.
const PLAY: &str = "@>";
/// Pause button text.
const PAUSE: &str = "@||";
/// The time between UI updates to the slider while actively playing audio.
const UPDATE_FREQUENCY: f64 = 0.1;

/// Struct that keeps the UI play button and controls kira.
pub struct Playback {
	/// The play widget.
	play_widget: Button,
	/// The slider widget.
	slider_widget: HorFillSlider,
	/// Whether or not we should be playing.
	playing: bool,
	/// Audio manager, or the error it gave.
	audio_manager: Result<AudioManager, CpalError>,
	/// Playback handle.
	playing_handle: Option<StaticSoundHandle>,
	loop_points_seconds: Option<(f64, f64)>,
	/// App sender.
	sender: fltk::app::Sender<crate::Message>
}

impl Playback {
	/// Attempt to create the audio manager.
	fn create_audio_manager() -> Result<AudioManager, CpalError> {
		let capacities = kira::manager::Capacities {
			command_capacity: 32,
			sound_capacity: 8,
			sub_track_capacity: 8,
			clock_capacity: 1
		};
		let main_track_builder = kira::track::TrackBuilder::default();
		let backend_settings = ();

		let manager_settings = AudioManagerSettings::<CpalBackend> {
			capacities,
			main_track_builder,
			backend_settings
		};
		AudioManager::<CpalBackend>::new(manager_settings)
	}

	/// Create a new instance of Self.
	pub fn new(sender: fltk::app::Sender<crate::Message>) -> Self {
		let mut play_widget = Button::default().with_label(PLAY);
		play_widget.set_tooltip("Play selected audio");
		play_widget.set_callback(move |c| c.emit(sender, crate::Message::PlayPause));

		// let mut slider_widget = HorNiceSlider::default();
		let mut slider_widget = HorFillSlider::default();
		slider_widget.set_tooltip("Position of the playing audio");
		// slider_widget.set_callback(move |c| c.emit(sender, crate::Message::Seek));
		slider_widget.deactivate();
		slider_widget.set_selection_color(fltk::enums::Color::Blue);
		slider_widget.set_minimum(0.0);
		slider_widget.set_maximum(1.0);
		slider_widget.set_step(1.0, 1);
		slider_widget.set_value(0.0);

		let audio_manager = Self::create_audio_manager();

		Self {
			play_widget,
			slider_widget,
			playing: false,
			audio_manager,
			playing_handle: None,
			loop_points_seconds: None,
			sender
		}
	}

	/// Try to get the stream handle.
	/// 
	/// If the stream handle isn't set already, tries to create it again.
	/// Otherwise, this does nothing.
	pub fn get_manager(&mut self) {
		if self.audio_manager.is_ok() { return }
		
		self.audio_manager = Self::create_audio_manager()
	}

	/// Updates the value of the slider widget to match the sink position.
	pub fn on_update(&mut self) {
		if self.playing {
			if let Some(handle) = &mut self.playing_handle {
				// if let Some((begin, end)) = self.loop_points_seconds {
				// 	let pos = handle.position();
				// 	if pos > end || float_cmp::approx_eq!(f64, pos, end) {
				// 		println!("looping!");
				// 		let _ = handle.seek_to(begin);
				// 		let _ = handle.resume(Self::no_tween());
				// 	}
				// }
				self.slider_widget.set_value(handle.position());
				// No need to run more updates if it's paused
				if handle.state() != PlaybackState::Playing {
					self.playing = false;
					self.play_widget.set_label(PLAY)
				} else {
					Self::queue_update(self.sender)
				}
				self.slider_widget.redraw()
			} else {
				self.playing = false;
				self.play_widget.set_label(PLAY)
			}
		}
		// Do nothing if we aren't playing anything
	}

	// pub fn on_seek(&mut self) {
	// 	if self.playing {
	// 		if let Some(sink) = &mut self.sink {
	// 			if let Ok(sink) = sink {
	// 				println!("setting pos");
	// 				sink.set_pos(self.slider_widget.value() as f32)
	// 			}
	// 		}
	// 	}
	// }

	/// Queue the slider update.
	fn queue_update(sender: fltk::app::Sender<crate::Message>) {
		fltk::app::add_timeout(UPDATE_FREQUENCY, move || sender.send(crate::Message::Update))
	}

	/// Try to play the currently selected sound.
	pub fn on_press(&mut self, file_list: &mut crate::list::List) -> Result<(), String> {
		// Make sure we have the audio manager
		self.get_manager();

		let result = match &mut self.audio_manager {
			Ok(manager) => {
					// Stream is fine
				match &mut self.playing_handle {
					Some(handle) if handle.state() != PlaybackState::Stopped => {
						// Already have a playback handle
						if handle.state() == PlaybackState::Paused {
							self.play_widget.set_label(PAUSE);
							self.playing = true;
							if let Err(error) = handle.resume(Tween::default()) {
								return Err(error.to_string())
							}
							// if let Some(time) = self.pause_time.take() {
							// 	self.start_time += time.elapsed()
							// }
							Self::queue_update(self.sender);
							Ok(())
						} else {
							self.play_widget.set_label(PLAY);
							if let Err(error) = handle.pause(Tween::default()) {
								return Err(error.to_string())
							}
							// self.pause_time = Some(Instant::now());
							Ok(())
						}
					},
					_ => {
						// No handle
						// Check if anything is selected
						if let Some((index, _)) = file_list.selected() {
							let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
							let raw = list_item.audio_raw.clone();
							match raw {
								Some(mut data) => {
									self.loop_points_seconds = list_item.loop_points_seconds().clone();
	
									// Set the length to the end of the loop *or* the actual length
									let length = if false /*let Some((_, end)) = list_item.loop_points()*/ {
										// if data.len() <= *end {
										// 	data.len()
										// } else {
										// 	*end
										// }
										unreachable!()
									} else {
										data.len()
									};
									// 
									data.resize_with(length, || panic!());
									// Create a cursor for the buffer
									let buffer = Cursor::new(data);
									// Create the sound settings
									let mut settings = StaticSoundSettings::default();
									if let Some((begin, _)) = self.loop_points_seconds {
										settings.loop_behavior = Some(kira::LoopBehavior { start_position: begin });
									}
									// Create the sound data
									let sound_data = StaticSoundData::from_cursor(buffer, settings);
									
									match sound_data {
										Ok(s) => {
											let duration = s.duration();
												self.slider_widget.set_bounds(0.0, duration.as_secs_f64());
											self.slider_widget.set_step((duration.as_secs_f64() / 20.0).min(0.2), 2);
											
											self.play_widget.set_label(PAUSE);
											self.playing = true;
											self.sender.send(crate::Message::Update);
											match manager.play(s) {
												Ok(handle) => self.playing_handle = Some(handle),
												Err(error) => return Err(error.to_string())
											};
											Ok(())
										},
										Err(error) => {
											Err(format!("Could not play audio:\n{}", error))
										}
									}
								},
								None => {
									Err("Audio of selected item is empty.".to_owned())
								}
							}
						} else {
							Err("Nothing is selected.".to_owned())
						}
					}
				}
				},
				Err(error) => Err(error.to_string())
		};
		if let Some((begin, end)) = self.loop_points_seconds {
			println!("from {} to {}", begin, end)
			}
		println!("{}", self.playing_handle.is_some());
		result
	}

	/// Stop the current sink.
	pub fn stop_sink(&mut self) {
		if let Some(handle) = &mut self.playing_handle {
			let _ = handle.stop(Self::no_tween());
		}
		self.play_widget.set_label(PLAY);
		self.slider_widget.set_value(0.0);
			self.playing = false;
		self.loop_points_seconds = None;
		self.playing_handle = None
	}

	/// Returns the [&mut Browser] widget of this List.
	pub fn get_widgets_mut(&mut self) -> (&mut Button, &mut HorFillSlider) {
		(&mut self.play_widget, &mut self.slider_widget)
	}

	pub fn no_tween() -> Tween {
		Tween { start_time: kira::StartTime::Immediate, duration: std::time::Duration::from_secs(0), easing: Easing::Linear }
	}
}
