//! The parser for the [`sent`] format.
//!
//! [`sent`]: https://tools.suckless.org/sent/

// Uses
use std::{fs::read_to_string, path::Path};

use crate::LinearRgbaColour;

// Constants
const COMMENT_MARKER: char = '#';
const IMAGE_SLIDE_MARKER: char = '@';
const ESCAPE_MARKER: char = '\\';
const OPTION_MARKER: &str = "#.";
const OPTION_SEPARATOR: char = ':';

const FONT_OPTION_NAME: &str = "font";
const FOREGROUND_COLOUR_OPTION_NAME: &str = "fg";
const BACKGROUND_COLOUR_OPTION_NAME: &str = "bg";
const SHOW_CURSOR_OPTION_NAME: &str = "cursor";

#[derive(Clone, Debug, PartialEq)]
pub struct Presentation {
	pub font_list:         Vec<String>,
	pub foreground_colour: Option<LinearRgbaColour>,
	pub background_colour: Option<LinearRgbaColour>,
	pub show_cursor:       Option<bool>,
	pub slides:            Vec<Slide>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Slide {
	Text(String),
	Image(String),
	Empty,
}

impl Presentation {
	pub fn load(contents: &str) -> Result<Self, String> {
		let mut font_list = Vec::new();
		let mut foreground_colour = None;
		let mut background_colour = None;
		let mut show_cursor = None;
		let mut slides = Vec::new();

		let mut current_paragraph = String::new();
		let mut skip_remainder_of_paragraph = false;
		for line in contents.lines() {
			let mut line_trimmed = line.trim_end();

			// If the line is empty, the paragraph is complete
			if line_trimmed.is_empty() {
				if !current_paragraph.is_empty() {
					slides.push(Slide::Text(current_paragraph));
					current_paragraph = String::new();
				}

				skip_remainder_of_paragraph = false;

				continue;
			}

			// Parse presentation options
			if line_trimmed.starts_with(OPTION_MARKER) {
				if let Some((option_name, option_value)) = line_trimmed
					.strip_prefix(OPTION_MARKER)
					.expect("the string starts with the prefix")
					.split_once(OPTION_SEPARATOR)
				{
					match option_name {
						FONT_OPTION_NAME => font_list.push(option_value.to_owned()),
						FOREGROUND_COLOUR_OPTION_NAME => {
							if foreground_colour.is_none() {
								foreground_colour =
									Some(parse_colour_hex_code(option_value).ok_or_else(|| {
										format!(
											"foreground colour \"{option_value}\" is not valid!",
										)
									})?);
							}
						}
						BACKGROUND_COLOUR_OPTION_NAME => {
							if background_colour.is_none() {
								background_colour =
									Some(parse_colour_hex_code(option_value).ok_or_else(|| {
										format!(
											"background colour \"{option_value}\" is not valid!",
										)
									})?);
							}
						}
						SHOW_CURSOR_OPTION_NAME => {
							if show_cursor.is_none() {
								show_cursor = Some(parse_bool(option_value).ok_or_else(|| {
									format!(
										"show cursor value \"{option_value}\" is not valid!\nit \
										 must be \"true\" or \"false\"",
									)
								})?);
							}
						}
						_ => {}
					}
				}

				continue;
			}

			// Skip comments and text following image slides
			if line_trimmed.starts_with(COMMENT_MARKER) || skip_remainder_of_paragraph {
				continue;
			}

			// Handle image slides
			if current_paragraph.is_empty() && line_trimmed.starts_with(IMAGE_SLIDE_MARKER) {
				slides.push(Slide::Image(line_trimmed[1..].to_owned()));
				skip_remainder_of_paragraph = true;

				continue;
			}

			// Remove the escape character if present
			if line_trimmed.starts_with(ESCAPE_MARKER) {
				line_trimmed = &line_trimmed[1..];
			}

			// If, after removing the escape character, the line is empty, this is an empty
			// slide
			if line_trimmed.is_empty() {
				if current_paragraph.is_empty() {
					slides.push(Slide::Empty);
					skip_remainder_of_paragraph = true;
				}

				continue;
			}

			// Insert a line separator
			if !current_paragraph.is_empty() {
				current_paragraph.push('\n');
			}
			current_paragraph.push_str(line_trimmed);
		}

		if !current_paragraph.is_empty() {
			slides.push(Slide::Text(current_paragraph));
		}

		// Ensure the presentation always has at least one slide
		if slides.is_empty() {
			slides.push(Slide::Empty);
		}

		// Construct the final result
		Ok(Self {
			font_list,
			foreground_colour,
			background_colour,
			show_cursor,
			slides,
		})
	}

	pub fn load_from_path<P>(path: P) -> Result<Self, String>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		let file_contents = read_to_string(path).map_err(|_| {
			format!(
				"unable to read the presentation file\n\"{}\"!",
				path.to_string_lossy()
			)
		})?;

		Self::load(file_contents.as_str())
	}

	pub fn try_get_title(&self) -> Option<String> {
		const MAXIMUM_TITLE_LENGTH: usize = 64;
		const ELLIPSIS: char = '\u{2026}';

		self.slides.iter().find_map(|slide| match slide {
			Slide::Text(text) => {
				// Since the user is expected to wrap the text on their own, newlines need to be
				// converted to spaces so the slide contents are on one long line
				// The trimming is to prevent having multiple spaces in the title, which looks
				// ugly
				let mut title_text = String::with_capacity(text.len());
				for line in text.lines().map(str::trim) {
					if !title_text.is_empty() {
						title_text.push(' ');
					}
					title_text.push_str(line);
				}

				// Truncate to the maximum length and put an ellipsis on the end if so
				if char_truncate(&mut title_text, MAXIMUM_TITLE_LENGTH - 1) {
					title_text.push(ELLIPSIS);
				}

				Some(title_text)
			}
			Slide::Image(_) | Slide::Empty => None,
		})
	}
}

