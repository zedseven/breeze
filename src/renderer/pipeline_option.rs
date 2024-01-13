// Uses
use gfx::pso::{DataBind, DataLink, ElementError, RawDataSet};
use gfx_core::{
	command::AccessInfo,
	format::Format,
	handle::Manager,
	pso::{
		AttributeDesc,
		BufferIndex,
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
	Resources,
};

/// A wrapper type to allow optional values in data pipelines.
///
/// Effectively just implements [`DataBind`] and [`DataLink`] on an [`Option`].
///
/// Panics if a `None` value is actually attempted to be used.
///
/// The utility of this is in allowing a value to be `None` until it's actually
/// required.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PipelineOption<T>(Option<T>);

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
