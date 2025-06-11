//! Pass for rendering gizmos.

use crate::{
    camera::buffer::CameraGPUBufferManager,
    gizmo,
    gpu::{
        GraphicsDevice,
        query::TimestampQueryRegistry,
        rendering::{
            render_command::begin_single_render_pass, resource::SynchronizedRenderResources,
            surface::RenderingSurface,
        },
        shader::{ShaderManager, template::line::LineShaderTemplate},
    },
    mesh::{VertexAttributeSet, VertexColor, VertexPosition, buffer::VertexBufferable},
    model::{InstanceFeature, transform::InstanceModelViewTransform},
    scene::ModelInstanceNode,
};
use anyhow::{Result, anyhow};
use std::borrow::Cow;

/// Pass for rendering gizmos.
#[derive(Debug)]
pub struct GizmoPass {
    pipeline: wgpu::RenderPipeline,
}

impl GizmoPass {
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
    ) -> Self {
        let camera_bind_group_layout =
            CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device);
        let vertex_buffer_layouts = Self::vertex_buffer_layouts();
        let color_target_state = Self::color_target_state(rendering_surface);

        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &LineShaderTemplate);

        let pipeline_layout = super::create_render_pipeline_layout(
            graphics_device.device(),
            &[camera_bind_group_layout],
            &[],
            "Gizmo pass render pipeline layout",
        );

        let pipeline = super::create_line_list_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &vertex_buffer_layouts,
            &[Some(color_target_state)],
            None,
            "Gizmo pass render pipeline",
        );

        Self { pipeline }
    }

    const fn vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 3] {
        [
            InstanceModelViewTransform::BUFFER_LAYOUT,
            VertexPosition::BUFFER_LAYOUT,
            VertexColor::BUFFER_LAYOUT,
        ]
    }

    fn color_target_state(rendering_surface: &RenderingSurface) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: rendering_surface.texture_format(),
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::all(),
        }
    }

    fn color_attachment(
        surface_texture_view: &wgpu::TextureView,
    ) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: surface_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }

    pub fn record(
        &self,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &SynchronizedRenderResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
            return Ok(());
        };

        let color_attachment = Self::color_attachment(surface_texture_view);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            None,
            Cow::Borrowed("Gizmo pass"),
        );

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for model_id in gizmo::gizmo_model_ids() {
            let instance_feature_buffer_managers = render_resources
                .get_instance_feature_buffer_managers(model_id)
                .ok_or_else(|| anyhow!("Missing instance GPU buffers for model {}", model_id))?;

            let transform_buffer_manager = instance_feature_buffer_managers
                .get(ModelInstanceNode::model_view_transform_feature_idx())
                .ok_or_else(|| {
                    anyhow!(
                        "Missing model-view transform GPU buffer for model {}",
                        model_id
                    )
                })?;

            let instance_range = transform_buffer_manager.initial_feature_range();

            if instance_range.is_empty() {
                continue;
            }

            render_pass.set_vertex_buffer(
                0,
                transform_buffer_manager
                    .vertex_gpu_buffer()
                    .valid_buffer_slice(),
            );

            let mut vertex_buffer_slot = 1;

            let mesh_id = model_id.mesh_id();

            let mesh_buffer_manager = render_resources
                .get_line_segment_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            for vertex_buffer in mesh_buffer_manager.request_vertex_gpu_buffers(
                VertexAttributeSet::POSITION | VertexAttributeSet::COLOR,
            )? {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            render_pass.draw(
                0..u32::try_from(mesh_buffer_manager.n_vertices()).unwrap(),
                instance_range,
            );
        }

        log::trace!("Recorded gizmo pass");

        Ok(())
    }
}
