//! Passes for rendering gizmos.

use crate::gizmo::{self, GizmoObscurability};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_gpu::{
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    shader::{Shader, ShaderManager},
};
use impact_mesh::{
    MeshPrimitive, VertexAttributeSet, VertexColor, VertexPosition, buffer::VertexBufferable,
};
use impact_model::{InstanceFeature, transform::InstanceModelViewTransform};
use impact_rendering::{
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    render_command::{self, STANDARD_FRONT_FACE, begin_single_render_pass},
    resource::BasicRenderResources,
    shader_templates::fixed_color::FixedColorShaderTemplate,
    surface::RenderingSurface,
};
use impact_scene::model::ModelID;
use std::borrow::Cow;

/// Passes for rendering gizmos.
#[derive(Debug)]
pub struct GizmoPasses {
    depth_tested_pass: GizmoPass,
    non_depth_tested_pass: GizmoPass,
}

#[derive(Debug)]
struct GizmoPass {
    obscurability: GizmoObscurability,
    triangle_pipeline: GizmoPassPipeline,
    line_pipeline: GizmoPassPipeline,
}

#[derive(Debug)]
struct GizmoPassPipeline {
    obscurability: GizmoObscurability,
    mesh_primitive: MeshPrimitive,
    pipeline: wgpu::RenderPipeline,
}

impl GizmoPasses {
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
    ) -> Self {
        let camera_bind_group_layout =
            CameraGPUBufferManager::get_or_create_bind_group_layout(graphics_device);

        let vertex_buffer_layouts = Self::vertex_buffer_layouts();

        let color_target_state = Self::color_target_state(rendering_surface);
        let color_target_states = [Some(color_target_state)];

        let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &FixedColorShaderTemplate,
        );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[camera_bind_group_layout],
            &[],
            "Gizmo pass render pipeline layout",
        );

        let depth_tested_pass = GizmoPass::new(
            graphics_device,
            &pipeline_layout,
            shader,
            &vertex_buffer_layouts,
            &color_target_states,
            GizmoObscurability::Obscurable,
        );

        let non_depth_tested_pass = GizmoPass::new(
            graphics_device,
            &pipeline_layout,
            shader,
            &vertex_buffer_layouts,
            &color_target_states,
            GizmoObscurability::NonObscurable,
        );

        Self {
            depth_tested_pass,
            non_depth_tested_pass,
        }
    }

    const fn vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 3] {
        [
            InstanceModelViewTransform::BUFFER_LAYOUT.unwrap(),
            VertexPosition::BUFFER_LAYOUT,
            VertexColor::BUFFER_LAYOUT,
        ]
    }

    fn color_target_state(rendering_surface: &RenderingSurface) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: rendering_surface.texture_format(),
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
        }
    }

    pub fn record(
        &self,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &impl BasicRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.depth_tested_pass.record(
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        self.non_depth_tested_pass.record(
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        impact_log::trace!("Recorded gizmo passes");

        Ok(())
    }
}

impl GizmoPass {
    fn new(
        graphics_device: &GraphicsDevice,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &Shader,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        obscurability: GizmoObscurability,
    ) -> Self {
        let triangle_pipeline = GizmoPassPipeline::new(
            graphics_device,
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            obscurability,
            MeshPrimitive::Triangle,
        );

        let line_pipeline = GizmoPassPipeline::new(
            graphics_device,
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            obscurability,
            MeshPrimitive::LineSegment,
        );

        Self {
            obscurability,
            triangle_pipeline,
            line_pipeline,
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
            stencil_ops: None,
        }
    }

    fn record(
        &self,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &impl BasicRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
            return Ok(());
        };

        let color_attachment = Self::color_attachment(surface_texture_view);

        let (label, depth_stencil_attachment) = match self.obscurability {
            GizmoObscurability::Obscurable => (
                "Gizmo pass with depth testing",
                Some(Self::depth_stencil_attachment(
                    render_attachment_texture_manager,
                )),
            ),
            GizmoObscurability::NonObscurable => ("Gizmo pass without depth testing", None),
        };

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            depth_stencil_attachment,
            Cow::Borrowed(label),
        );

        self.triangle_pipeline
            .record(render_resources, camera_buffer_manager, &mut render_pass)?;

        self.line_pipeline
            .record(render_resources, camera_buffer_manager, &mut render_pass)?;

        Ok(())
    }
}

