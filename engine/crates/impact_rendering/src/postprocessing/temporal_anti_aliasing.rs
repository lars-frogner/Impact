//! Render passes for applying temporal anti-aliasing.

use crate::{
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    postprocessing::Postprocessor,
    render_command::{
        postprocessing_pass::PostprocessingRenderPass,
        render_attachment_texture_copy_command::RenderAttachmentTextureCopyCommand,
    },
    resource::BasicGPUResources,
    shader_templates::temporal_anti_aliasing::TemporalAntiAliasingShaderTemplate,
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
use impact_math::{hash::ConstStringHash64, hash64};
use std::borrow::Cow;

/// Configuration options for temporal anti-aliasing.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct TemporalAntiAliasingConfig {
    /// Whether temporal anti-aliasing is enabled.
    pub enabled: bool,
    /// How much the luminance of the current frame should be weighted compared
    /// to the luminance reprojected from the previous frame.
    pub current_frame_weight: f32,
    /// The maximum variance allowed between the current and previous frame's
    /// luminance when performing temporal blending.
    pub variance_clipping_threshold: f32,
}

#[derive(Debug)]
pub struct TemporalAntiAliasingRenderCommands {
    copy_command: RenderAttachmentTextureCopyCommand,
    blending_pass: PostprocessingRenderPass,
    config: TemporalAntiAliasingConfig,
}

/// Uniform holding parameters needed in the shader for applying temporal
/// anti-aliasing.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct TemporalAntiAliasingParameters {
    current_frame_weight: f32,
    variance_clipping_threshold: f32,
    _pad: [u8; 8],
}

impl TemporalAntiAliasingConfig {
    fn new_config_requires_resource_update(&self, other: &Self) -> bool {
        abs_diff_ne!(
            self.current_frame_weight,
            other.current_frame_weight,
            epsilon = 1e-6
        ) || abs_diff_ne!(
            self.variance_clipping_threshold,
            other.variance_clipping_threshold,
            epsilon = 1e-6
        )
    }
}

impl Default for TemporalAntiAliasingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            current_frame_weight: 0.1,
            variance_clipping_threshold: 1.0,
        }
    }
}

impl TemporalAntiAliasingRenderCommands {
    pub(super) fn new(
        config: TemporalAntiAliasingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let copy_command = create_temporal_anti_aliasing_texture_copy_command();

        let blending_pass = create_temporal_anti_aliasing_blending_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
            config.current_frame_weight,
            config.variance_clipping_threshold,
        )?;

        Ok(Self {
            copy_command,
            blending_pass,
            config,
        })
    }

    pub fn enabled_mut(&mut self) -> &mut bool {
        &mut self.config.enabled
    }

    pub(super) fn config(&self) -> &TemporalAntiAliasingConfig {
        &self.config
    }

    pub(super) fn set_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: TemporalAntiAliasingConfig,
    ) {
        if self.config.new_config_requires_resource_update(&config) {
            let parameters_uniform = TemporalAntiAliasingParameters::new(
                config.current_frame_weight,
                config.variance_clipping_threshold,
            );
            update_temporal_anti_aliasing_parameters_uniform(
                graphics_device,
                gpu_resource_group_manager,
                &parameters_uniform,
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
        self.copy_command
            .record(render_attachment_texture_manager, command_encoder);

        if self.config.enabled {
            self.blending_pass.record(
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

impl TemporalAntiAliasingParameters {
    fn new(current_frame_weight: f32, variance_clipping_threshold: f32) -> Self {
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

/// Creates a [`RenderAttachmentTextureCopyCommand`] that copies the auxiliary
/// luminance attachment to the luminance history attachment.
fn create_temporal_anti_aliasing_texture_copy_command() -> RenderAttachmentTextureCopyCommand {
    RenderAttachmentTextureCopyCommand::new(
        // The previous pass (bloom) writes to this attachment
        RenderAttachmentQuantity::LuminanceAux,
        RenderAttachmentQuantity::LuminanceHistory,
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
    bind_group_layout_registry: &BindGroupLayoutRegistry,
    current_frame_weight: f32,
    variance_clipping_threshold: f32,
) -> Result<PostprocessingRenderPass> {
    let resource_group_id = temporal_anti_aliasing_parameters_resource_group_id();

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
        bind_group_layout_registry,
        &shader_template,
        Cow::Borrowed("Temporal anti-aliasing blend pass"),
    )
}

fn update_temporal_anti_aliasing_parameters_uniform(
    graphics_device: &GraphicsDevice,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    uniform: &TemporalAntiAliasingParameters,
) {
    let resource_group_id = temporal_anti_aliasing_parameters_resource_group_id();
    let resource_group = gpu_resource_group_manager
        .get_resource_group(resource_group_id)
        .expect(
            "Temporal anti-aliasing parameters resource group should not be missing during update",
        );
    let buffer = resource_group.single_uniform_buffer(0).expect(
        "Temporal anti-aliasing parameters resource group should have single uniform buffer",
    );
    buffer.update_uniform(graphics_device, uniform);
}

fn temporal_anti_aliasing_parameters_resource_group_id() -> GPUResourceGroupID {
    GPUResourceGroupID(hash64!("TemporalAntiAliasingParameters"))
}
