//! A tool for running presentations without fluff. Effectively a spiritual fork
//! of the `suckless` tool, `sent`.

// Nightly Features
#![feature(slice_as_chunks)]
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
mod fonts;
mod presentation;
mod renderer;

// Uses
use std::{
	collections::HashMap,
	env::args,
	path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result as AnyhowResult};
use image::{io::Reader as ImageReader, DynamicImage};
use winit::{
	event::{ElementState, Event, MouseButton, WindowEvent},
	event_loop::{ControlFlow, EventLoop},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
	window::{Fullscreen, Window, WindowBuilder},
};

use self::{
	fonts::load_font,
	presentation::{Presentation, Slide},
	renderer::Renderer,
};

// Constants
const USABLE_WIDTH_PERCENTAGE: f32 = 0.75;
const USABLE_HEIGHT_PERCENTAGE: f32 = 0.75;
const DEFAULT_FOREGROUND_COLOUR: LinearRgbaColour = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_BACKGROUND_COLOUR: LinearRgbaColour = [0.0, 0.0, 0.0, 1.0];
/// The default search list for system fonts, searched in order from top to
/// bottom, using the first one that's found.
///
/// All of these are sans-serif typefaces.
///
/// Some of these selections are based on the following article: https://www.ctrl.blog/entry/font-stack-text.html
const DEFAULT_FONT_LIST: &[&str] = &[
	"Roboto",
	"Aptos",
	"Segoe UI",
	"Noto Sans",
	"Calibri",
	"Arial Nova",
	"Arial",
	"Helvetica Neue",
	"Helvetica",
	"Arimo",
	"Liberation Sans",
	"Nimbus Sans",
	"DejaVu Sans",
	"Ubuntu",
];
const DEFAULT_TITLE: &str = "`breeze` Presentation";
/// The minimum scaling factor at which to enable nearest-neighbour image
/// sampling.
///
/// This heuristic matches what [Emulsion] uses.
///
/// [Emulsion]: https://github.com/ArturKovacs/emulsion/blob/db5992432ca9f3e0044b967713316ce267e64837/src/widgets/picture_widget.rs#L35
const IMAGE_SAMPLING_NEAREST_NEIGHBOUR_SCALING_FACTOR_MINIMUM: f32 = 4.0;

// Type Definitions
type LinearRgbaColour = [f32; 4];

// Entry Point
fn main() -> AnyhowResult<()> {
	// Read the file path from the command line
	let args = args().collect::<Vec<_>>();
	if args.len() != 2 {
		return Err(anyhow!("exactly one argument, the file path, is required"));
	}
	let file_path = PathBuf::from(&args[1]);

	// Load the presentation
	let presentation = Presentation::load_from_path(file_path.clone())
		.with_context(|| "unable to load the presentation")?;

	// Load all images into memory
	let base_path = file_path.parent();
	let image_cache = load_images_from_presentation(&presentation, base_path)
		.with_context(|| "unable to load a presentation image")?;

	// Run the presentation
	run_presentation(&presentation, image_cache)
}

fn load_images_from_presentation<'a>(
	presentation: &'a Presentation,
	base_path: Option<&Path>,
) -> AnyhowResult<HashMap<&'a String, DynamicImage>> {
	let mut image_cache = HashMap::new();

	for image_path in presentation.slides.iter().filter_map(|slide| match slide {
		Slide::Image(image_path) => Some(image_path),
		Slide::Text(_) | Slide::Empty => None,
	}) {
		// Resolve the image path relative to the presentation file
		let resolved_image_path = if let Some(base_path) = base_path {
			base_path.to_owned().join(image_path)
		} else {
			PathBuf::from(image_path)
		};

		// Load the image into memory
		let image = ImageReader::open(resolved_image_path.as_path())
			.with_context(|| {
				format!(
					"unable to open \"{}\"",
					resolved_image_path.to_string_lossy()
				)
			})?
			.with_guessed_format()
			.with_context(|| {
				format!(
					"unable to guess the format of \"{}\"",
					resolved_image_path.to_string_lossy()
				)
			})?
			.decode()
			.with_context(|| {
				format!(
					"unable to load \"{}\"",
					resolved_image_path.to_string_lossy()
				)
			})?;

		image_cache.insert(image_path, image);
	}

	Ok(image_cache)
}

