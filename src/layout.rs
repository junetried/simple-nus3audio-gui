//! Code to lay the buttons out.
//! 
//! Necessary since fltk does not cleanly handle resizing events.
use crate::rect::Rect;
use fltk::{
	prelude::*,
	browser::Browser,
	button::Button,
	dialog::{
		alert as fltk_alert,
		choice2 as fltk_choice2,
		input as fltk_input
	},
	input::IntInput,
	menu::MenuBar,
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

/// Lays out widgets given the window size.
pub fn lay_widgets(window: &mut Window, menu: &mut MenuBar, play: &mut Button, slider: &mut HorFillSlider, list: &mut Browser, start_input: &mut IntInput, end_input: &mut IntInput) {
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

	if start_input.visible() { unallocated.height -= increment + MARGIN }
	// Inputs will each be half a square
	start_input.set_pos(MARGIN, unallocated.y + unallocated.height);
	start_input.set_size(unallocated.width / 2 - MARGIN * 2, increment);
	end_input.set_pos(unallocated.width / 2, unallocated.y + unallocated.height);
	end_input.set_size(unallocated.width / 2 - MARGIN, increment);

	// Now we can finally place the list
	list.set_pos(MARGIN, unallocated.y + MARGIN);
	list.set_size(window_width - MARGIN * 2, unallocated.height - MARGIN * 2);

	// Finally, redraw the windows
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
