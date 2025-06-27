//! Command for copying the contents of one render attachment texture into
//! another.

use crate::gpu::rendering::attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager};

/// Recorder for a command copying the contents of one render attachment texture
/// into another.
#[derive(Debug)]
pub struct RenderAttachmentTextureCopyCommand {
    source: RenderAttachmentQuantity,
    destination: RenderAttachmentQuantity,
}

impl RenderAttachmentTextureCopyCommand {
    /// Creates a new render attachment texture copy command for the given
    /// source and destination render attachment quantities.
    ///
    /// # Panics
    /// - If the source and destination render attachment quantities are the
    ///   same.
    /// - If the source and destination texture formats are not the same.
    pub fn new(source: RenderAttachmentQuantity, destination: RenderAttachmentQuantity) -> Self {
        if source == destination {
            panic!(
                "Tried to create render attachment texture copy command with same source and destination: {:?}",
                source,
            );
        }
        if source.texture_format() != destination.texture_format() {
            panic!(
                "Tried to create render attachment texture copy command with different formats: {:?} and {:?}",
                source, destination,
            );
        }
        Self {
            source,
            destination,
        }
    }

    /// Records the copy pass to the given command encoder.
    pub fn record(
        &self,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let source_texture = render_attachment_texture_manager
            .render_attachment_texture(self.source)
            .texture()
            .texture();
        let destination_texture = render_attachment_texture_manager
            .render_attachment_texture(self.destination)
            .texture()
            .texture();

        command_encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: destination_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            source_texture.size(),
        );

        impact_log::trace!(
            "Recorded texture copy command ({:?} to {:?})",
            self.source,
            self.destination
        );
    }
}
