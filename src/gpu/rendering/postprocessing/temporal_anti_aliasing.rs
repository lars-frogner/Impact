//! Render passes for applying temporal anti-aliasing.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};

use crate::{
    assert_uniform_valid,
    gpu::{
        query::TimestampQueryRegistry,
        rendering::{
            fre,
            postprocessing::Postprocessor,
            render_command::{PostprocessingRenderPass, RenderAttachmentTextureCopyCommand},
            resource::SynchronizedRenderResources,
            surface::RenderingSurface,
        },
        resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
        shader::{
            template::temporal_anti_aliasing::TemporalAntiAliasingShaderTemplate, ShaderManager,
        },
        texture::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
};
use anyhow::Result;

/// Configuration options for temporal anti-aliasing.
#[derive(Clone, Debug)]
pub struct TemporalAntiAliasingConfig {
    /// Whether temporal anti-aliasing should be enabled when the scene loads.
    pub initially_enabled: bool,
    /// How much the luminance of the current frame should be weighted compared
    /// to the luminance reprojected from the previous frame.
    pub current_frame_weight: fre,
    pub variance_clipping_threshold: fre,
}

#[derive(Debug)]
pub(super) struct TemporalAntiAliasingRenderCommands {
    copy_command: RenderAttachmentTextureCopyCommand,
    blending_pass: PostprocessingRenderPass,
}

/// Uniform holding parameters needed in the shader for applying temporal
/// anti-aliasing.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct TemporalAntiAliasingParameters {
    current_frame_weight: fre,
    variance_clipping_threshold: fre,
    _pad: [u8; 8],
}

impl Default for TemporalAntiAliasingConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            current_frame_weight: 0.1,
            variance_clipping_threshold: 1.0,
        }
    }
}

impl TemporalAntiAliasingRenderCommands {
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        config: &TemporalAntiAliasingConfig,
    ) -> Result<Self> {
        let copy_command = create_temporal_anti_aliasing_texture_copy_command();

        let blending_pass = create_temporal_anti_aliasing_blending_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            config.current_frame_weight,
            config.variance_clipping_threshold,
        )?;

        Ok(Self {
            copy_command,
            blending_pass,
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
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        enabled: bool,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.copy_command
            .record(render_attachment_texture_manager, command_encoder);

        if enabled {
            self.blending_pass.record(
                rendering_surface,
                surface_texture_view,
                render_resources,
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

impl TemporalAntiAliasingParameters {
    fn new(current_frame_weight: fre, variance_clipping_threshold: fre) -> Self {
        Self {
            current_frame_weight,
            variance_clipping_threshold,
            _pad: [0; 8],
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

/// Creates a [`RenderAttachmentTextureCopyCommand`] that copies the luminance
/// attachment to the auxiliary luminance attachment.
fn create_temporal_anti_aliasing_texture_copy_command() -> RenderAttachmentTextureCopyCommand {
    RenderAttachmentTextureCopyCommand::new(
        RenderAttachmentQuantity::Luminance,
        RenderAttachmentQuantity::LuminanceAux,
    )
}

/// Creates a [`PostprocessingRenderPass`] that applies temporal anti-aliasing
/// by blending the previous auxiliary luminance with the current luminance,
/// writing the result to the auxiliary luminance attachment.
fn create_temporal_anti_aliasing_blending_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    current_frame_weight: fre,
    variance_clipping_threshold: fre,
) -> Result<PostprocessingRenderPass> {
    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "TemporalAntiAliasingParameters{{ current_frame_weight: {} }}",
        current_frame_weight
    )));

    gpu_resource_group_manager
        .resource_group_entry(resource_group_id)
        .or_insert_with(|| {
            let parameter_uniform = TemporalAntiAliasingParameters::new(
                current_frame_weight,
                variance_clipping_threshold,
            );

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

    let shader_template = TemporalAntiAliasingShaderTemplate::new(resource_group_id);

    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        &shader_template,
        Cow::Borrowed("Temporal anti-aliasing blend pass"),
    )
}
