//! Render passes for computing and applying ambient occlusion.

use crate::{
    assert_uniform_valid,
    camera::buffer::CameraGPUBufferManager,
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
    num::Float,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64, HaltonSequence};
use nalgebra::Vector4;
use std::borrow::Cow;

/// The maximum number of samples that can be used for computing ambient
/// occlusion.
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 16;

/// Configuration options for ambient occlusion.
#[derive(Clone, Debug)]
pub struct AmbientOcclusionConfig {
    /// Whether ambient occlusion should be enabled when the scene loads.
    pub initially_enabled: bool,
    /// The number of samples to use for computing ambient occlusion.
    pub sample_count: u32,
    /// The sampling radius to use when computing ambient occlusion.
    pub sample_radius: fre,
    /// Factor for scaling the intensity of the ambient occlusion.
    pub intensity: fre,
    /// Factor for scaling the contrast of the ambient occlusion.
    pub contrast: fre,
}

/// Uniform holding horizontal offsets for the ambient occlusion samples. Only
/// the first `sample_count` offsets in the array will be computed. The uniform
/// also contains the ambient occlusion parameters needed in the shader.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct AmbientOcclusionSamples {
    sample_offsets: [Vector4<fre>; MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT],
    sample_count: u32,
    sample_radius: f32,
    sample_normalization: f32,
    contrast: fre,
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            sample_count: 4,
            sample_radius: 0.5,
            intensity: 1.0,
            contrast: 0.75,
        }
    }
}

impl AmbientOcclusionSamples {
    fn new(sample_count: u32, sample_radius: fre, intensity_scale: f32, contrast: f32) -> Self {
        assert_ne!(sample_count, 0);
        assert!(sample_count <= MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32);
        assert!(sample_radius > 0.0);

        let mut sample_offsets = [Vector4::zeroed(); MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT];

        for (offset, (radius_halton_sample, angle_halton_sample)) in sample_offsets
            [..(sample_count as usize)]
            .iter_mut()
            .zip(HaltonSequence::<fre>::new(2).zip(HaltonSequence::<fre>::new(3)))
        {
            // Take square root of the sampled value (which is uniformly
            // distributed between 0 and 1) to ensure uniform distribution over
            // the disk
            let radius_fraction = fre::sqrt(radius_halton_sample);
            let radius = sample_radius * radius_fraction;

            let angle = angle_halton_sample * fre::TWO_PI;
            let (sin_angle, cos_angle) = fre::sin_cos(angle);

            offset.x = radius * cos_angle;
            offset.y = radius * sin_angle;
        }

        let sample_normalization = 2.0 * intensity_scale / (fre::PI * (sample_count as fre));

        Self {
            sample_offsets,
            sample_count,
            sample_radius,
            sample_normalization,
            contrast,
        }
    }
}

