//! Render passes for computing and applying ambient occlusion.

use crate::{
    assert_uniform_valid,
    gpu::{
        rendering::{
            fre,
            postprocessing::Postprocessor,
            render_command::{PostprocessingRenderPass, StencilValue},
            resource::SynchronizedRenderResources,
            surface::RenderingSurface,
        },
        resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
        shader::{
            template::{
                ambient_occlusion_application::AmbientOcclusionApplicationShaderTemplate,
                ambient_occlusion_computation::AmbientOcclusionComputationShaderTemplate,
                passthrough::PassthroughShaderTemplate,
            },
            ShaderManager,
        },
        texture::attachment::{Blending, RenderAttachmentQuantity, RenderAttachmentTextureManager},
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
    num::Float,
};
use anyhow::Result;
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

#[derive(Debug)]
pub(super) struct AmbientOcclusionRenderCommands {
    computation_pass: PostprocessingRenderPass,
    application_pass: PostprocessingRenderPass,
    disabled_pass: PostprocessingRenderPass,
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

impl AmbientOcclusionRenderCommands {
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        config: &AmbientOcclusionConfig,
    ) -> Result<Self> {
        let computation_pass = create_ambient_occlusion_computation_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
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
        )?;

        let disabled_pass = create_unoccluded_ambient_reflected_luminance_application_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        Ok(Self {
            computation_pass,
            application_pass,
            disabled_pass,
        })
    }

    pub(super) fn record(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        enabled: bool,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if enabled {
            self.computation_pass.record(
                rendering_surface,
                surface_texture_view,
                render_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                command_encoder,
            )?;
            log::debug!("Recorded ambient occlusion computation pass");

            self.application_pass.record(
                rendering_surface,
                surface_texture_view,
                render_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                command_encoder,
            )?;
            log::debug!("Recorded ambient occlusion application pass");
        } else {
            self.disabled_pass.record(
                rendering_surface,
                surface_texture_view,
                render_resources,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                postprocessor,
                frame_counter,
                command_encoder,
            )?;
            log::debug!("Recorded ambient reflected luminance application pass");
        }
        Ok(())
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
    sample_count: u32,
    sample_radius: fre,
    intensity_scale: fre,
    contrast: fre,
) -> Result<PostprocessingRenderPass> {
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

    let shader_template = AmbientOcclusionComputationShaderTemplate::new(resource_group_id);

    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        &shader_template,
        Cow::Borrowed("Ambient occlusion computation"),
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
) -> Result<PostprocessingRenderPass> {
    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        &AmbientOcclusionApplicationShaderTemplate::new(),
        Cow::Borrowed("Ambient occlusion application"),
    )
}

fn create_unoccluded_ambient_reflected_luminance_application_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &GPUResourceGroupManager,
) -> Result<PostprocessingRenderPass> {
    let shader_template = PassthroughShaderTemplate::new(
        RenderAttachmentQuantity::AmbientReflectedLuminance,
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
        &shader_template,
        Cow::Borrowed("Ambient light application with occlusion"),
    )
}
