//! Code to lay the buttons out.
//! 
//! Necessary since fltk does not cleanly handle resizing events.
use crate::rect::Rect;
use fltk::{
	prelude::*,
	browser::Browser,
	button::{
		Button,
		RadioRoundButton,
		CheckButton
	},
	dialog::{
		alert as fltk_alert,
		choice2 as fltk_choice2,
		input as fltk_input
	},
	menu::MenuBar,
	input::{
		Input,
		IntInput
	},
	// tree::Tree,
	valuator::{
		HorFillSlider
	},
	window::Window
};

/// Margins between things
pub const MARGIN: i32 = 5;
/// Height of the menu bar
pub const MENUBAR_HEIGHT: i32 = 30;
/// Height of radios, doesn't need to grow
pub const RADIO_HEIGHT: i32 = 25;

/// Lays out widgets given the window size.
#[allow(clippy::too_many_arguments)]
pub fn lay_widgets(window: &mut Window, menu: &mut MenuBar, play: &mut Button, slider: &mut HorFillSlider, list: &mut Browser) {
	let window_width = window.width();
	let window_height = window.height();

	// Keep track of the window's space
	let mut unallocated = Rect { x: 0, y: 0, width: window_width, height: window_height };

	let increment = row_height(window_height);

	// Starts easy, the MenuBar doesn't need a margin
	menu.set_pos(0, 0);
	menu.set_size(window_width, unallocated.y_consume(increment));

	// Now we need the margin
	play.set_pos(MARGIN, unallocated.y + MARGIN);
	// Play button will always be a square
	play.set_size(increment, increment);
	// Place the slider next to the play button
	slider.set_pos(MARGIN * 2 + increment, unallocated.y + MARGIN);
	slider.set_size(window_width - MARGIN * 3 - increment, increment);
	unallocated.y_bump(increment + MARGIN);

	// Now we can finally place the list
	list.set_pos(MARGIN, unallocated.y + MARGIN);
	list.set_size(window_width - MARGIN * 2, unallocated.height - MARGIN * 2);

	// Finally, redraw the window
	window.redraw()
}

/// Lays out property widgets given the window size.
#[allow(clippy::too_many_arguments)]
pub fn lay_prop_widgets(window: &mut Window, name_input: &mut Input, idsp_radio: &mut RadioRoundButton, lopus_radio: &mut RadioRoundButton, loop_toggle: &mut CheckButton, loop_from_input: &mut IntInput, loop_to_input: &mut IntInput, save_button: &mut Button) {
	let window_width = window.width();
	let window_height = window.height();

	// Keep track of the window's space
	let mut unallocated = Rect { x: 0, y: 0, width: window_width, height: window_height };

	let increment = row_height(window_height);

	// Place the name input
	name_input.set_pos(MARGIN, MARGIN);
	// Input will stretch across the window
	name_input.set_size(window_width - MARGIN * 2, increment);
	unallocated.y_bump(increment + MARGIN);

	// Place the save button
	save_button.set_pos(MARGIN, unallocated.height - MARGIN);
	save_button.set_size((window_width / 2) - 75 - MARGIN * 2, increment);

	// Place the radios
	idsp_radio.set_pos(MARGIN, unallocated.y + MARGIN);
	idsp_radio.set_size((window_width / 2) - MARGIN * 2, RADIO_HEIGHT);
	lopus_radio.set_pos(MARGIN, unallocated.y + RADIO_HEIGHT + MARGIN * 2);
	lopus_radio.set_size((window_width / 2) - MARGIN * 2, RADIO_HEIGHT);
	unallocated.x_bump(window_width / 2);

	// Place the loop toggle
	loop_toggle.set_pos(unallocated.x + MARGIN, unallocated.y + MARGIN);
	loop_toggle.set_size(unallocated.x - MARGIN * 2, RADIO_HEIGHT);
	unallocated.y_bump(RADIO_HEIGHT + MARGIN);

	// Place the loop input boxes
	// Offset the width of the inputs by a little to make room for the label they have
	loop_from_input.set_pos(unallocated.x + MARGIN + 20, unallocated.y + MARGIN);
	loop_from_input.set_size(unallocated.x - 20 - MARGIN * 2, increment);
	unallocated.y_bump(increment + MARGIN);
	loop_to_input.set_pos(unallocated.x + MARGIN + 20, unallocated.y + MARGIN);
	loop_to_input.set_size(unallocated.x - 20 - MARGIN * 2, increment);

	// Finally, redraw the window
	window.redraw()
}

/// Helpful layout function
fn row_height(window_height: i32) -> i32 {
	let maximum = MENUBAR_HEIGHT;

	let target = window_height / 5;

	if target > maximum { maximum } else { target }
}

pub fn get_x(window: &Window) -> i32 {
	window.x() + 20
}

pub fn get_y(window: &Window) -> i32 {
	window.y() + 20
}

/// Open an alert dialog somewhere near the main window.
pub fn alert(window: &Window, message: &str) {
	let x = get_x(window);
	let y = get_y(window);

	fltk_alert(x, y, message)
}

/// Open a choice dialog somewhere near the main window.
pub fn choice2(window: &Window, message: &str, c0: &str, c1: &str, c2: &str) -> Option<i32> {
	let x = get_x(window);
	let y = get_y(window);

	fltk_choice2(x, y, message, c0, c1, c2)
}

/// Open an input dialog somewhere near the main window.
pub fn input(window: &Window, message: &str, default: &str) -> Option<String> {
	let x = get_x(window);
	let y = get_y(window);

	fltk_input(x, y, message, default)
}
