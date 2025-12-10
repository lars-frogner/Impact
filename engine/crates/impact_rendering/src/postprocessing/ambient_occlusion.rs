//! Render passes for computing and applying ambient occlusion.

use crate::{
    attachment::{Blending, RenderAttachmentQuantity, RenderAttachmentTextureManager},
    postprocessing::Postprocessor,
    render_command::{StencilValue, postprocessing_pass::PostprocessingRenderPass},
    resource::BasicGPUResources,
    shader_templates::{
        ambient_occlusion_application::AmbientOcclusionApplicationShaderTemplate,
        ambient_occlusion_computation::AmbientOcclusionComputationShaderTemplate,
        passthrough::PassthroughShaderTemplate,
    },
    surface::RenderingSurface,
};
use anyhow::Result;
use approx::abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_gpu::{
    assert_uniform_valid,
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
    shader::ShaderManager,
    timestamp_query::TimestampQueryRegistry,
    uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
    wgpu,
};
use impact_math::{Float, halton::HaltonSequence, hash::ConstStringHash64, hash64};
use nalgebra::Vector4;
use std::borrow::Cow;

/// The maximum number of samples that can be used for computing ambient
/// occlusion.
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 16;

/// Configuration options for ambient occlusion.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct AmbientOcclusionConfig {
    /// Whether ambient occlusion is enabled.
    pub enabled: bool,
    /// The number of samples to use for computing ambient occlusion.
    pub sample_count: u32,
    /// The sampling radius to use when computing ambient occlusion.
    pub sample_radius: f32,
    /// Factor for scaling the intensity of the ambient occlusion.
    pub intensity: f32,
    /// Factor for scaling the contrast of the ambient occlusion.
    pub contrast: f32,
}

#[derive(Debug)]
pub struct AmbientOcclusionRenderCommands {
    computation_pass: PostprocessingRenderPass,
    application_pass: PostprocessingRenderPass,
    disabled_pass: PostprocessingRenderPass,
    config: AmbientOcclusionConfig,
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
    sample_offsets: [Vector4<f32>; MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT],
    sample_count: u32,
    sample_radius: f32,
    sample_normalization: f32,
    contrast: f32,
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_count: 4,
            sample_radius: 1.0,
            intensity: 2.0,
            contrast: 0.75,
        }
    }
}

impl AmbientOcclusionRenderCommands {
    pub(super) fn new(
        config: AmbientOcclusionConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let computation_pass = create_ambient_occlusion_computation_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
            config.sample_count,
            config.sample_radius,
            config.intensity,
            config.contrast,
        )?;

        let application_pass = create_ambient_occlusion_application_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
        )?;

        let disabled_pass = create_unoccluded_ambient_reflected_luminance_application_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
        )?;

        Ok(Self {
            computation_pass,
            application_pass,
            disabled_pass,
            config,
        })
    }

    pub fn enabled_mut(&mut self) -> &mut bool {
        &mut self.config.enabled
    }

    pub(super) fn config(&self) -> &AmbientOcclusionConfig {
        &self.config
    }

    pub(super) fn set_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: AmbientOcclusionConfig,
    ) {
        if self.config.new_config_requires_resource_update(&config) {
            let sample_uniform = AmbientOcclusionSamples::new(
                config.sample_count,
                config.sample_radius,
                config.intensity,
                config.contrast,
            );
            update_ambient_occlusion_samples_uniform(
                graphics_device,
                gpu_resource_group_manager,
                &sample_uniform,
            );
        }
        self.config = config;
    }

    pub(super) fn record(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.config.enabled {
            self.computation_pass.record(
                rendering_surface,
                None,
                gpu_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                timestamp_recorder,
                command_encoder,
            )?;

            self.application_pass.record(
                rendering_surface,
                None,
                gpu_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                timestamp_recorder,
                command_encoder,
            )?;
        } else {
            self.disabled_pass.record(
                rendering_surface,
                None,
                gpu_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                timestamp_recorder,
                command_encoder,
            )?;
        }
        Ok(())
    }
}

