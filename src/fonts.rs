// Uses
use std::{fs::File, io::Read};

use gfx_glyph::ab_glyph::FontArc;
use rust_fontconfig::{FcFontCache, FcPattern, PatternMatch};

/// Loads a font from the system by going through a list of fonts until it
/// successfully finds & loads one.
pub fn load_font(font_names: &[&str]) -> Option<FontArc> {
	// Build the cache
	let font_cache = FcFontCache::build();

	// Perform the search
	for font_name in font_names {
		let font_results = font_cache.query(&FcPattern {
			family: Some((*font_name).to_owned()),
			bold: PatternMatch::False,
			italic: PatternMatch::False,
			..Default::default()
		});

		if font_results.is_empty() {
			continue;
		}

		// Load the font
		let font_path = font_results[0];
		let mut font_bytes = Vec::new();
		let Ok(mut file) = File::open(font_path.path.as_str()) else {
			continue;
		};
		if file.read_to_end(&mut font_bytes).is_err() {
			continue;
		}

		match FontArc::try_from_vec(font_bytes) {
			Ok(font) => return Some(font),
			Err(_) => continue,
		}
	}

	None
}
