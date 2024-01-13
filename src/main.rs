#![feature(slice_as_chunks)]
//! A tool for running presentations without fluff. Effectively a spiritual fork
//! of the `suckless` tool, `sent`.

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
use std::{collections::HashMap, env::args, path::PathBuf};

use anyhow::{anyhow, Context, Result as AnyhowResult};
pub use gfx; // Required by `gfx_defines`
use gfx::{
	format::{Depth, Srgba8},
	gfx_defines,
	gfx_impl_struct_meta,
	gfx_pipeline,
	gfx_pipeline_inner,
	gfx_vertex_struct_meta,
	handle::Manager,
	pso::{buffer::BufferIndex, AccessInfo, DataBind, DataLink, ElementError, RawDataSet},
	texture::{AaMode, Kind, Mipmap},
	traits::FactoryExt,
	Device,
	Encoder,
	Factory,
	RenderTarget,
	Resources,
	TextureSampler,
	VertexBuffer,
};
use gfx_core::{
	format::Format,
	pso::{
		AttributeDesc,
		ColorTargetDesc,
		ConstantBufferDesc,
		DepthStencilDesc,
		ResourceViewDesc,
		SamplerDesc,
		UnorderedViewDesc,
		VertexBufferDesc,
	},
	shade::{
		AttributeVar,
		CompatibilityError,
		ConstVar,
		ConstantBufferVar,
		OutputVar,
		SamplerVar,
		TextureVar,
		UnorderedVar,
	},
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
use image::{io::Reader as ImageReader, DynamicImage};
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

use crate::sent::{Presentation, Slide};

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

// Type Definitions
type ColourFormat = Srgba8;
type DepthFormat = Depth;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PipelineOption<T>(Option<T>);

impl<T> PipelineOption<T> {
	const PANIC_MESSAGE: &'static str =
		"`None` value attempted to be bound to the rendering pipeline";

	fn unwrap_as_ref(&self) -> &T {
		match &self.0 {
			Some(inner) => inner,
			None => panic!("{}", Self::PANIC_MESSAGE),
		}
	}

	fn unwrap_as_ref_mut(&mut self) -> &mut T {
		match &mut self.0 {
			Some(inner) => inner,
			None => panic!("{}", Self::PANIC_MESSAGE),
		}
	}
}

impl<T, R> DataBind<R> for PipelineOption<T>
where
	T: DataBind<R>,
	R: Resources,
{
	type Data = Option<T::Data>;

	fn bind_to(
		&self,
		raw_data_set: &mut RawDataSet<R>,
		data: &Self::Data,
		manager: &mut Manager<R>,
		access_info: &mut AccessInfo<R>,
	) {
		let Some(unwrapped_data) = data else {
			panic!("{}", Self::PANIC_MESSAGE)
		};

		self.unwrap_as_ref()
			.bind_to(raw_data_set, unwrapped_data, manager, access_info);
	}
}

impl<'a, T> DataLink<'a> for PipelineOption<T>
where
	T: DataLink<'a>,
{
	type Init = T::Init;

	fn new() -> Self {
		Self(Some(T::new()))
	}

	fn is_active(&self) -> bool {
		self.unwrap_as_ref().is_active()
	}

	fn link_vertex_buffer(
		&mut self,
		buffer_index: BufferIndex,
		init: &Self::Init,
	) -> Option<VertexBufferDesc> {
		self.unwrap_as_ref_mut()
			.link_vertex_buffer(buffer_index, init)
	}

	fn link_input(
		&mut self,
		attribute_var: &AttributeVar,
		init: &Self::Init,
	) -> Option<Result<AttributeDesc, Format>> {
		self.unwrap_as_ref_mut().link_input(attribute_var, init)
	}

	fn link_constant_buffer<'b>(
		&mut self,
		constant_buffer_var: &'b ConstantBufferVar,
		init: &Self::Init,
	) -> Option<Result<ConstantBufferDesc, ElementError<&'b str>>> {
		self.unwrap_as_ref_mut()
			.link_constant_buffer(constant_buffer_var, init)
	}

	fn link_global_constant(
		&mut self,
		const_var: &ConstVar,
		init: &Self::Init,
	) -> Option<Result<(), CompatibilityError>> {
		self.unwrap_as_ref_mut()
			.link_global_constant(const_var, init)
	}

	fn link_output(
		&mut self,
		output_var: &OutputVar,
		init: &Self::Init,
	) -> Option<Result<ColorTargetDesc, Format>> {
		self.unwrap_as_ref_mut().link_output(output_var, init)
	}

	fn link_depth_stencil(&mut self, init: &Self::Init) -> Option<DepthStencilDesc> {
		self.unwrap_as_ref_mut().link_depth_stencil(init)
	}

	fn link_resource_view(
		&mut self,
		texture_var: &TextureVar,
		init: &Self::Init,
	) -> Option<Result<ResourceViewDesc, Format>> {
		self.unwrap_as_ref_mut()
			.link_resource_view(texture_var, init)
	}

	fn link_unordered_view(
		&mut self,
		unordered_var: &UnorderedVar,
		init: &Self::Init,
	) -> Option<Result<UnorderedViewDesc, Format>> {
		self.unwrap_as_ref_mut()
			.link_unordered_view(unordered_var, init)
	}

	fn link_sampler(&mut self, sampler_var: &SamplerVar, init: &Self::Init) -> Option<SamplerDesc> {
		self.unwrap_as_ref_mut().link_sampler(sampler_var, init)
	}

	fn link_scissor(&mut self) -> bool {
		self.unwrap_as_ref_mut().link_scissor()
	}
}

