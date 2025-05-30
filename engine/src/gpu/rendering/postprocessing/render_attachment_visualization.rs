//! Render passes for visualizing render attachments.

use crate::gpu::{
    GraphicsDevice,
    query::TimestampQueryRegistry,
    rendering::{
        postprocessing::Postprocessor, render_command::PostprocessingRenderPass,
        resource::SynchronizedRenderResources, surface::RenderingSurface,
    },
    resource_group::GPUResourceGroupManager,
    shader::{
        ShaderManager,
        template::render_attachment_visualization::RenderAttachmentVisualizationShaderTemplate,
    },
    texture::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
};
use anyhow::{Result, anyhow};
use std::{borrow::Cow, collections::HashMap};

/// Render passes for visualizing render attachments.
#[derive(Debug)]
pub struct RenderAttachmentVisualizationPasses {
    passes: HashMap<RenderAttachmentQuantity, PostprocessingRenderPass>,
    idx_of_quantity_to_visualize: usize,
    enabled: bool,
}

impl RenderAttachmentVisualizationPasses {
    pub const SUPPORTED_QUANTITIES: [RenderAttachmentQuantity; 10] = [
        RenderAttachmentQuantity::LinearDepth,
        RenderAttachmentQuantity::NormalVector,
        RenderAttachmentQuantity::MotionVector,
        RenderAttachmentQuantity::MaterialColor,
        RenderAttachmentQuantity::MaterialProperties,
        RenderAttachmentQuantity::Luminance,
        RenderAttachmentQuantity::LuminanceHistory,
        RenderAttachmentQuantity::PreviousLuminanceHistory,
        RenderAttachmentQuantity::LuminanceAux,
        RenderAttachmentQuantity::Occlusion,
    ];

    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
    ) -> Result<Self> {
        let mut passes = HashMap::with_capacity(Self::SUPPORTED_QUANTITIES.len());
        for quantity in Self::SUPPORTED_QUANTITIES {
            let shader_template = RenderAttachmentVisualizationShaderTemplate::new(quantity);
            passes.insert(
                quantity,
                PostprocessingRenderPass::new(
                    graphics_device,
                    rendering_surface,
                    shader_manager,
                    render_attachment_texture_manager,
                    gpu_resource_group_manager,
                    &shader_template,
                    Cow::Owned(format!(
                        "Visualization pass for render attachment: {:?}",
                        quantity
                    )),
                )?,
            );
        }
        Ok(Self {
            passes,
            idx_of_quantity_to_visualize: 0,
            enabled: false,
        })
    }

    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(super) fn enabled_mut(&mut self) -> &mut bool {
        &mut self.enabled
    }

    pub(super) fn quantity(&self) -> RenderAttachmentQuantity {
        Self::SUPPORTED_QUANTITIES[self.idx_of_quantity_to_visualize]
    }

    pub(super) fn set_quantity(&mut self, to: RenderAttachmentQuantity) -> Result<()> {
        self.idx_of_quantity_to_visualize = Self::SUPPORTED_QUANTITIES
            .iter()
            .position(|&quantity| quantity == to)
            .ok_or_else(|| {
                anyhow!("Visualization of render attachment quantity {to:?} not supported")
            })?;
        Ok(())
    }

    pub(super) fn cycle_quantity_forward(&mut self) {
        self.idx_of_quantity_to_visualize =
            (self.idx_of_quantity_to_visualize + 1) % Self::SUPPORTED_QUANTITIES.len();
    }

    pub(super) fn cycle_quantity_backward(&mut self) {
        self.idx_of_quantity_to_visualize =
            (self.idx_of_quantity_to_visualize + Self::SUPPORTED_QUANTITIES.len() - 1)
                % Self::SUPPORTED_QUANTITIES.len();
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
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.enabled {
            let quantity = self.quantity();
            self.passes[&quantity].record(
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
