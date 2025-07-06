//! Render passes for visualizing render attachments.

use crate::{
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    postprocessing::Postprocessor,
    render_command::postprocessing_pass::PostprocessingRenderPass,
    resource::BasicRenderResources,
    shader_templates::render_attachment_visualization::RenderAttachmentVisualizationShaderTemplate,
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_containers::HashMap;
use impact_gpu::{
    device::GraphicsDevice, query::TimestampQueryRegistry, resource_group::GPUResourceGroupManager,
    shader::ShaderManager, wgpu,
};
use std::borrow::Cow;

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
        let mut passes =
            HashMap::with_capacity_and_hasher(Self::SUPPORTED_QUANTITIES.len(), Default::default());
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
                        "Visualization pass for render attachment: {quantity:?}"
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

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn enabled_mut(&mut self) -> &mut bool {
        &mut self.enabled
    }

    pub fn quantity(&self) -> RenderAttachmentQuantity {
        Self::SUPPORTED_QUANTITIES[self.idx_of_quantity_to_visualize]
    }

    pub fn set_quantity(&mut self, to: RenderAttachmentQuantity) -> Result<()> {
        self.idx_of_quantity_to_visualize = Self::SUPPORTED_QUANTITIES
            .iter()
            .position(|&quantity| quantity == to)
            .ok_or_else(|| {
                anyhow!("Visualization of render attachment quantity {to:?} not supported")
            })?;
        Ok(())
    }

    pub fn cycle_quantity_forward(&mut self) {
        self.idx_of_quantity_to_visualize =
            (self.idx_of_quantity_to_visualize + 1) % Self::SUPPORTED_QUANTITIES.len();
    }

    pub fn cycle_quantity_backward(&mut self) {
        self.idx_of_quantity_to_visualize =
            (self.idx_of_quantity_to_visualize + Self::SUPPORTED_QUANTITIES.len() - 1)
                % Self::SUPPORTED_QUANTITIES.len();
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
