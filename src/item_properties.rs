use fltk::{
	prelude::*,
	app,
	button::{
		Button,
		RadioRoundButton,
		CheckButton
	},
	enums::Event,
	input::{
		Input,
		IntInput
	},
	window::Window
};
use crate::{
	list::{
		AudioExtension,
		ListItem
	},
	layout::{
		self,
		alert
	}
};

#[derive(Clone)]
enum PropMessage {
	ReLay,
	BinRadio,
	ToggleLoop,
	Save
}

/// Configure a single item. Returns a boolean representing whether or not the item was modified.
pub fn configure(item: &mut ListItem, parent: &Window) -> bool {
	let (s, r) = app::channel();

	let mut window = Window::new(parent.x(), parent.y(), 350, 125, Some("Properties"))
		.with_label(&format!("Properties of {}", &item.name));
	window.make_resizable(true);
	window.size_range(350, 170, 0, 0);

	let mut name_input = Input::default();
	name_input.set_tooltip("Unique name of the sound");
	name_input.set_value(&item.name);

	// Create the two radio buttons for format
	let mut idsp_radio = RadioRoundButton::default()
		.with_label("IDSP format");
	idsp_radio.set_tooltip("Used for lower-quality sound effects");
	idsp_radio.toggle(item.extension == AudioExtension::Idsp);
	idsp_radio.emit(s.clone(), PropMessage::BinRadio);

	let mut lopus_radio = RadioRoundButton::default()
		.with_label("LOPUS format");
	lopus_radio.set_tooltip("Used for high-quality music");
	lopus_radio.toggle(item.extension == AudioExtension::Lopus);
	lopus_radio.emit(s.clone(), PropMessage::BinRadio);

	let mut bin_radio = RadioRoundButton::default()
		.with_label("Binary data");
	bin_radio.set_tooltip("Any data which is not audio");
	bin_radio.toggle(item.extension == AudioExtension::Bin);
	bin_radio.emit(s.clone(), PropMessage::BinRadio);

	// Create the loop toggle button
	let mut loop_toggle = CheckButton::default()
		.with_label("Loop audio");
	loop_toggle.set_tooltip("Whether or not this audio will loop");
	loop_toggle.emit(s.clone(), PropMessage::ToggleLoop);

	// Create the loop from input
	let mut loop_from_input = IntInput::default()
		.with_label("Loop from");
	loop_from_input.set_tooltip("Beginning of the loop in samples, starts again here when reaching the end of the loop");

	// Create the loop to input
	let mut loop_to_input = IntInput::default()
		.with_label("Loop to");
	loop_to_input.set_tooltip("End of the loop in samples, when it reaches this point it loops back to the beginning of the loop");

	// Set the value of the loop things
	if let Some((from, to)) = item.loop_points() {
		loop_toggle.set(true);
		loop_from_input.set_value(&from.to_string());
		loop_to_input.set_value(&to.to_string())
	} else {
		loop_toggle.set(false);
		loop_from_input.deactivate();
		loop_to_input.deactivate()
	}

	// Create the button to apply changes
	let mut save_button = Button::default()
		.with_label("Ok");
	save_button.set_tooltip("Apply changes and close this window");
	save_button.emit(s.clone(), PropMessage::Save);

	window.handle(move |_, event| match event {
		Event::Resize => {
			s.send(PropMessage::ReLay);
			true
		},
		_ => { false }
	});

	window.end();
	layout::lay_prop_widgets(&mut window, &mut name_input, &mut idsp_radio, &mut lopus_radio,  &mut bin_radio, &mut loop_toggle, &mut loop_from_input, &mut loop_to_input, &mut save_button);
	window.show();

	let mut apply = false;

	// Mini event loop
	while window.shown() {
		app::wait();
		if let Some(e) = r.recv() {
			match e {
				PropMessage::ReLay => layout::lay_prop_widgets(&mut window, &mut name_input, &mut idsp_radio, &mut lopus_radio, &mut bin_radio, &mut loop_toggle, &mut loop_from_input, &mut loop_to_input, &mut save_button),
				PropMessage::BinRadio => {
					if bin_radio.is_toggled() {
						loop_toggle.set_checked(false);
						loop_toggle.deactivate();
						loop_from_input.deactivate();
						loop_to_input.deactivate()
					} else {
						loop_toggle.activate();
					}
				},
				PropMessage::ToggleLoop => {
					if item.loop_points().is_some() {
						loop_from_input.deactivate();
						loop_to_input.deactivate()
					} else {
						loop_from_input.activate();
						loop_from_input.set_value("0");
						loop_to_input.activate();
						loop_to_input.set_value(&item.length_in_samples.to_string())
					}
				},
				PropMessage::Save => {
					// usize can't be signed
					if loop_from_input.value().contains('-') || loop_to_input.value().contains('-') {
						alert(&window, "Loop points must be positive.");
						continue
					}
					if loop_toggle.is_checked() {
						// End can't be before beginning
						if loop_from_input.value().parse::<usize>().unwrap_or(0) >= loop_to_input.value().parse().unwrap_or(0) {
							alert(&window, "Loop beginning must be placed before loop end.");
							continue
						}
					}
					apply = true;
					window.hide()
				}
			}
		}
	}

	// Window has been closed, so now apply the settings
	if apply {
		let new_name = name_input.value();
		let new_extension = {
			if idsp_radio.is_toggled() { AudioExtension::Idsp }
			else if lopus_radio.is_toggled() { AudioExtension::Lopus }
			else { AudioExtension::Bin }
		};
		let new_loop = if loop_toggle.is_checked() {
			Some((loop_from_input.value().parse().unwrap_or(0), loop_to_input.value().parse().unwrap_or(0)))
		} else {
			None
		};

		if item.name == new_name && item.extension == new_extension && *item.loop_points() == new_loop {
			false
		} else {
			if item.extension != new_extension || *item.loop_points() == new_loop {
				item.clear_bytes();
			}
			item.name = new_name;
			item.extension = new_extension;
			item.loop_points_samples = new_loop;
			true
		}
	} else { false }
}