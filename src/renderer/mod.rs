// Modules
mod pipeline_option;

// Uses
use std::collections::HashMap;

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
	Encoder,
	PipelineState,
	RenderTarget,
	TextureSampler,
	VertexBuffer,
};
use gfx_core::{
	format::Vec4,
	handle::{DepthStencilView, RenderTargetView, Sampler, ShaderResourceView},
	texture::{FilterMethod, SamplerInfo, WrapMode},
	Device as DeviceTrait,
	Factory as FactoryTrait,
};
use gfx_device_gl::{CommandBuffer, Device, Factory, Resources};
use gfx_glyph::{
	ab_glyph::FontArc,
	BuiltInLineBreaker,
	GlyphBrush,
	GlyphBrushBuilder,
	GlyphCruncher,
	HorizontalAlign,
	Layout,
	Section,
	Text,
	VerticalAlign,
};
use glutin::{
	context::PossiblyCurrentContext,
	surface::{GlSurface, Surface, WindowSurface},
};
use glutin_winit::GlWindow;
use image::{DynamicImage, GenericImageView};
use old_school_gfx_glutin_ext::{
	resize_views,
	window_builder as old_school_gfx_glutin_ext_window_builder,
	Init,
};
use winit::{
	dpi::PhysicalSize,
	event_loop::EventLoop,
	window::{Window, WindowBuilder},
};

use self::pipeline_option::PipelineOption;
use crate::{
	sent::Slide,
	DEFAULT_BACKGROUND_COLOUR,
	DEFAULT_FOREGROUND_COLOUR,
	IMAGE_SAMPLING_NEAREST_NEIGHBOUR_SCALING_FACTOR_MINIMUM,
	USABLE_HEIGHT_PERCENTAGE,
	USABLE_WIDTH_PERCENTAGE,
};

// Type Definitions
type ColourFormat = Srgba8;
type DepthFormat = Depth;

gfx_defines! {
	vertex Vertex {
		pos: [f32; 2] = "a_Pos",
		uv: [f32; 2] = "a_Uv",
	}

	pipeline image_pipeline {
		vertex_buffer: PipelineOption<VertexBuffer<Vertex>> = (),
		current_texture: PipelineOption<TextureSampler<[f32; 4]>> = "t_Current",
		render_target: RenderTarget<ColourFormat> = "Target0",
	}
}

pub struct Renderer<'a> {
	// Window Management
	window: Window,
	last_view_size: PhysicalSize<u32>,
	// Rendering Infrastructure
	gl_surface: Surface<WindowSurface>,
	gl_context: PossiblyCurrentContext,
	device: Device,
	factory: Factory,
	colour_view: RenderTargetView<Resources, ColourFormat>,
	depth_view: DepthStencilView<Resources, DepthFormat>,
	encoder: Encoder<Resources, CommandBuffer>,
	glyph_brush: GlyphBrush<Resources, Factory, FontArc>,
	image_pipeline: PipelineState<Resources, image_pipeline::Meta>,
	// Runtime State
	image_sampler_nearest_neighbour: Sampler<Resources>,
	image_sampler_anisotropic: Sampler<Resources>,
	image_texture_cache: HashMap<&'a String, CachedImageTexture>,
	image_pipeline_data: image_pipeline::Data<Resources>,
}

impl<'a> Renderer<'a> {
	pub fn new(
		event_loop: &EventLoop<()>,
		window_builder: WindowBuilder,
		font: FontArc,
		image_cache: HashMap<&'a String, DynamicImage>,
	) -> AnyhowResult<Self> {
		// I wanted to implement the renderer initialisation myself, but the myriad ways
		// to do it without any consistency or documentation led me to just use the same
		// approach that the `glyph_brush` examples use. Perhaps this can be revisited
		// in the future.
		// https://github.com/alexheretic/glyph-brush/blob/bcf31b4ea716e86f942f018a580693fa3cabc8e2/gfx-glyph/examples/paragraph.rs
		let Init {
			window,
			gl_surface,
			gl_context,
			device,
			mut factory,
			color_view: colour_view,
			depth_view,
			..
		} = old_school_gfx_glutin_ext_window_builder(event_loop, window_builder)
			.build::<ColourFormat, DepthFormat>()
			.map_err(|error| anyhow!(error.to_string()))
			.with_context(|| "unable to build the window")?;

		let encoder = factory.create_command_buffer().into();

		let glyph_brush = GlyphBrushBuilder::using_font(font).build(factory.clone());

		let image_pipeline = factory
			.create_pipeline_simple(
				include_bytes!("./texture_simple.vert"),
				include_bytes!("./texture_simple.frag"),
				image_pipeline::new(),
			)
			.with_context(|| "unable to prepare the rendering pipeline for texture rendering")?;
		let image_pipeline_data = image_pipeline::Data {
			vertex_buffer:   None,
			current_texture: None,
			render_target:   colour_view.clone(),
		};

		let image_sampler_anisotropic = factory.create_sampler(SamplerInfo::new(
			FilterMethod::Anisotropic(16),
			WrapMode::Clamp,
		));
		let image_sampler_nearest_neighbour =
			factory.create_sampler(SamplerInfo::new(FilterMethod::Scale, WrapMode::Clamp));

		let last_view_size = window.inner_size();

		let image_texture_cache = convert_image_cache_to_textures(&mut factory, image_cache)
			.with_context(|| "unable to prepare a presentation image for rendering")?;

		Ok(Self {
			window,
			last_view_size,
			gl_surface,
			gl_context,
			device,
			factory,
			colour_view,
			depth_view,
			encoder,
			glyph_brush,
			image_pipeline,
			image_sampler_nearest_neighbour,
			image_sampler_anisotropic,
			image_texture_cache,
			image_pipeline_data,
		})
	}

