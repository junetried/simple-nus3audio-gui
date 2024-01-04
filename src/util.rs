/// Rect helper struct.
pub struct Rect {
	pub x: i32,
	pub y: i32,
	pub width: i32,
	pub height: i32
}

impl Rect {
	/// Bump the x by `value` amount while keeping the width the same
	pub fn x_bump(&mut self, value: i32) {
		self.x += value;
		self.width -= value
	}

	/// Bump the y by `value` amount while keeping the height the same
	pub fn y_bump(&mut self, value: i32) {
		self.y += value;
		self.height -= value
	}

	#[allow(dead_code)]
	/// Bump the x by `value` amount and return `value`
	pub fn x_consume(&mut self, value: i32) -> i32 {
		self.x_bump(value);
		value
	}

	/// Bump the y by `value` amount and return `value`
	pub fn y_consume(&mut self, value: i32) -> i32 {
		self.y_bump(value);
		value
	}
}

/// Return a human-readable data size from a number of bytes.
pub fn human_readable_size(bytes: u64) -> String {
	if bytes < 1_000 {
		format!("{} bytes", bytes)
	} else {
		let kilobytes = bytes as f64 / 1_000.0;
		if kilobytes < 1_000.0 {
			format!("{} KB", kilobytes)
		} else {
			let megabytes = kilobytes / 1_000.0;
			// This is as high as we'll go for now.
			format!("{} MB", megabytes)
		}
	}
}
