//! Pass for clearing the render attachments.

use crate::{
    attachment::{
        RenderAttachmentQuantity, RenderAttachmentQuantitySet, RenderAttachmentTextureManager,
    },
    render_command::{StencilValue, begin_single_render_pass},
};
use anyhow::Result;
use impact_gpu::{timestamp_query::TimestampQueryRegistry, wgpu};
use std::borrow::Cow;

/// Pass for clearing the render attachments.
#[derive(Debug)]
pub struct AttachmentClearingPass {
    attachments: RenderAttachmentQuantitySet,
}

impl AttachmentClearingPass {
    const CLEAR_DEPTH: f32 = 1.0;

    const MAX_ATTACHMENTS_PER_PASS: usize = 8;

    pub fn new(attachments: RenderAttachmentQuantitySet) -> Self {
        Self { attachments }
    }

    fn color_attachments<'a, 'b: 'a>(
        &self,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'a>>> {
        let mut color_attachments = Vec::with_capacity(RenderAttachmentQuantity::count());

        color_attachments.extend(
            render_attachment_texture_manager
                .request_render_attachment_textures(self.attachments.with_clear_color_only())
                .map(|texture| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: texture.base_texture_view(),
                depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(texture.quantity().clear_color().unwrap()),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
        );

        color_attachments
    }

    fn depth_stencil_attachment<'a>(
        &self,
        render_attachment_texture_manager: &'a RenderAttachmentTextureManager,
    ) -> Option<wgpu::RenderPassDepthStencilAttachment<'a>> {
        if self
            .attachments
            .contains(RenderAttachmentQuantitySet::DEPTH_STENCIL)
        {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: render_attachment_texture_manager
                    .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                    .base_texture_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Self::CLEAR_DEPTH),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(StencilValue::Background as u32),
                    store: wgpu::StoreOp::Store,
                }),
            })
        } else {
            None
        }
    }

    pub fn record(
        &self,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let color_attachments = self.color_attachments(render_attachment_texture_manager);

        let mut depth_stencil_attachment =
            self.depth_stencil_attachment(render_attachment_texture_manager);

        let n_attachments =
            color_attachments.len() + usize::from(depth_stencil_attachment.is_some());

        if color_attachments.len() < Self::MAX_ATTACHMENTS_PER_PASS {
            begin_single_render_pass(
                command_encoder,
                timestamp_recorder,
                &color_attachments,
                depth_stencil_attachment,
                Cow::Borrowed("Clearing pass"),
            );
        } else {
            // Chunk up the passes to avoid exceeding the maximum number of color
            // attachments
            for (idx, color_attachments) in color_attachments.chunks(8).enumerate() {
                // Only clear depth once
                let depth_stencil_attachment = depth_stencil_attachment.take();
                command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments,
                    depth_stencil_attachment,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    label: Some(&format!("Clearing pass {idx}")),
                });
            }
        }

        impact_log::trace!("Recorded clearing pass for {n_attachments} render attachments");

        Ok(())
    }
}