impl Default for Presentation {
	fn default() -> Self {
		Self {
			font_list:         vec![],
			foreground_colour: None,
			background_colour: None,
			show_cursor:       None,
			slides:            vec![Slide::Empty],
		}
	}
}

impl From<String> for Presentation {
	fn from(value: String) -> Self {
		Self {
			slides: vec![Slide::Text(value)],
			..Default::default()
		}
	}
}

fn parse_bool(bool_string: &str) -> Option<bool> {
	match bool_string {
		"true" => Some(true),
		"false" => Some(false),
		_ => None,
	}
}

fn parse_colour_hex_code(mut hex_value: &str) -> Option<LinearRgbaColour> {
	const HEX_CODE_MARKER: char = '#';
	const HEX_RADIX: u32 = 0x10;
	const EXPECTED_LENGTH: usize = 3 * 2;
	const OPAQUE_ALPHA_VALUE: f32 = 1.0;

	fn parse_single_channel(channel_hex_value: &str) -> Option<f32> {
		let parsed_value = u8::from_str_radix(channel_hex_value, HEX_RADIX).ok()?;
		let srgb_value = f32::from(parsed_value) / f32::from(u8::MAX);
		let linear_rgb_value = srgb_to_linear_rgb_channel(srgb_value);

		Some(linear_rgb_value)
	}

	// Remove the leading marker character if present
	if hex_value.starts_with(HEX_CODE_MARKER) {
		hex_value = hex_value
			.strip_prefix(HEX_CODE_MARKER)
			.expect("the string starts with the prefix");
	}

	// Trim trailing whitespace
	hex_value = hex_value.trim_end();

	// Ensure the value is of the correct length
	if hex_value.len() != EXPECTED_LENGTH {
		return None;
	}

	// Parse the channels
	Some([
		parse_single_channel(&hex_value[0..2])?,
		parse_single_channel(&hex_value[2..4])?,
		parse_single_channel(&hex_value[4..6])?,
		OPAQUE_ALPHA_VALUE,
	])
}

/// Truncates based on Unicode char boundaries instead of bytes.
///
/// This avoids potential panics when using the base [`truncate`] function.
///
/// Returns whether anything was actually truncated.
///
/// [`truncate`]: String::truncate
fn char_truncate(string: &mut String, maximum_chars: usize) -> bool {
	if let Some((index, _)) = string.char_indices().nth(maximum_chars) {
		string.truncate(index);

		return true;
	}

	false
}

/// Converts an sRGB value to linear RGB.
///
/// This implementation matches what is specified here: https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_sRGB_decode.txt
fn srgb_to_linear_rgb_channel(srgb_value: f32) -> f32 {
	const GAMMA: f32 = 2.4;
	const A: f32 = 0.055;
	const X: f32 = 0.04045;
	const PHI: f32 = 12.92;

	if srgb_value > X {
		((srgb_value + A) / (1.0 + A)).powf(GAMMA)
	} else {
		srgb_value / PHI
	}
}

#[cfg(test)]
mod tests {
	// Uses
	use super::{Presentation, Slide};

	#[test]
	fn many_slides() {
		let actual_result = Presentation::load(
			r"
This is a text slide.

Text slide with multiple lines:
- item 1
- item 2
- item 3

Another text slide!

\

@image.png
This text won't be shown, since this is an image slide

Final slide
",
		)
		.slides;

		let expected_result = vec![
			Slide::Text(r"This is a text slide.".to_owned()),
			Slide::Text(
				r"Text slide with multiple lines:
- item 1
- item 2
- item 3"
					.to_owned(),
			),
			Slide::Text(r"Another text slide!".to_owned()),
			Slide::Empty,
			Slide::Image("image.png".to_owned()),
			Slide::Text(r"Final slide".to_owned()),
		];

		assert_eq!(expected_result, actual_result);
	}

	#[test]
	fn comments() {
		let actual_result = Presentation::load(
			r"
# Comment at the beginning of a text slide
Text slide
# Comment at the end of a text slide

# Solitary comment

Another text slide

A text slide demonstrating that comments
don't work unless they're at the beginning
of the line: # Comment

# Comment at the end of the file
",
		)
		.slides;

		let expected_result = vec![
			Slide::Text(r"Text slide".to_owned()),
			Slide::Text(r"Another text slide".to_owned()),
			Slide::Text(
				r"A text slide demonstrating that comments
don't work unless they're at the beginning
of the line: # Comment"
					.to_owned(),
			),
		];

		assert_eq!(expected_result, actual_result);
	}

	#[test]
	fn configuration() {
		let actual_result = Presentation::load(
			r"
#.font:Roboto
#.font:Helvetica
#.fg:#ffffff
#.bg:#000000

This is a presentation for testing the configuration parameters.
",
		);

		let expected_result = Presentation {
			font_list:         vec!["Roboto".to_owned(), "Helvetica".to_owned()],
			foreground_colour: Some([1.0, 1.0, 1.0, 1.0]),
			background_colour: Some([0.0, 0.0, 0.0, 1.0]),
			slides:            vec![Slide::Text(
				"This is a presentation for testing the configuration parameters.".to_owned(),
			)],
		};

		assert_eq!(expected_result, actual_result);
	}

	#[test]
	fn get_title() {
		let actual_result = Presentation::load(
			r"
First Slide

A text slide with some content
",
		)
		.try_get_title();

		let expected_result = Some("First Slide".to_owned());

		assert_eq!(expected_result, actual_result);
	}
}
