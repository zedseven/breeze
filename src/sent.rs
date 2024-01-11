//! The parser for the [`sent`] format.
//!
//! [`sent`]: https://tools.suckless.org/sent/

// Uses
use std::{fs::read_to_string, path::Path};

use anyhow::{Context, Result as AnyhowResult};

// Constants
const COMMENT_CHAR: char = '#';
const IMAGE_SLIDE_CHAR: char = '@';
const ESCAPE_CHAR: char = '\\';

#[derive(Clone, Debug)]
pub struct Presentation(pub Vec<Slide>);

#[derive(Clone, Debug)]
pub enum Slide {
	Text(String),
	Image(String),
	Empty,
}

impl Presentation {
	pub fn load(contents: &str) -> Self {
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

			// Skip comments and text following image slides
			if line_trimmed.starts_with(COMMENT_CHAR) || skip_remainder_of_paragraph {
				continue;
			}

			// Handle image slides
			if current_paragraph.is_empty() && line_trimmed.starts_with(IMAGE_SLIDE_CHAR) {
				slides.push(Slide::Image(line_trimmed[1..].to_owned()));
				skip_remainder_of_paragraph = true;

				continue;
			}

			// Push the line to the current paragraph
			if line_trimmed.starts_with(ESCAPE_CHAR) {
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

		Self(slides)
	}

	pub fn load_from_path<P>(path: P) -> AnyhowResult<Self>
	where
		P: AsRef<Path> + Copy,
	{
		let file_contents = read_to_string(path).with_context(|| {
			format!(
				"unable to read the file \"{}\"",
				path.as_ref().to_string_lossy()
			)
		})?;

		Ok(Self::load(file_contents.as_str()))
	}

	pub fn try_get_title(&self) -> Option<String> {
		const MAXIMUM_TITLE_LENGTH: usize = 64;
		const ELLIPSIS: char = '…';

		self.0.iter().find_map(|slide| match slide {
			Slide::Text(text) => {
				// Since the user is expected to wrap the text on their own, newlines need to be
				// converted to spaces so the slide contents are on one long line
				// The trimming is to prevent having multiple spaces in the title, which looks
				// ugly
				let mut title_text = String::with_capacity(text.len());
				for line in text.lines().map(|line| line.trim()) {
					if !title_text.is_empty() {
						title_text.push(' ');
					}
					title_text.push_str(line);
				}

				// Truncate to the maximum length and put an ellipsis on the end if so
				title_text.truncate(MAXIMUM_TITLE_LENGTH - 1);
				if title_text.len() == MAXIMUM_TITLE_LENGTH - 1 {
					title_text.push(ELLIPSIS);
				}

				Some(title_text)
			}
			Slide::Image(_) | Slide::Empty => None,
		})
	}
}
