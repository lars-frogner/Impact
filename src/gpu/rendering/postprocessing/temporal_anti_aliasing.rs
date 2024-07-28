//! Render passes for applying temporal anti-aliasing.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use nalgebra::Vector4;

use crate::{
    assert_uniform_valid,
    gpu::{
        push_constant::{PushConstant, PushConstantVariant},
        rendering::{
            fre,
            render_command::{
                Blending, DepthMapUsage, RenderCommandSpecification, RenderPassHints,
                RenderPassSpecification,
            },
        },
        resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
        shader::{template::SpecificShaderTemplate, ShaderManager},
        texture::attachment::{
            OutputAttachmentSampling, RenderAttachmentInputDescription,
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescription,
            RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
            RenderAttachmentQuantitySet, RenderAttachmentSampler,
        },
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
    mesh::{buffer::VertexBufferable, VertexPosition, SCREEN_FILLING_QUAD_MESH_ID},
};

/// Configuration options for temporal anti-aliasing.
#[derive(Clone, Debug)]
pub struct TemporalAntiAliasingConfig {
    /// Whether temporal anti-aliasing should be enabled when the scene loads.
    pub initially_enabled: bool,
    /// How much the luminance of the current frame should be weighted compared
    /// to the luminance reprojected from the previous frame.
    pub current_frame_weight: fre,
}

/// Uniform holding parameters needed in the shader for applying temporal
/// anti-aliasing.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct TemporalAntiAliasingParameters {
    current_frame_weight: Vector4<fre>,
}

impl Default for TemporalAntiAliasingConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            current_frame_weight: 0.1,
        }
    }
}

impl TemporalAntiAliasingParameters {
    fn new(current_frame_weight: fre) -> Self {
        Self {
            current_frame_weight: Vector4::new(current_frame_weight, 0.0, 0.0, 0.0),
        }
    }
}