fn run_presentation(
	presentation: &Presentation,
	image_cache: HashMap<&String, DynamicImage>,
) -> AnyhowResult<()> {
	let window_title = presentation
		.try_get_title()
		.unwrap_or_else(|| DEFAULT_TITLE.to_owned());

	// Load the font to use for rendering text
	// The user font list is extended with the default list so that there's a
	// fallback in case none of the user fonts can be found
	let mut font_list = presentation
		.font_list
		.iter()
		.map(String::as_str)
		.collect::<Vec<_>>();
	font_list.extend_from_slice(DEFAULT_FONT_LIST);
	let font = load_font(font_list.as_slice())
		.with_context(|| "unable to find & load any font in the list")?;

	// Prepare the colours to use
	let foreground_colour = presentation
		.foreground_colour
		.unwrap_or(DEFAULT_FOREGROUND_COLOUR);
	let background_colour = presentation
		.background_colour
		.unwrap_or(DEFAULT_BACKGROUND_COLOUR);

	// Initialise the event loop and renderer
	let event_loop =
		EventLoop::new().with_context(|| "unable to initialise the display backend")?;
	event_loop.set_control_flow(ControlFlow::Wait);
	let window_builder = WindowBuilder::new()
		.with_title(window_title)
		.with_fullscreen(Some(Fullscreen::Borderless(None)));

	let mut renderer = Renderer::new(
		&event_loop,
		window_builder,
		font,
		foreground_colour,
		background_colour,
		image_cache,
	)
	.with_context(|| "unable to initialise the renderer")?;

	// Runtime State
	let mut current_slide = 0;

	#[allow(clippy::wildcard_enum_match_arm, clippy::single_match)]
	event_loop
		.run(move |event, window_target| {
			let window = renderer.get_window();

			match event {
				Event::WindowEvent { event, .. } => match event {
					WindowEvent::CloseRequested => window_target.exit(),
					WindowEvent::RedrawRequested => {
						renderer.render(&presentation.slides[current_slide]);
					}
					WindowEvent::MouseInput {
						state: ElementState::Pressed,
						button: MouseButton::Right | MouseButton::Back,
						..
					} => change_slides(window, presentation, &mut current_slide, false),
					WindowEvent::MouseInput {
						state: ElementState::Pressed,
						button: MouseButton::Left | MouseButton::Forward,
						..
					} => change_slides(window, presentation, &mut current_slide, true),
					WindowEvent::KeyboardInput { event, .. } => {
						if event.state == ElementState::Pressed && !event.repeat {
							// TODO: Functionality to reload the presentation
							match event.key_without_modifiers().as_ref() {
								Key::Named(NamedKey::Escape) | Key::Character("q") => {
									window_target.exit();
								}
								Key::Named(
									NamedKey::ArrowLeft
									| NamedKey::ArrowUp
									| NamedKey::Backspace
									| NamedKey::NavigatePrevious,
								)
								| Key::Character("h" | "k" | "p") => {
									change_slides(window, presentation, &mut current_slide, false);
								}
								Key::Named(
									NamedKey::ArrowRight
									| NamedKey::ArrowDown
									| NamedKey::Enter
									| NamedKey::Space
									| NamedKey::NavigateNext,
								)
								| Key::Character("l" | "j" | "n") => {
									change_slides(window, presentation, &mut current_slide, true);
								}
								_ => {}
							}
						}
					}
					_ => {}
				},
				_ => {}
			}
		})
		.with_context(|| "encountered an error during the event loop")
}

fn change_slides(
	window: &Window,
	presentation: &Presentation,
	current_slide: &mut usize,
	forward: bool,
) {
	if forward {
		if *current_slide < presentation.slides.len() - 1 {
			*current_slide += 1;
			window.request_redraw();
		}
	} else if *current_slide > 0 {
		*current_slide -= 1;
		window.request_redraw();
	}
}