gfx_defines! {
	vertex Vertex {
		pos: [f32; 2] = "a_Pos",
		uv: [f32; 2] = "a_Uv",
	}

	pipeline pipe {
		vertex_buffer: VertexBuffer<Vertex> = (),
		current_texture: PipelineOption<TextureSampler<[f32; 4]>> = "t_Current",
		render_target: RenderTarget<ColourFormat> = "Target0",
	}
}

const SQUARE: (&[Vertex], &[u16]) = (
	&[
		Vertex {
			pos: [0.5, -0.5],
			uv:  [1.0, 0.0],
		},
		Vertex {
			pos: [-0.5, -0.5],
			uv:  [0.0, 0.0],
		},
		Vertex {
			pos: [-0.5, 0.5],
			uv:  [0.0, 1.0],
		},
		Vertex {
			pos: [0.5, 0.5],
			uv:  [1.0, 1.0],
		},
	],
	&[0, 1, 2, 2, 3, 0],
);

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
			.decode()
			.with_context(|| {
				format!(
					"unable to load \"{}\"",
					resolved_image_path.to_string_lossy()
				)
			})?;

		image_cache.insert(image_path, image);
	}

	// Run the presentation
	run(&presentation, image_cache)
}

fn run(
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

	let mut image_shader_cache = HashMap::new();
	for (image_path, image) in image_cache {
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
		image_shader_cache.insert(image_path, resource_view);
	}

	let pipeline = factory
		.create_pipeline_simple(
			include_bytes!("./texture_simple.vert"),
			include_bytes!("./texture_simple.frag"),
			pipe::new(),
		)
		.with_context(|| "unable to prepare the rendering pipeline for texture rendering")?;
	let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(SQUARE.0, SQUARE.1);
	let mut data = pipe::Data {
		vertex_buffer,
		current_texture: None,
		render_target: colour_view,
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

					let (width, height, ..) = data.render_target.get_dimensions();
					let (width, height) = (f32::from(width), f32::from(height));
					let (usable_width, usable_height) = (
						width * USABLE_WIDTH_PERCENTAGE,
						height * USABLE_HEIGHT_PERCENTAGE,
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
							let new_width_scale_multiplier =
								usable_width / unscaled_section_dimensions.width();
							let new_height_scale_multiplier =
								usable_height / unscaled_section_dimensions.height();
							let new_scale_multiplier =
								new_width_scale_multiplier.min(new_height_scale_multiplier);
							let new_scale = base_scale * new_scale_multiplier;

							let scaled_section_width =
								unscaled_section_dimensions.width() * new_scale_multiplier;

							// There's only one text element, so this is safe to do
							section.text[0].scale = new_scale.into();
							section.layout = Layout::default()
								.h_align(HorizontalAlign::Left)
								.v_align(VerticalAlign::Center);
							// The reason the calculations for X and Y are different is that the
							// alignment horizontally and vertically is different
							section.screen_position =
								((width - scaled_section_width) / 2.0, height / 2.0);

							// Queue the finished section
							glyph_brush.queue(&section);

							// Draw the text
							glyph_brush
								.use_queue()
								.draw(&mut encoder, &data.render_target)
								.unwrap();
						}
						Slide::Image(image_path) => {
							let resource_view = image_shader_cache[image_path].clone();
							data.current_texture =
								Some((resource_view, factory.create_sampler_linear()));
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