impl UniformBufferable for TemporalAntiAliasingParameters {
    const ID: ConstStringHash64 = ConstStringHash64::new("Temporal anti-aliasing parameters");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(TemporalAntiAliasingParameters);

pub(super) fn create_temporal_anti_aliasing_render_commands(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    config: &TemporalAntiAliasingConfig,
) -> Vec<RenderCommandSpecification> {
    vec![
        create_temporal_anti_aliasing_render_prepass(graphics_device, shader_manager),
        create_temporal_anti_aliasing_render_pass(
            graphics_device,
            shader_manager,
            gpu_resource_group_manager,
            config.current_frame_weight,
        ),
    ]
}

fn create_temporal_anti_aliasing_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    current_frame_weight: fre,
) -> RenderCommandSpecification {
    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "TemporalAntiAliasingParameters{{ current_frame_weight: {} }}",
        current_frame_weight
    )));

    gpu_resource_group_manager
        .resource_group_entry(resource_group_id)
        .or_insert_with(|| {
            let parameter_uniform = TemporalAntiAliasingParameters::new(current_frame_weight);

            let parameter_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
                graphics_device,
                &parameter_uniform,
                wgpu::ShaderStages::FRAGMENT,
                Cow::Borrowed("Temporal anti-aliasing parameters"),
            );

            GPUResourceGroup::new(
                graphics_device,
                vec![parameter_uniform_buffer],
                &[],
                &[],
                &[],
                wgpu::ShaderStages::FRAGMENT,
                "Temporal anti-aliasing resources",
            )
        });

    let (linear_depth_texture_binding, linear_depth_sampler_binding) =
        RenderAttachmentQuantity::LinearDepth.bindings();
    let (previous_linear_depth_texture_binding, previous_linear_depth_sampler_binding) =
        RenderAttachmentQuantity::PreviousLinearDepth.bindings();
    let (luminance_texture_binding, luminance_sampler_binding) =
        RenderAttachmentQuantity::Luminance.bindings();
    let (previous_luminance_texture_binding, previous_luminance_sampler_binding) =
        RenderAttachmentQuantity::PreviousLuminanceAux.bindings();
    let (motion_vector_texture_binding, motion_vector_sampler_binding) =
        RenderAttachmentQuantity::MotionVector.bindings();
    let (previous_motion_vector_texture_binding, previous_motion_vector_sampler_binding) =
        RenderAttachmentQuantity::PreviousMotionVector.bindings();

    let shader_id = shader_manager
        .get_or_create_rendering_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::TemporalAntiAliasing,
            &[
                (
                    "position_location",
                    VertexPosition::BINDING_LOCATION.to_string(),
                ),
                ("linear_depth_texture_group", "0".to_string()),
                (
                    "linear_depth_texture_binding",
                    linear_depth_texture_binding.to_string(),
                ),
                (
                    "linear_depth_sampler_binding",
                    linear_depth_sampler_binding.to_string(),
                ),
                ("previous_linear_depth_texture_group", "1".to_string()),
                (
                    "previous_linear_depth_texture_binding",
                    previous_linear_depth_texture_binding.to_string(),
                ),
                (
                    "previous_linear_depth_sampler_binding",
                    previous_linear_depth_sampler_binding.to_string(),
                ),
                ("luminance_texture_group", "2".to_string()),
                (
                    "luminance_texture_binding",
                    luminance_texture_binding.to_string(),
                ),
                (
                    "luminance_sampler_binding",
                    luminance_sampler_binding.to_string(),
                ),
                ("previous_luminance_texture_group", "3".to_string()),
                (
                    "previous_luminance_texture_binding",
                    previous_luminance_texture_binding.to_string(),
                ),
                (
                    "previous_luminance_sampler_binding",
                    previous_luminance_sampler_binding.to_string(),
                ),
                ("motion_vector_texture_group", "4".to_string()),
                (
                    "motion_vector_texture_binding",
                    motion_vector_texture_binding.to_string(),
                ),
                (
                    "motion_vector_sampler_binding",
                    motion_vector_sampler_binding.to_string(),
                ),
                ("previous_motion_vector_texture_group", "5".to_string()),
                (
                    "previous_motion_vector_texture_binding",
                    previous_motion_vector_texture_binding.to_string(),
                ),
                (
                    "previous_motion_vector_sampler_binding",
                    previous_motion_vector_sampler_binding.to_string(),
                ),
                ("params_group", "6".to_string()),
                ("params_binding", "0".to_string()),
            ],
        )
        .unwrap();

    let mut input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
        RenderAttachmentQuantitySet::LINEAR_DEPTH
            | RenderAttachmentQuantitySet::LUMINANCE
            | RenderAttachmentQuantitySet::MOTION_VECTOR,
    );

    input_render_attachments.insert_description(
        RenderAttachmentQuantity::PreviousLinearDepth,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );
    input_render_attachments.insert_description(
        RenderAttachmentQuantity::PreviousLuminanceAux,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );
    input_render_attachments.insert_description(
        RenderAttachmentQuantity::PreviousMotionVector,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );

    let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
        RenderAttachmentQuantity::LuminanceAux,
        RenderAttachmentOutputDescription::default()
            .with_sampling(OutputAttachmentSampling::Single),
    );

    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_shader_id: Some(shader_id),
        resource_group_id: Some(resource_group_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        input_render_attachments,
        output_render_attachments,
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        label: format!(
            "Temporal anti-aliasing pass (current frame weight: {})",
            current_frame_weight
        ),
        ..Default::default()
    })
}

fn create_temporal_anti_aliasing_render_prepass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
) -> RenderCommandSpecification {
    super::create_passthrough_render_pass(
        graphics_device,
        shader_manager,
        RenderAttachmentQuantity::Luminance,
        RenderAttachmentQuantity::LuminanceAux,
        OutputAttachmentSampling::Single,
        Blending::Replace,
        DepthMapUsage::None,
    )
}
