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

/// Play button text. In this case, FLTK gives us the option to use a nice, fancy icon.
const PLAY: &str = "@>";
/// Pause button text.
// const PAUSE: &str = "@||";
/// The time between UI updates to the slider while actively playing audio.
const UPDATE_FREQUENCY: f64 = 0.1;

pub struct Playback {
	play_widget: Button,
	slider_widget: HorFillSlider,
	playing: bool,
	start_time: Instant,
	pause_time: Option<Instant>,
	stream_handle: Result<(OutputStream, OutputStreamHandle), StreamError>,
	sink: Option<Result<AudioSink, PlayError>>,
	sender: fltk::app::Sender<crate::Message>
}

impl Playback {
	fn create_stream_handle() -> Result<(OutputStream, OutputStreamHandle), StreamError> {
		OutputStream::try_default()
	}

	fn create_sink(handle: &Result<(OutputStream, OutputStreamHandle), StreamError>) -> Option<Result<AudioSink, PlayError>> {
		match handle {
			Ok((_, handle)) => {
				Some(AudioSink::try_new(&handle))
			},
			Err(_) => None
		}
	}

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
			pause_time: None,
			stream_handle,
			sink,
			sender
		}
	}

	pub fn get_handle(&mut self) {
		if self.stream_handle.is_ok() { return () }
		
		self.stream_handle = Self::create_stream_handle()
	}

	pub fn get_sink(&mut self) {
		self.get_handle();

		if self.sink.is_some() {
			if self.sink.as_ref().unwrap().is_ok() {
				return ()
			}
		}

		self.sink = Self::create_sink(&self.stream_handle)
	}

	pub fn get_time(&self) -> Instant {
		if let Some(time) = self.pause_time {
			self.start_time - time.elapsed()
		} else {
			self.start_time
		}
	}

	pub fn on_update(&mut self) {
		if self.playing {
			if let Some(sink) = &self.sink {
				if let Ok(sink) = sink {
					self.slider_widget.set_value(self.get_time().elapsed().as_secs_f64());
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

	fn queue_update(sender: fltk::app::Sender<crate::Message>) {
		fltk::app::add_timeout(UPDATE_FREQUENCY, move || sender.send(crate::Message::Update))
	}

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
							if let Some(time) = self.pause_time.take() {
								self.start_time += time.elapsed()
							}
							Self::queue_update(self.sender);
							Ok(())
						} else {
							self.play_widget.set_label(PLAY);
							sink.pause();
							self.pause_time = Some(Instant::now());
							Ok(())
						}
					} else {
						// Check if anything is selected
						if let Some((index, sound_name)) = file_list.selected() {
							let list_item = file_list.items.get_mut(index).expect("Failed to find internal list item");
							let raw = list_item.get_raw(&file_list.name, &sound_name, &settings.vgaudio_cli_path).clone();
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

	pub fn stop_sink(&mut self) {
		self.slider_widget.set_value(0.0);
		if let Some(sink) = &mut self.sink {
			if let Ok(sink) = sink {
				sink.stop();
				self.playing = false;
				self.play_widget.set_label(PLAY);
				// https://github.com/RustAudio/rodio/issues/315
				self.sink = None
			}
		}
	}

	/// Returns the [&mut Browser] widget of this List.
	pub fn get_widgets_mut(&mut self) -> (&mut Button, &mut HorFillSlider) {
		(&mut self.play_widget, &mut self.slider_widget)
	}
}
