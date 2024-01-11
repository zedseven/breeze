//! A tool for running presentations. Effectively a spiritual fork of the
//! `suckless` tool, `sent`.

// Linting Rules
#![warn(
	clippy::complexity,
	clippy::correctness,
	clippy::pedantic,
	clippy::perf,
	clippy::style,
	clippy::suspicious,
	clippy::clone_on_ref_ptr,
	clippy::dbg_macro,
	clippy::decimal_literal_representation,
	clippy::exit,
	clippy::filetype_is_file,
	clippy::if_then_some_else_none,
	clippy::non_ascii_literal,
	clippy::self_named_module_files,
	clippy::str_to_string,
	clippy::undocumented_unsafe_blocks,
	clippy::wildcard_enum_match_arm
)]
#![allow(
	clippy::cast_possible_truncation,
	clippy::cast_possible_wrap,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::doc_markdown,
	clippy::module_name_repetitions,
	clippy::similar_names,
	clippy::too_many_lines,
	clippy::unnecessary_wraps,
	dead_code,
	unused_macros
)]

// Modules
mod sent;

// Uses
use std::env::args;

use anyhow::{anyhow, Context, Result as AnyhowResult};
use gfx::{
	format::{Depth, Srgba8},
	Device,
	Encoder,
};
use gfx_glyph::{
	ab_glyph::FontArc,
	GlyphBrushBuilder,
	GlyphCruncher,
	HorizontalAlign,
	Layout,
	Section,
	Text,
	VerticalAlign,
};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use old_school_gfx_glutin_ext::{window_builder as old_school_gfx_glutin_ext_window_builder, Init};
use winit::{
	event::{Event, WindowEvent},
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

use crate::sent::Presentation;

// Constants
/// Doesn't really matter, but we need something to start with before scaling to
/// fit the space.
const BASE_FONT_SIZE: f32 = 18.0;
const USABLE_WIDTH_PERCENTAGE: f32 = 0.75;
const USABLE_HEIGHT_PERCENTAGE: f32 = 0.75;

// Entry Point
fn main() -> AnyhowResult<()> {
	// Read the file path from the command line
	let args = args().collect::<Vec<_>>();
	if args.len() != 2 {
		return Err(anyhow!("exactly one argument, the file path, is required"));
	}
	let file_path = args[1].as_str();

	// Load the presentation
	let presentation = Presentation::load_from_path(file_path)
		.with_context(|| "unable to load the presentation")?;

	dbg!(&presentation);

	// Run the presentation
	run(&presentation)
}

fn run(presentation: &Presentation) -> AnyhowResult<()> {
	let event_loop =
		EventLoop::new().with_context(|| "unable to initialise the display backend")?;
	event_loop.set_control_flow(ControlFlow::Wait);
	// TODO: Use the contents of the first slide as the title?
	let window_builder = WindowBuilder::new().with_title("TEMP_TITLE");

	// I wanted to implement the renderer initialisation myself, but the myriad ways
	// to do it without any consistency or documentation led me to just use the same
	// approach that the `glyph_brush` examples use. Perhaps this can be revisited
	// in the future.
	// https://github.com/alexheretic/glyph-brush/blob/main/gfx-glyph/examples/paragraph.rs
	let Init {
		window,
		gl_surface,
		gl_context,
		mut device,
		mut factory,
		mut color_view,
		mut depth_view,
		..
	} = old_school_gfx_glutin_ext_window_builder(&event_loop, window_builder)
		.build::<Srgba8, Depth>()
		.map_err(|error| anyhow!(error.to_string()))
		.with_context(|| "unable to build the window")?;

	let font = FontArc::try_from_slice(include_bytes!(
		"/home/zacc/typefaces/pro-fonts/PragmataPro/PragmataPro0.829/PragmataPro_Mono_R_liga_0829.\
		 ttf"
	))
	.with_context(|| "unable to load the font")?;
	let mut glyph_brush = GlyphBrushBuilder::using_font(font).build(factory.clone());

	let mut encoder: Encoder<_, _> = factory.create_command_buffer().into();

	let mut view_size = window.inner_size();
	let non_centered_layout = Layout::default()
		.h_align(HorizontalAlign::Left)
		.v_align(VerticalAlign::Top);

	#[allow(clippy::wildcard_enum_match_arm)]
	event_loop
		.run(move |event, window_target| match event {
			Event::AboutToWait => window.request_redraw(),
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::CloseRequested => window_target.exit(),
				WindowEvent::RedrawRequested => {
					// Handle resizes
					let window_size = window.inner_size();
					if view_size != window_size {
						window.resize_surface(&gl_surface, &gl_context);
						old_school_gfx_glutin_ext::resize_views(
							window_size,
							&mut color_view,
							&mut depth_view,
						);
						view_size = window_size;
					}

					encoder.clear(&color_view, [0.02, 0.02, 0.02, 1.0]);

					let (width, height, ..) = color_view.get_dimensions();
					let (width, height) = (f32::from(width), f32::from(height));
					let (usable_width, usable_height) = (
						width * USABLE_WIDTH_PERCENTAGE,
						height * USABLE_HEIGHT_PERCENTAGE,
					);
					let base_scale = BASE_FONT_SIZE * window.scale_factor() as f32;

					let text = "Test\n    Lorem ipsum dolor sit amet.";

					// Start with an unscaled, non-centered layout in the top-left corner
					let mut section = Section::default()
						.add_text(
							Text::new(text)
								.with_scale(base_scale)
								.with_color([0.9, 0.3, 0.3, 1.0]),
						)
						.with_layout(non_centered_layout)
						.with_bounds((usable_width, usable_height));

					// Get the dimensions of it with the base scale so that it can be scaled to fit
					// the usable space
					let unscaled_section_dimensions = glyph_brush
						.glyph_bounds(&section)
						.expect("the section is not empty");

					// Calculate the new scale and set the final values for the section
					let new_width_scale =
						usable_width / unscaled_section_dimensions.width() * base_scale;
					let new_height_scale =
						usable_height / unscaled_section_dimensions.height() * base_scale;
					let new_scale = new_width_scale.min(new_height_scale);

					// There's only one text element, so this is safe to do
					section.text[0].scale = new_scale.into();
					section.layout = Layout::default()
						.h_align(HorizontalAlign::Left)
						.v_align(VerticalAlign::Center);
					// The reason the calculations for X and Y are different is that the alignment
					// horizontally and vertically is different
					section.screen_position = dbg!(((width - usable_width) / 2.0, height / 2.0));

					// Queue the finished section
					glyph_brush.queue(&section);

					// Draw the text
					glyph_brush
						.use_queue()
						.draw(&mut encoder, &color_view)
						.unwrap();

					encoder.flush(&mut device);
					gl_surface.swap_buffers(&gl_context).unwrap();
					device.cleanup();
				}
				_ => {}
			},
			_ => {}
		})
		.with_context(|| "encountered an error during the event loop")
}