impl AmbientOcclusionSamples {
    fn new(sample_count: u32, sample_radius: f32, intensity_scale: f32, contrast: f32) -> Self {
        assert_ne!(sample_count, 0);
        assert!(sample_count <= MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32);
        assert!(sample_radius > 0.0);

        let mut sample_offsets = [Vector4::zeroed(); MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT];

        for (offset, (radius_halton_sample, angle_halton_sample)) in sample_offsets
            [..(sample_count as usize)]
            .iter_mut()
            .zip(HaltonSequence::<f32>::new(2).zip(HaltonSequence::<f32>::new(3)))
        {
            // Take square root of the sampled value (which is uniformly
            // distributed between 0 and 1) to ensure uniform distribution over
            // the disk
            let radius_fraction = f32::sqrt(radius_halton_sample);
            let radius = sample_radius * radius_fraction;

            let angle = angle_halton_sample * f32::TWO_PI;
            let (sin_angle, cos_angle) = f32::sin_cos(angle);

            offset.x = radius * cos_angle;
            offset.y = radius * sin_angle;
        }

        let sample_normalization = 2.0 * intensity_scale / (f32::PI * (sample_count as f32));

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

impl AmbientOcclusionConfig {
    fn new_config_requires_resource_update(&self, other: &Self) -> bool {
        self.sample_count != other.sample_count
            || abs_diff_ne!(self.sample_radius, other.sample_radius, epsilon = 1e-6)
            || abs_diff_ne!(self.intensity, other.intensity, epsilon = 1e-6)
            || abs_diff_ne!(self.contrast, other.contrast, epsilon = 1e-6)
    }
}

/// Creates a [`PostprocessingRenderPass`] that computes ambient occlusion and
/// writes it to the occlusion attachment.
///
/// # Panics
/// - If the sample count is zero or exceeds
///   [`MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT`].
/// - If the sample radius does not exceed zero.
fn create_ambient_occlusion_computation_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
    sample_count: u32,
    sample_radius: f32,
    intensity_scale: f32,
    contrast: f32,
) -> Result<PostprocessingRenderPass> {
    let resource_group_id = ambient_occlusion_samples_resource_group_id();
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

    let shader_template = AmbientOcclusionComputationShaderTemplate::new(resource_group_id);

    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        bind_group_layout_registry,
        &shader_template,
        Cow::Borrowed("Ambient occlusion computation pass"),
    )
}

/// Creates a [`PostprocessingRenderPass`] that combines occlusion and ambient
/// reflected luminance from their respective attachments and adds the resulting
/// occluded ambient reflected luminance to the luminance attachment.
fn create_ambient_occlusion_application_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
) -> Result<PostprocessingRenderPass> {
    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        bind_group_layout_registry,
        &AmbientOcclusionApplicationShaderTemplate::new(),
        Cow::Borrowed("Ambient occlusion application pass"),
    )
}

fn create_unoccluded_ambient_reflected_luminance_application_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
) -> Result<PostprocessingRenderPass> {
    let shader_template = PassthroughShaderTemplate::new(
        RenderAttachmentQuantity::LuminanceAux,
        RenderAttachmentQuantity::Luminance,
        Blending::Additive,
        Some((wgpu::CompareFunction::Equal, StencilValue::PhysicalModel)),
    );
    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        bind_group_layout_registry,
        &shader_template,
        Cow::Borrowed("Ambient light application pass without occlusion"),
    )
}

fn update_ambient_occlusion_samples_uniform(
    graphics_device: &GraphicsDevice,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    uniform: &AmbientOcclusionSamples,
) {
    let resource_group_id = ambient_occlusion_samples_resource_group_id();
    let resource_group = gpu_resource_group_manager
        .get_resource_group(resource_group_id)
        .expect("Ambient occlusion samples resource group should not be missing during update");
    let buffer = resource_group
        .single_uniform_buffer(0)
        .expect("Ambient occlusion samples resource group should have single uniform buffer");
    buffer.update_uniform(graphics_device, uniform);
}

fn ambient_occlusion_samples_resource_group_id() -> GPUResourceGroupID {
    GPUResourceGroupID(hash64!("AmbientOcclusionSamples"))
}
