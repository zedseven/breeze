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
use gfx_glyph::{ab_glyph::FontArc, GlyphBrushBuilder, Section, Text};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use old_school_gfx_glutin_ext::{window_builder as old_school_gfx_glutin_ext_window_builder, Init};
use winit::{
	event::{Event, WindowEvent},
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

use crate::sent::Presentation;

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

	let event_loop =
		EventLoop::new().with_context(|| "unable to initialise the display backend")?;
	event_loop.set_control_flow(ControlFlow::Wait);
	// TODO: Use the contents of the first slide as the title?
	let window_builder = WindowBuilder::new().with_title(file_path);

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
	let mut glyph_brush = GlyphBrushBuilder::using_font(font)
		.initial_cache_size((1024, 1024))
		.build(factory.clone());

	let mut encoder: Encoder<_, _> = factory.create_command_buffer().into();

	let font_size: f32 = 18.0;
	let mut view_size = window.inner_size();

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
					let scale = font_size * window.scale_factor() as f32;

					// The section is all the info needed for the glyph brush to render a 'section'
					// of text.
					let text = "Test\n    Lorem ipsum dolor sit amet.";

					let section = Section::default()
						.add_text(
							Text::new(text)
								.with_scale(scale)
								.with_color([0.9, 0.3, 0.3, 1.0]),
						)
						.with_bounds((width / 3.15, height));

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
