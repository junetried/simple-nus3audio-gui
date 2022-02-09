use fltk::{
	prelude::{ WidgetExt, ValuatorExt },
	button::Button,
	// valuator::HorNiceSlider
	valuator::HorFillSlider
};
use rodio::{
	Decoder as AudioDecoder,
	OutputStream,
	OutputStreamHandle,
	PlayError,
	Sink as AudioSink,
	source::Source as AudioSource,
	StreamError
};
use std::{
	io::Cursor,
	time::Instant
};

/// Play button text. FLTK gives us the option to use a nice, fancy icon.
const PLAY: &str = "@>";
/// Pause button text.
// const PAUSE: &str = "@||";
/// The time between UI updates to the slider while actively playing audio.
const UPDATE_FREQUENCY: f64 = 0.1;

/// Struct that keeps the UI play button and controls rodio.
pub struct Playback {
	/// The play widget.
	play_widget: Button,
	/// The slider widget.
	slider_widget: HorFillSlider,
	/// Whether or not we should be playing.
	playing: bool,
	/// The instant the button was pressed.
	start_time: Instant,
	// Seeking in rodio is not possible. Pausing with no seek doesn't seem very useful.
	// pause_time: Option<Instant>,
	/// Stream handle, or the error it gave.
	stream_handle: Result<(OutputStream, OutputStreamHandle), StreamError>,
	/// Sink, or the error it gave.
	sink: Option<Result<AudioSink, PlayError>>,
	/// App sender.
	sender: fltk::app::Sender<crate::Message>
}

impl Playback {
	/// Attempt to create the stream handle.
	fn create_stream_handle() -> Result<(OutputStream, OutputStreamHandle), StreamError> {
		OutputStream::try_default()
	}

	/// Create the sink from the stream handle result.
	/// 
	/// Returns [None] if the handle is an error, otherwise tries to create the sink and returns the result in an [Option].
	fn create_sink(handle: &Result<(OutputStream, OutputStreamHandle), StreamError>) -> Option<Result<AudioSink, PlayError>> {
		match handle {
			Ok((_, handle)) => {
				Some(AudioSink::try_new(handle))
			},
			Err(_) => None
		}
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

		let stream_handle = Self::create_stream_handle();
		let sink = Self::create_sink(&stream_handle);

		Self {
			play_widget,
			slider_widget,
			playing: false,
			start_time: Instant::now(),
			// pause_time: None,
			stream_handle,
			sink,
			sender
		}
	}

	/// Try to get the stream handle.
	/// 
	/// If the stream handle isn't set already, tries to create it again.
	/// Otherwise, this does nothing.
	pub fn get_handle(&mut self) {
		if self.stream_handle.is_ok() { return }
		
		self.stream_handle = Self::create_stream_handle()
	}

	/// Try to get a sink.
	/// 
	/// If the current sink isn't set already, tries to create it again.
	/// Otherwise, this does nothing.
	pub fn get_sink(&mut self) {
		self.get_handle();

		if self.sink.is_some() && self.sink.as_ref().unwrap().is_ok() { return }

		self.sink = Self::create_sink(&self.stream_handle)
	}

	// pub fn get_time(&self) -> Instant {
	// 	if let Some(time) = self.pause_time {
	// 		self.start_time - time.elapsed()
	// 	} else {
	// 		self.start_time
	// 	}
	// }

	/// Updates the value of the slider widget to match the sink position.
	pub fn on_update(&mut self) {
		if self.playing {
			if let Some(Ok(sink)) = &self.sink {
					self.slider_widget.set_value(self.start_time.elapsed().as_secs_f64());
					// No need to run more updates if it's paused
					if sink.is_paused() || sink.empty() {
						self.playing = false;
						self.play_widget.set_label(PLAY)
					} else {
						Self::queue_update(self.sender)
					}
					self.slider_widget.redraw()
				} else {
					self.playing = false
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
	pub fn on_press(&mut self, file_list: &mut crate::list::List, settings: &crate::settings::Settings) -> Result<(), String> {
		// Stop any playback already happening
		self.stop_sink();

		// Make sure we should have a sink by this point
		self.get_sink();

		if let Some(sink) = &mut self.sink {
			match sink {
				Ok(sink) => {
					// Stream is fine
					// if !sink.empty() {
					if false {
						if sink.is_paused() {
							// self.play_widget.set_label(PAUSE);
							self.playing = true;
							sink.play();
							// if let Some(time) = self.pause_time.take() {
							// 	self.start_time += time.elapsed()
							// }
							Self::queue_update(self.sender);
							Ok(())
						} else {
							self.play_widget.set_label(PLAY);
							sink.pause();
							// self.pause_time = Some(Instant::now());
							Ok(())
						}
					} else {
						// Check if anything is selected
						if let Some((index, sound_name)) = file_list.selected() {
							let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
							let raw = list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path);
							match raw {
								Ok(data) => {
									// Create a cursor for the buffer
									let buffer = Cursor::new(data);
									// Create the source
									let source = AudioDecoder::new(buffer);
									
									match source {
										Ok(s) => {
											if let Some(duration) = s.total_duration() {
												self.slider_widget.set_bounds(0.0, duration.as_secs_f64());
												self.slider_widget.set_step((duration.as_secs_f64() / 20.0).min(0.2), 2)
											}
											
											// self.play_widget.set_label(PAUSE);
											sink.append(s);
											self.start_time = Instant::now();
											self.playing = true;
											self.sender.send(crate::Message::Update);
											sink.play();
											Ok(())
										},
										Err(error) => {
											Err(format!("Could not play audio:\n{}", error))
										}
									}
								},
								Err(error) => {
									Err(format!("Error converting idsp:\n{}", error))
								}
							}
						} else {
							Err("Nothing is selected.".to_owned())
						}
					}
				},
				Err(error) => Err(error.to_string())
			}
		} else {
			// Output handle is fine but sink has no result
			// This shouldn't be reachable, unless I did something wrong
			unreachable!()
		}
	}

	/// Stop the current sink.
	pub fn stop_sink(&mut self) {
		self.slider_widget.set_value(0.0);
		if let Some(Ok(sink)) = &mut self.sink {
				sink.stop();
				self.playing = false;
				self.play_widget.set_label(PLAY);
				// https://github.com/RustAudio/rodio/issues/315
				self.sink = None
		}
	}

	/// Returns the [&mut Browser] widget of this List.
	pub fn get_widgets_mut(&mut self) -> (&mut Button, &mut HorFillSlider) {
		(&mut self.play_widget, &mut self.slider_widget)
	}
}