impl UniformBufferable for AmbientOcclusionSamples {
    const ID: ConstStringHash64 = ConstStringHash64::new("Ambient occlusion samples");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(AmbientOcclusionSamples);

pub(super) fn create_ambient_occlusion_render_commands(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    ambient_occlusion_config: &AmbientOcclusionConfig,
) -> Vec<RenderCommandSpecification> {
    vec![
        create_unoccluded_ambient_reflected_luminance_application_render_pass(
            graphics_device,
            shader_manager,
        ),
        create_ambient_occlusion_computation_render_pass(
            graphics_device,
            shader_manager,
            gpu_resource_group_manager,
            ambient_occlusion_config.sample_count,
            ambient_occlusion_config.sample_radius,
            ambient_occlusion_config.intensity,
            ambient_occlusion_config.contrast,
        ),
        create_ambient_occlusion_application_render_pass(graphics_device, shader_manager),
    ]
}

/// Creates a [`RenderCommandSpecification`] for a render pass that computes
/// ambient occlusion and writes it to the occlusion attachment.
///
/// # Panics
/// - If the sample count is zero or exceeds
///   [`MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT`].
/// - If the sample radius does not exceed zero.
fn create_ambient_occlusion_computation_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    sample_count: u32,
    sample_radius: fre,
    intensity_scale: fre,
    contrast: fre,
) -> RenderCommandSpecification {
    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "AmbientOcclusionSamples{{ sample_count: {}, sample_radius: {} }}",
        sample_count, sample_radius,
    )));
    gpu_resource_group_manager
        .resource_group_entry(resource_group_id)
        .or_insert_with(|| {
            let sample_uniform = AmbientOcclusionSamples::new(
                sample_count,
                sample_radius,
                intensity_scale,
                contrast,
            );

            let sample_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
                graphics_device,
                &sample_uniform,
                wgpu::ShaderStages::FRAGMENT,
                Cow::Borrowed("Ambient occlusion samples"),
            );
            GPUResourceGroup::new(
                graphics_device,
                vec![sample_uniform_buffer],
                &[],
                &[],
                &[],
                wgpu::ShaderStages::FRAGMENT,
                "Ambient occlusion samples",
            )
        });

    let (linear_depth_texture_binding, linear_depth_sampler_binding) =
        RenderAttachmentQuantity::LinearDepth.bindings();
    let (normal_vector_texture_binding, normal_vector_sampler_binding) =
        RenderAttachmentQuantity::NormalVector.bindings();

    let shader_id = shader_manager
        .get_or_create_rendering_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::AmbientOcclusionComputation,
            &[
                (
                    "max_samples",
                    MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT.to_string(),
                ),
                (
                    "position_location",
                    VertexPosition::BINDING_LOCATION.to_string(),
                ),
                ("projection_uniform_group", "0".to_string()),
                (
                    "projection_uniform_binding",
                    CameraGPUBufferManager::shader_input()
                        .projection_binding
                        .to_string(),
                ),
                ("linear_depth_texture_group", "1".to_string()),
                (
                    "linear_depth_texture_binding",
                    linear_depth_texture_binding.to_string(),
                ),
                (
                    "linear_depth_sampler_binding",
                    linear_depth_sampler_binding.to_string(),
                ),
                ("normal_vector_texture_group", "2".to_string()),
                (
                    "normal_vector_texture_binding",
                    normal_vector_texture_binding.to_string(),
                ),
                (
                    "normal_vector_sampler_binding",
                    normal_vector_sampler_binding.to_string(),
                ),
                ("samples_group", "3".to_string()),
                ("samples_binding", "0".to_string()),
            ],
        )
        .unwrap();

    let mut input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
        RenderAttachmentQuantitySet::NORMAL_VECTOR,
    );
    input_render_attachments.insert_description(
        RenderAttachmentQuantity::LinearDepth,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );

    let output_render_attachments =
        RenderAttachmentOutputDescriptionSet::with_defaults(RenderAttachmentQuantitySet::OCCLUSION);

    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_shader_id: Some(shader_id),
        resource_group_id: Some(resource_group_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        input_render_attachments,
        output_render_attachments,
        push_constants: [
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            ),
            PushConstant::new(
                PushConstantVariant::FrameCounter,
                wgpu::ShaderStages::FRAGMENT,
            ),
        ]
        .into_iter()
        .collect(),
        hints: RenderPassHints::NO_DEPTH_PREPASS,
        label: "Ambient occlusion computation pass".to_string(),
        ..Default::default()
    })
}

/// Creates a [`RenderCommandSpecification`] for a render pass that combines
/// occlusion and ambient reflected luminance from their respective attachments
/// and adds the resulting occluded ambient reflected luminance to the luminance
/// attachment.
fn create_ambient_occlusion_application_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
) -> RenderCommandSpecification {
    let (linear_depth_texture_binding, linear_depth_sampler_binding) =
        RenderAttachmentQuantity::LinearDepth.bindings();
    let (ambient_reflected_luminance_texture_binding, ambient_reflected_luminance_sampler_binding) =
        RenderAttachmentQuantity::NormalVector.bindings();
    let (occlusion_texture_binding, occlusion_sampler_binding) =
        RenderAttachmentQuantity::Occlusion.bindings();

    let shader_id = shader_manager
        .get_or_create_rendering_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::AmbientOcclusionApplication,
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
                ("ambient_reflected_luminance_texture_group", "1".to_string()),
                (
                    "ambient_reflected_luminance_texture_binding",
                    ambient_reflected_luminance_texture_binding.to_string(),
                ),
                (
                    "ambient_reflected_luminance_sampler_binding",
                    ambient_reflected_luminance_sampler_binding.to_string(),
                ),
                ("occlusion_texture_group", "2".to_string()),
                (
                    "occlusion_texture_binding",
                    occlusion_texture_binding.to_string(),
                ),
                (
                    "occlusion_sampler_binding",
                    occlusion_sampler_binding.to_string(),
                ),
            ],
        )
        .unwrap();

    let mut input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
        RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
    );
    input_render_attachments.insert_description(
        RenderAttachmentQuantity::LinearDepth,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );
    input_render_attachments.insert_description(
        RenderAttachmentQuantity::Occlusion,
        RenderAttachmentInputDescription::default()
            .with_sampler(RenderAttachmentSampler::Filtering),
    );

    let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
        RenderAttachmentQuantity::Luminance,
        RenderAttachmentOutputDescription::default().with_blending(Blending::Additive),
    );

    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_shader_id: Some(shader_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        input_render_attachments,
        output_render_attachments,
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        label: "Ambient occlusion application pass".to_string(),
        ..Default::default()
    })
}

fn create_unoccluded_ambient_reflected_luminance_application_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
) -> RenderCommandSpecification {
    super::create_passthrough_render_pass(
        graphics_device,
        shader_manager,
        RenderAttachmentQuantity::AmbientReflectedLuminance,
        RenderAttachmentQuantity::Luminance,
        OutputAttachmentSampling::MultiIfAvailable,
        Blending::Additive,
        DepthMapUsage::StencilTest,
    )
}
