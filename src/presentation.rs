//! The parser for the [`sent`] format.
//!
//! [`sent`]: https://tools.suckless.org/sent/

// Uses
use std::{fs::read_to_string, path::Path};

use anyhow::{Context, Result as AnyhowResult};

use crate::Colour;

// Constants
const COMMENT_MARKER: char = '#';
const IMAGE_SLIDE_MARKER: char = '@';
const ESCAPE_MARKER: char = '\\';
const OPTION_MARKER: &str = "#.";
const OPTION_SEPARATOR: char = ':';

const FONT_OPTION_NAME: &str = "font";
const FOREGROUND_COLOUR_OPTION_NAME: &str = "fg";
const BACKGROUND_COLOUR_OPTION_NAME: &str = "bg";

#[derive(Clone, Debug)]
pub struct Presentation {
	pub font_list:         Vec<String>,
	pub foreground_colour: Option<Colour>,
	pub background_colour: Option<Colour>,
	pub slides:            Vec<Slide>,
}

#[derive(Clone, Debug)]
pub enum Slide {
	Text(String),
	Image(String),
	Empty,
}

impl Presentation {
	pub fn load(contents: &str) -> Self {
		let mut font_list = Vec::new();
		let mut foreground_colour = None;
		let mut background_colour = None;
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
								foreground_colour = parse_colour_hex_code(option_value);
							}
						}
						BACKGROUND_COLOUR_OPTION_NAME => {
							if background_colour.is_none() {
								background_colour = parse_colour_hex_code(option_value);
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

			// Push the line to the current paragraph
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

		Self {
			font_list,
			foreground_colour,
			background_colour,
			slides,
		}
	}

	pub fn load_from_path<P>(path: P) -> AnyhowResult<Self>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		let file_contents = read_to_string(path)
			.with_context(|| format!("unable to read the file \"{}\"", path.to_string_lossy()))?;

		Ok(Self::load(file_contents.as_str()))
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

fn parse_colour_hex_code(mut hex_value: &str) -> Option<Colour> {
	const HEX_CODE_MARKER: char = '#';
	const HEX_RADIX: u32 = 0x10;
	const EXPECTED_LENGTH: usize = 3 * 2;
	const OPAQUE_ALPHA_VALUE: f32 = 1.0;

	fn parse_single_channel(channel_hex_value: &str) -> Option<f32> {
		let parsed_value = u8::from_str_radix(channel_hex_value, HEX_RADIX).ok()?;

		Some(f32::from(parsed_value) / f32::from(u8::MAX))
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