impl GizmoPassPipeline {
    fn new(
        graphics_device: &GraphicsDevice,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &Shader,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        obscurability: GizmoObscurability,
        mesh_primitive: MeshPrimitive,
    ) -> Self {
        let depth_stencil_state = match obscurability {
            GizmoObscurability::Obscurable => {
                Some(render_command::depth_stencil_state_for_depth_test_without_write())
            }
            GizmoObscurability::NonObscurable => None,
        };

        let label = format!(
            "Gizmo pass render pipeline {{ mesh_primitive: {mesh_primitive:?}, obscurability: {obscurability:?} }}"
        );

        let pipeline = match mesh_primitive {
            MeshPrimitive::Triangle => render_command::create_render_pipeline(
                graphics_device.device(),
                pipeline_layout,
                shader,
                vertex_buffer_layouts,
                color_target_states,
                STANDARD_FRONT_FACE,
                None,
                wgpu::PolygonMode::Fill,
                depth_stencil_state,
                &label,
            ),
            MeshPrimitive::LineSegment => render_command::create_line_list_render_pipeline(
                graphics_device.device(),
                pipeline_layout,
                shader,
                vertex_buffer_layouts,
                color_target_states,
                depth_stencil_state,
                &label,
            ),
        };

        Self {
            obscurability,
            mesh_primitive,
            pipeline,
        }
    }

    fn record(
        &self,
        render_resources: &impl BasicRenderResources,
        camera_buffer_manager: &CameraGPUBufferManager,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()> {
        let model_ids = gizmo::model::gizmo_model_ids_for_mesh_primitive_and_obscurability(
            self.mesh_primitive,
            self.obscurability,
        );

        match self.mesh_primitive {
            MeshPrimitive::Triangle => Self::record_for_triangles(
                render_resources,
                camera_buffer_manager,
                render_pass,
                &self.pipeline,
                model_ids,
            ),
            MeshPrimitive::LineSegment => Self::record_for_lines(
                render_resources,
                camera_buffer_manager,
                render_pass,
                &self.pipeline,
                model_ids,
            ),
        }
    }

    fn record_for_triangles<'a>(
        render_resources: &impl BasicRenderResources,
        camera_buffer_manager: &CameraGPUBufferManager,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        model_ids: impl IntoIterator<Item = &'a ModelID>,
    ) -> Result<()> {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for model_id in model_ids {
            let transform_buffer_manager = render_resources
                .get_instance_feature_buffer_manager_for_feature_type::<InstanceModelViewTransform>(
                    model_id,
                )
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

            let mesh_id = model_id.triangle_mesh_id();

            let mesh_buffer_manager = render_resources
                .get_triangle_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            for vertex_buffer in mesh_buffer_manager.request_vertex_gpu_buffers(
                VertexAttributeSet::POSITION | VertexAttributeSet::COLOR,
            )? {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            render_pass.set_index_buffer(
                mesh_buffer_manager
                    .triangle_mesh_index_gpu_buffer()
                    .valid_buffer_slice(),
                mesh_buffer_manager.triangle_mesh_index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                instance_range,
            );
        }

        Ok(())
    }

    fn record_for_lines<'a>(
        render_resources: &impl BasicRenderResources,
        camera_buffer_manager: &CameraGPUBufferManager,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        model_ids: impl IntoIterator<Item = &'a ModelID>,
    ) -> Result<()> {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for model_id in model_ids {
            let transform_buffer_manager = render_resources
                .get_instance_feature_buffer_manager_for_feature_type::<InstanceModelViewTransform>(
                    model_id,
                )
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

            let mesh_id = model_id.line_segment_mesh_id();

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

        Ok(())
    }
}
