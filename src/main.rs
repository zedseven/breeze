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
mod pipeline_option;
mod sent;

// Uses
use std::{
	collections::HashMap,
	env::args,
	path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result as AnyhowResult};
pub use gfx; // Required by `gfx_defines`
use gfx::{
	format::{Depth, Srgba8},
	gfx_defines,
	gfx_impl_struct_meta,
	gfx_pipeline,
	gfx_pipeline_inner,
	gfx_vertex_struct_meta,
	texture::{AaMode, Kind, Mipmap},
	traits::FactoryExt,
	Device,
	Encoder,
	Factory,
	RenderTarget,
	TextureSampler,
	VertexBuffer,
};
use gfx_core::texture::{FilterMethod, SamplerInfo, WrapMode};
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
use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use old_school_gfx_glutin_ext::{
	resize_views,
	window_builder as old_school_gfx_glutin_ext_window_builder,
	Init,
};
use winit::{
	event::{ElementState, Event, MouseButton, WindowEvent},
	event_loop::{ControlFlow, EventLoop},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
	window::{Window, WindowBuilder},
};

use crate::{
	pipeline_option::PipelineOption,
	sent::{Presentation, Slide},
};

// Constants
/// Doesn't really matter, but we need something to start with before scaling to
/// fit the space.
///
/// The reason it's set so small is so that no wrapping is applied to the base
/// before scaling, since wrapping would throw off the calculations.
///
/// It doesn't seem like there's a way to fully disable wrapping in
/// `glyph-brush`.
const BASE_FONT_SIZE: f32 = 1.0;
const USABLE_WIDTH_PERCENTAGE: f32 = 0.75;
const USABLE_HEIGHT_PERCENTAGE: f32 = 0.75;
const DEFAULT_BACKGROUND_COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const DEFAULT_FOREGROUND_COLOUR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_TITLE: &str = "`breeze` Presentation";
/// The minimum scaling factor at which to enable nearest-neighbour image
/// sampling.
///
/// This heuristic matches what [Emulsion] uses.
///
/// [Emulsion]: https://github.com/ArturKovacs/emulsion/blob/db5992432ca9f3e0044b967713316ce267e64837/src/widgets/picture_widget.rs#L35
const IMAGE_SAMPLING_NEAREST_NEIGHBOUR_SCALING_FACTOR_MINIMUM: f32 = 4.0;
const RECT_VERTEX_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

// Type Definitions
type ColourFormat = Srgba8;
type DepthFormat = Depth;

gfx_defines! {
	vertex Vertex {
		pos: [f32; 2] = "a_Pos",
		uv: [f32; 2] = "a_Uv",
	}

	pipeline pipe {
		vertex_buffer: PipelineOption<VertexBuffer<Vertex>> = (),
		current_texture: PipelineOption<TextureSampler<[f32; 4]>> = "t_Current",
		render_target: RenderTarget<ColourFormat> = "Target0",
	}
}

/// Converts a rect defined by coordinates in pixels to a set of vertices that
/// use normalised coordinates for rendering.
fn screen_rect_to_vertices(
	screen_width: f32,
	screen_height: f32,
	x: f32,
	y: f32,
	width: f32,
	height: f32,
) -> [Vertex; 4] {
	let transform_x = |x: f32| -> f32 { (x / screen_width) * 2.0 - 1.0 };
	let transform_y = |y: f32| -> f32 { (y / screen_height) * 2.0 - 1.0 };

	[
		// Top Right
		Vertex {
			pos: [transform_x(x + width), transform_y(y)],
			uv:  [1.0, 1.0],
		},
		// Top Left
		Vertex {
			pos: [transform_x(x), transform_y(y)],
			uv:  [0.0, 1.0],
		},
		// Bottom Left
		Vertex {
			pos: [transform_x(x), transform_y(y + height)],
			uv:  [0.0, 0.0],
		},
		// Bottom Right
		Vertex {
			pos: [transform_x(x + width), transform_y(y + height)],
			uv:  [1.0, 0.0],
		},
	]
}

