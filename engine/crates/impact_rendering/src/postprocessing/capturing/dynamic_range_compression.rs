//! Render passes for applying tone mapping.

use crate::{
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    postprocessing::Postprocessor,
    render_command::postprocessing_pass::PostprocessingRenderPass,
    resource::BasicRenderResources,
    shader_templates::dynamic_range_compression::DynamicRangeCompressionShaderTemplate,
    surface::RenderingSurface,
};
use anyhow::Result;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryRegistry, resource_group::GPUResourceGroupManager, shader::ShaderManager,
    wgpu,
};
use roc_integration::roc;
use std::{borrow::Cow, fmt::Display};

/// Configuration options for dynamic range compression.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct DynamicRangeCompressionConfig {
    /// The method to use for tone mapping.
    pub tone_mapping_method: ToneMappingMethod,
}

/// The method to use for tone mapping.
#[roc(parents = "Rendering")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToneMappingMethod {
    None,
    #[default]
    ACES,
    KhronosPBRNeutral,
}

#[derive(Debug)]
pub(super) struct DynamicRangeCompressionRenderCommands {
    no_tone_mapping_pass: PostprocessingRenderPass,
    aces_tone_mapping_pass: PostprocessingRenderPass,
    khronos_pbr_neutral_tone_mapping_pass: PostprocessingRenderPass,
    config: DynamicRangeCompressionConfig,
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

impl DynamicRangeCompressionRenderCommands {
    pub(super) fn new(
        config: DynamicRangeCompressionConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        // The last shader before dynamic range compression (the TAA shader)
        // writes to the luminance history attachment
        let input_render_attachment_quantity = RenderAttachmentQuantity::LuminanceHistory;

        let no_tone_mapping_pass = create_dynamic_range_compression_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
            input_render_attachment_quantity,
            ToneMappingMethod::None,
        )?;

        let aces_tone_mapping_pass = create_dynamic_range_compression_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
            input_render_attachment_quantity,
            ToneMappingMethod::ACES,
        )?;

        let khronos_pbr_neutral_tone_mapping_pass = create_dynamic_range_compression_render_pass(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
            input_render_attachment_quantity,
            ToneMappingMethod::KhronosPBRNeutral,
        )?;

        Ok(Self {
            config,
            no_tone_mapping_pass,
            aces_tone_mapping_pass,
            khronos_pbr_neutral_tone_mapping_pass,
        })
    }

    pub(super) fn config(&self) -> &DynamicRangeCompressionConfig {
        &self.config
    }

    pub(super) fn config_mut(&mut self) -> &mut DynamicRangeCompressionConfig {
        &mut self.config
    }

    pub(super) fn record(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &impl BasicRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        match self.config.tone_mapping_method {
            ToneMappingMethod::ACES => {
                self.aces_tone_mapping_pass.record(
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
                self.khronos_pbr_neutral_tone_mapping_pass.record(
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
                self.no_tone_mapping_pass.record(
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
/// as well as gamma correction to the input attachment and writes the result to
/// the surface attachment.
fn create_dynamic_range_compression_render_pass(
    graphics_device: &GraphicsDevice,
    rendering_surface: &RenderingSurface,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    tone_mapping_method: ToneMappingMethod,
) -> Result<PostprocessingRenderPass> {
    let shader_template = DynamicRangeCompressionShaderTemplate::new(
        input_render_attachment_quantity,
        tone_mapping_method,
    );

    PostprocessingRenderPass::new(
        graphics_device,
        rendering_surface,
        shader_manager,
        render_attachment_texture_manager,
        gpu_resource_group_manager,
        bind_group_layout_registry,
        &shader_template,
        Cow::Owned(format!(
            "Dynamic range compression pass ({tone_mapping_method})"
        )),
    )
}
