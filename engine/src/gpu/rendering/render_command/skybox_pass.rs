//! Pass for filling in emitted luminance from the skybox.

use super::{STANDARD_FRONT_FACE, StencilValue};
use crate::{
    gpu::rendering::{
        attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
        postprocessing::Postprocessor,
        push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
        render_command::begin_single_render_pass,
        resource::SynchronizedRenderResources,
        shader_templates::skybox::SkyboxShaderTemplate,
    },
    skybox::Skybox,
};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_gpu::{device::GraphicsDevice, query::TimestampQueryRegistry, shader::ShaderManager};
use impact_mesh::{self, VertexAttributeSet, VertexPosition, buffer::VertexBufferable};
use std::borrow::Cow;

/// Pass for filling in emitted luminance from the skybox.
#[derive(Debug)]
pub struct SkyboxPass {
    push_constants: RenderingPushConstantGroup,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
    color_target_state: wgpu::ColorTargetState,
    depth_stencil_state: wgpu::DepthStencilState,
    pipeline: Option<wgpu::RenderPipeline>,
    skybox: Option<Skybox>,
}

impl SkyboxPass {
    pub fn new(graphics_device: &GraphicsDevice, shader_manager: &mut ShaderManager) -> Self {
        let push_constants = SkyboxShaderTemplate::push_constants();
        let output_render_attachment_quantity =
            SkyboxShaderTemplate::output_render_attachment_quantity();

        let push_constant_ranges = push_constants.create_ranges();
        let color_target_state = Self::color_target_state(output_render_attachment_quantity);
        let depth_stencil_state = super::depth_stencil_state_for_equal_stencil_testing();

        // Make sure the shader is compiled
        shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &SkyboxShaderTemplate);

        Self {
            push_constants,
            output_render_attachment_quantity,
            push_constant_ranges,
            color_target_state,
            depth_stencil_state,
            pipeline: None,
            skybox: None,
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_resources: &SynchronizedRenderResources,
    ) {
        match (
            self.skybox.as_ref(),
            render_resources.get_skybox_resource_manager(),
        ) {
            (Some(&skybox), Some(skybox_resource_manager))
                if skybox == skybox_resource_manager.skybox() => {}
            (_, None) => {
                self.pipeline = None;
                self.skybox = None;
            }
            (_, Some(skybox_resource_manager)) => {
                let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
                    graphics_device,
                    &SkyboxShaderTemplate,
                );

                let pipeline_layout = super::create_render_pipeline_layout(
                    graphics_device.device(),
                    &[
                        CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device),
                        skybox_resource_manager.bind_group_layout(),
                    ],
                    &self.push_constant_ranges,
                    "Skybox pass render pipeline layout",
                );

                self.pipeline = Some(super::create_render_pipeline(
                    graphics_device.device(),
                    &pipeline_layout,
                    shader,
                    &[VertexPosition::BUFFER_LAYOUT],
                    &[Some(self.color_target_state.clone())],
                    STANDARD_FRONT_FACE,
                    Some(wgpu::Face::Back),
                    wgpu::PolygonMode::Fill,
                    Some(self.depth_stencil_state.clone()),
                    "Skybox pass render pipeline",
                ));
                self.skybox = Some(skybox_resource_manager.skybox());
            }
        }
    }

    fn color_target_state(
        output_render_attachment_quantity: RenderAttachmentQuantity,
    ) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: output_render_attachment_quantity.texture_format(),
            blend: Some(super::additive_blend_state()),
            write_mask: wgpu::ColorWrites::COLOR,
        }
    }

    fn color_attachment<'a, 'b: 'a>(
        &self,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> wgpu::RenderPassColorAttachment<'a> {
        let texture = render_attachment_texture_manager
            .render_attachment_texture(self.output_render_attachment_quantity);
        wgpu::RenderPassColorAttachment {
            view: texture.base_texture_view(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }

    fn depth_stencil_attachment(
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
    ) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: render_attachment_texture_manager
                .render_attachment_texture(RenderAttachmentQuantity::DepthStencil)
                .base_texture_view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
        }
    }

    fn set_push_constants(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        postprocessor: &Postprocessor,
        camera_buffer_manager: &CameraGPUBufferManager,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                RenderingPushConstantVariant::CameraRotationQuaternion,
                || camera_buffer_manager.camera_rotation_quaternion_push_constant(),
            );
    }

    pub fn record(
        &self,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let pipeline = if let Some(pipeline) = self.pipeline.as_ref() {
            pipeline
        } else {
            return Ok(());
        };

        let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
            return Ok(());
        };

        let color_attachment = self.color_attachment(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            Some(depth_stencil_attachment),
            Cow::Borrowed("Skybox pass"),
        );

        render_pass.set_pipeline(pipeline);

        render_pass.set_stencil_reference(StencilValue::Background as u32);

        self.set_push_constants(&mut render_pass, postprocessor, camera_buffer_manager);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        let skybox_resource_manager = render_resources
            .get_skybox_resource_manager()
            .ok_or_else(|| anyhow!("Missing GPU resources for skybox"))?;

        render_pass.set_bind_group(1, skybox_resource_manager.bind_group(), &[]);

        let mesh_id = impact_mesh::skybox_mesh_id();

        let mesh_buffer_manager = render_resources
            .get_triangle_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        let position_buffer = mesh_buffer_manager
            .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
            .next()
            .unwrap();

        render_pass.set_vertex_buffer(0, position_buffer.valid_buffer_slice());

        render_pass.set_index_buffer(
            mesh_buffer_manager
                .triangle_mesh_index_gpu_buffer()
                .valid_buffer_slice(),
            mesh_buffer_manager.triangle_mesh_index_format(),
        );

        render_pass.draw_indexed(
            0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
            0,
            0..1,
        );

        log::trace!("Recorded skybox pass");

        Ok(())
    }
}