	pub fn render(&mut self, slide: &Slide) {
		/// Doesn't really matter, but we need something to start with before
		/// scaling to fit the space.
		///
		/// The reason it's set so small is so that no wrapping is applied to
		/// the base before scaling, since wrapping would throw off the
		/// calculations.
		///
		/// It doesn't seem like there's a way to fully disable wrapping in
		/// `glyph-brush`.
		const BASE_FONT_SIZE: f32 = 1.0;

		// Handle resizes
		let window_size = self.window.inner_size();
		if self.last_view_size != window_size {
			self.window
				.resize_surface(&self.gl_surface, &self.gl_context);
			resize_views(window_size, &mut self.colour_view, &mut self.depth_view);
			self.last_view_size = window_size;
		}

		// Clear the screen with the background colour
		self.encoder
			.clear(&self.colour_view, DEFAULT_BACKGROUND_COLOUR);

		let (screen_width, screen_height, ..) = self.colour_view.get_dimensions();
		let (screen_width, screen_height) = (f32::from(screen_width), f32::from(screen_height));
		let (usable_width, usable_height) = (
			screen_width * USABLE_WIDTH_PERCENTAGE,
			screen_height * USABLE_HEIGHT_PERCENTAGE,
		);
		let base_scale = BASE_FONT_SIZE * self.window.scale_factor() as f32;

		match slide {
			Slide::Text(text) => {
				/// Floating-point imprecision can cause text to
				/// wrap when it's not supposed to because it's
				/// ever-so-slightly larger than the bounds.
				///
				/// This value exists to account for that.
				const FLOATING_POINT_IMPRECISION_ACCOMMODATION: f32 = 0.1;
				const NON_CENTERED_LAYOUT: Layout<BuiltInLineBreaker> = Layout::Wrap {
					line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
					h_align:      HorizontalAlign::Left,
					v_align:      VerticalAlign::Top,
				};

				// Start with an unscaled, non-centered layout in the top-left corner
				let mut section = Section::default()
					.add_text(
						Text::new(text)
							.with_scale(base_scale)
							.with_color(DEFAULT_FOREGROUND_COLOUR),
					)
					.with_layout(NON_CENTERED_LAYOUT)
					.with_bounds((f32::INFINITY, f32::INFINITY));

				// Get the dimensions of it with the base scale so that it can be scaled
				// to fit the usable space
				let unscaled_section_dimensions = self
					.glyph_brush
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

				let scaled_section_width = unscaled_section_dimensions.width() * scaling_factor;

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
				section.bounds = (
					usable_width + FLOATING_POINT_IMPRECISION_ACCOMMODATION,
					usable_height,
				);

				// Queue the finished section
				self.glyph_brush.queue(&section);

				// Draw the text
				self.glyph_brush
					.use_queue()
					.draw(&mut self.encoder, &self.colour_view)
					.unwrap();
			}
			Slide::Image(image_path) => {
				const RECT_VERTEX_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

				let CachedImageTexture {
					dimensions: (image_width, image_height),
					resource_view,
				} = &self.image_texture_cache[image_path];
				let (image_width, image_height) = (*image_width as f32, *image_height as f32);

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
				let (vertex_buffer, slice) = self
					.factory
					.create_vertex_buffer_with_slice(&vertices, RECT_VERTEX_INDICES);

				let image_sampler =
					if scaling_factor >= IMAGE_SAMPLING_NEAREST_NEIGHBOUR_SCALING_FACTOR_MINIMUM {
						self.image_sampler_nearest_neighbour.clone()
					} else {
						self.image_sampler_anisotropic.clone()
					};

				self.image_pipeline_data.current_texture =
					Some((resource_view.clone(), image_sampler));
				self.image_pipeline_data.vertex_buffer = Some(vertex_buffer);

				self.encoder
					.draw(&slice, &self.image_pipeline, &self.image_pipeline_data);
			}
			Slide::Empty => {}
		}

		self.encoder.flush(&mut self.device);
		self.gl_surface.swap_buffers(&self.gl_context).unwrap();
		self.device.cleanup();
	}

	pub fn get_window(&self) -> &Window {
		&self.window
	}
}

struct CachedImageTexture {
	dimensions:    (u32, u32),
	resource_view: ShaderResourceView<Resources, Vec4<f32>>,
}

fn convert_image_cache_to_textures<'a>(
	factory: &mut Factory,
	image_cache: HashMap<&'a String, DynamicImage>,
) -> AnyhowResult<HashMap<&'a String, CachedImageTexture>> {
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
		image_texture_cache.insert(
			image_path,
			CachedImageTexture {
				dimensions: image_dimensions,
				resource_view,
			},
		);
	}

	Ok(image_texture_cache)
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
