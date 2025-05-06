//! Render passes for applying tone mapping.

use crate::gpu::{
    GraphicsDevice,
    query::TimestampQueryRegistry,
    rendering::{
        postprocessing::Postprocessor, render_command::PostprocessingRenderPass,
        resource::SynchronizedRenderResources, surface::RenderingSurface,
    },
    resource_group::GPUResourceGroupManager,
    shader::{ShaderManager, template::tone_mapping::ToneMappingShaderTemplate},
    texture::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
};
use anyhow::Result;
use roc_codegen::roc;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

/// The method to use for tone mapping.
#[roc(prefix = "Engine")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToneMappingMethod {
    None,
    #[default]
    ACES,
    KhronosPBRNeutral,
}

#[derive(Debug)]
pub(super) struct ToneMappingRenderCommands {
    disabled_pass: PostprocessingRenderPass,
    aces_pass: PostprocessingRenderPass,
    khronos_pbr_neutral_pass: PostprocessingRenderPass,
}

impl ToneMappingMethod {
    /// Returns all available tone mapping methods.
    pub fn all() -> [Self; 3] {
        [Self::None, Self::ACES, Self::KhronosPBRNeutral]
    }
}

impl Display for ToneMappingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "None",
                Self::ACES => "ACES",
                Self::KhronosPBRNeutral => "KhronosPBRNeutral",
            }
        )
    }
}

impl ToneMappingRenderCommands {
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
    ) -> Result<Self> {
        // The last shader before tone mapping (the TAA shader) writes
        // to the luminance history attachment
        let input_render_attachment_quantity = RenderAttachmentQuantity::LuminanceHistory;

        let disabled_pass = create_tone_mapping_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            input_render_attachment_quantity,
            ToneMappingMethod::None,
        )?;

        let aces_pass = create_tone_mapping_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            input_render_attachment_quantity,
            ToneMappingMethod::ACES,
        )?;

        let khronos_pbr_neutral_pass = create_tone_mapping_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            input_render_attachment_quantity,
            ToneMappingMethod::KhronosPBRNeutral,
        )?;

        Ok(Self {
            disabled_pass,
            aces_pass,
            khronos_pbr_neutral_pass,
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
        method: ToneMappingMethod,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        match method {
            ToneMappingMethod::ACES => {
                self.aces_pass.record(
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
            ToneMappingMethod::KhronosPBRNeutral => {
                self.khronos_pbr_neutral_pass.record(
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
            ToneMappingMethod::None => {
                self.disabled_pass.record(
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
        }
        Ok(())
    }
}

/// Creates a [`PostprocessingRenderPass`] that applies the given tone mapping
/// to the input attachment and writes the result to the surface attachment.
fn create_tone_mapping_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    method: ToneMappingMethod,
) -> Result<PostprocessingRenderPass> {
    let shader_template = ToneMappingShaderTemplate::new(input_render_attachment_quantity, method);

    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        &shader_template,
        Cow::Owned(format!("Tone mapping pass ({})", method)),
    )
}
