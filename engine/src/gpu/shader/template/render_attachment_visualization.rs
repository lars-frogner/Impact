//! Shader template for visualizing render attachments.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for visualizing a render attachment texture by rendering it
/// to the display surface.
#[derive(Clone, Debug)]
pub struct RenderAttachmentVisualizationShaderTemplate {
    render_attachment_quantity: RenderAttachmentQuantity,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("passthrough")).unwrap());

impl RenderAttachmentVisualizationShaderTemplate {
    /// Creates a new visualization shader template for the given render
    /// attachment quantity.
    pub fn new(render_attachment_quantity: RenderAttachmentQuantity) -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let input_render_attachments =
            RenderAttachmentInputDescriptionSet::with_defaults(render_attachment_quantity.flag());

        Self {
            render_attachment_quantity,
            push_constants,
            input_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for RenderAttachmentVisualizationShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "input_texture_binding" => self.render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.render_attachment_quantity.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for RenderAttachmentVisualizationShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::empty()
    }

    fn writes_to_surface(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&RenderAttachmentVisualizationShaderTemplate::new(
            RenderAttachmentQuantity::NormalVector,
        ));
    }
}
