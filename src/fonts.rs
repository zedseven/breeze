// Uses
use std::{fs::File, io::Read};

use anyhow::{anyhow, Context, Result as AnyhowResult};
use gfx_glyph::ab_glyph::FontArc;
use rust_fontconfig::{FcFontCache, FcPattern, PatternMatch};

/// Loads a font from the system by name.
pub fn load_font(font_name: &str) -> AnyhowResult<FontArc> {
	// Build the cache
	let font_cache = FcFontCache::build();

	// Perform the search
	let font_results = font_cache.query(&FcPattern {
		family: Some(font_name.to_owned()),
		bold: PatternMatch::False,
		italic: PatternMatch::False,
		..Default::default()
	});

	if font_results.is_empty() {
		return Err(anyhow!("unable to find the specified font in the system"));
	}

	// Load the font
	let font_path = font_results[0];
	let mut font_bytes = Vec::new();
	File::open(font_path.path.as_str())
		.with_context(|| "unable to find the font on disk")?
		.read_to_end(&mut font_bytes)
		.with_context(|| "unable to read the font file")?;

	FontArc::try_from_vec(font_bytes).with_context(|| "unable to load the font")
}