fn calculate_scaling_factor(
	usable_width: f32,
	usable_height: f32,
	unscaled_width: f32,
	unscaled_height: f32,
) -> f32 {
	let width_scaling_factor = usable_width / unscaled_width;
	let height_scaling_factor = usable_height / unscaled_height;

	width_scaling_factor.min(height_scaling_factor)
}

fn load_images_from_presentation<'a>(
	presentation: &'a Presentation,
	base_path: Option<&Path>,
) -> AnyhowResult<HashMap<&'a String, DynamicImage>> {
	let mut image_cache = HashMap::new();

	for image_path in presentation.0.iter().filter_map(|slide| match slide {
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

fn run_presentation(
	presentation: &Presentation,
	image_cache: HashMap<&String, DynamicImage>,
) -> AnyhowResult<()> {
	let mut current_slide = 0;
	let window_title = presentation
		.try_get_title()
		.unwrap_or_else(|| DEFAULT_TITLE.to_owned());

	let event_loop =
		EventLoop::new().with_context(|| "unable to initialise the display backend")?;
	event_loop.set_control_flow(ControlFlow::Wait);
	let window_builder = WindowBuilder::new().with_title(window_title);

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
		color_view: colour_view,
		mut depth_view,
		..
	} = old_school_gfx_glutin_ext_window_builder(&event_loop, window_builder)
		.build::<ColourFormat, DepthFormat>()
		.map_err(|error| anyhow!(error.to_string()))
		.with_context(|| "unable to build the window")?;

	let font = FontArc::try_from_slice(include_bytes!(
		"/home/zacc/typefaces/pro-fonts/PragmataPro/PragmataPro0.829/PragmataPro_Mono_R_liga_0829.\
		 ttf"
	))
	.with_context(|| "unable to load the font")?;
	let mut glyph_brush = GlyphBrushBuilder::using_font(font).build(factory.clone());

	let mut encoder: Encoder<_, _> = factory.create_command_buffer().into();

	let mut image_texture_cache = HashMap::new();
	for (image_path, image) in image_cache {
		let image_dimensions = image.dimensions();
		let image_data = image.to_rgba8();
		let (image_width, image_height) = image_data.dimensions();
		let kind = Kind::D2(image_width as u16, image_height as u16, AaMode::Single);
		let (_, resource_view) = factory
			.create_texture_immutable::<ColourFormat>(
				kind,
				Mipmap::Provided,
				&[image_data.as_chunks::<4>().0],
			)
			.with_context(|| {
				format!("unable to prepare the image \"{image_path}\" for rendering")
			})?;
		image_texture_cache.insert(image_path, (image_dimensions, resource_view));
	}

	let pipeline = factory
		.create_pipeline_simple(
			include_bytes!("./texture_simple.vert"),
			include_bytes!("./texture_simple.frag"),
			pipe::new(),
		)
		.with_context(|| "unable to prepare the rendering pipeline for texture rendering")?;
	let image_sampler_anisotropic = factory.create_sampler(SamplerInfo::new(
		FilterMethod::Anisotropic(16),
		WrapMode::Clamp,
	));
	let image_sampler_nearest_neighbour =
		factory.create_sampler(SamplerInfo::new(FilterMethod::Scale, WrapMode::Clamp));
	let mut data = pipe::Data {
		vertex_buffer:   None,
		current_texture: None,
		render_target:   colour_view,
	};

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
						resize_views(window_size, &mut data.render_target, &mut depth_view);
						view_size = window_size;
					}

					// Clear the screen with the background colour
					encoder.clear(&data.render_target, DEFAULT_BACKGROUND_COLOUR);

					let (screen_width, screen_height, ..) = data.render_target.get_dimensions();
					let (screen_width, screen_height) =
						(f32::from(screen_width), f32::from(screen_height));
					let (usable_width, usable_height) = (
						screen_width * USABLE_WIDTH_PERCENTAGE,
						screen_height * USABLE_HEIGHT_PERCENTAGE,
					);
					let base_scale = BASE_FONT_SIZE * window.scale_factor() as f32;

					let current_slide_value = &presentation.0[current_slide];
					match current_slide_value {
						Slide::Text(text) => {
							// Start with an unscaled, non-centered layout in the top-left corner
							let mut section = Section::default()
								.add_text(
									Text::new(text)
										.with_scale(base_scale)
										.with_color(DEFAULT_FOREGROUND_COLOUR),
								)
								.with_layout(non_centered_layout)
								.with_bounds((usable_width, usable_height));

							// Get the dimensions of it with the base scale so that it can be scaled
							// to fit the usable space
							let unscaled_section_dimensions = glyph_brush
								.glyph_bounds(&section)
								.expect("the section is not empty");

							// Calculate the new scale and set the final values for the section
							let scaling_factor = calculate_scaling_factor(
								usable_width,
								usable_height,
								unscaled_section_dimensions.width(),
								unscaled_section_dimensions.height(),
							);
							let new_scale = base_scale * scaling_factor;

							let scaled_section_width =
								unscaled_section_dimensions.width() * scaling_factor;

							// There's only one text element, so this is safe to do
							section.text[0].scale = new_scale.into();
							section.layout = Layout::default()
								.h_align(HorizontalAlign::Left)
								.v_align(VerticalAlign::Center);
							// The reason the calculations for X and Y are different is that the
							// alignment horizontally and vertically is different
							section.screen_position = (
								(screen_width - scaled_section_width) / 2.0,
								screen_height / 2.0,
							);

							// Queue the finished section
							glyph_brush.queue(&section);

							// Draw the text
							glyph_brush
								.use_queue()
								.draw(&mut encoder, &data.render_target)
								.unwrap();
						}
						Slide::Image(image_path) => {
							let ((image_width, image_height), resource_view) =
								image_texture_cache[image_path].clone();
							let (image_width, image_height) =
								(image_width as f32, image_height as f32);

							let scaling_factor = calculate_scaling_factor(
								usable_width,
								usable_height,
								image_width,
								image_height,
							);

							let (scaled_width, scaled_height) =
								(image_width * scaling_factor, image_height * scaling_factor);
							let (x, y) = (
								(screen_width - scaled_width) / 2.0,
								(screen_height - scaled_height) / 2.0,
							);

							let vertices = screen_rect_to_vertices(
								screen_width,
								screen_height,
								x,
								y,
								scaled_width,
								scaled_height,
							);
							let (vertex_buffer, slice) = factory
								.create_vertex_buffer_with_slice(&vertices, RECT_VERTEX_INDICES);

							let image_sampler = if scaling_factor
								>= IMAGE_SAMPLING_NEAREST_NEIGHBOUR_SCALING_FACTOR_MINIMUM
							{
								image_sampler_nearest_neighbour.clone()
							} else {
								image_sampler_anisotropic.clone()
							};

							data.current_texture = Some((resource_view, image_sampler));
							data.vertex_buffer = Some(vertex_buffer);

							encoder.draw(&slice, &pipeline, &data);
						}
						Slide::Empty => {}
					}

					encoder.flush(&mut device);
					gl_surface.swap_buffers(&gl_context).unwrap();
					device.cleanup();
				}
				WindowEvent::MouseInput {
					state: ElementState::Pressed,
					button: MouseButton::Right | MouseButton::Back,
					..
				} => change_slide(&window, presentation, &mut current_slide, false),
				WindowEvent::MouseInput {
					state: ElementState::Pressed,
					button: MouseButton::Left | MouseButton::Forward,
					..
				} => change_slide(&window, presentation, &mut current_slide, true),
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
								change_slide(&window, presentation, &mut current_slide, false);
							}
							Key::Named(
								NamedKey::ArrowRight
								| NamedKey::ArrowDown
								| NamedKey::Enter
								| NamedKey::Space
								| NamedKey::NavigateNext,
							)
							| Key::Character("l" | "j" | "n") => {
								change_slide(&window, presentation, &mut current_slide, true);
							}
							_ => {}
						}
					}
				}
				_ => {}
			},
			_ => {}
		})
		.with_context(|| "encountered an error during the event loop")
}

fn change_slide(
	window: &Window,
	presentation: &Presentation,
	current_slide: &mut usize,
	advance: bool,
) {
	if advance {
		if *current_slide < presentation.0.len() - 1 {
			*current_slide += 1;
			window.request_redraw();
		}
	} else if *current_slide > 0 {
		*current_slide -= 1;
		window.request_redraw();
	}
}
