//! Passes for rendering gizmos.

use crate::{
    GizmoDepthClipping, GizmoObscurability,
    model::{GizmoInstanceModelViewTransform, GizmoModel},
    shader_templates::gizmo::GizmoShaderTemplate,
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::CameraGPUResource;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    shader::{Shader, ShaderManager},
    timestamp_query::TimestampQueryRegistry,
    wgpu,
};
use impact_mesh::{
    MeshPrimitive, VertexAttributeSet, VertexColor, VertexPosition, gpu_resource::VertexBufferable,
};
use impact_model::InstanceFeature;
use impact_rendering::{
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    render_command::{self, STANDARD_FRONT_FACE, begin_single_render_pass},
    resource::BasicGPUResources,
    surface::RenderingSurface,
};
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
    triangle_pipeline_with_unclipped_depth: GizmoPassPipeline,
    line_pipeline: GizmoPassPipeline,
    line_pipeline_with_unclipped_depth: GizmoPassPipeline,
}

#[derive(Debug)]
struct GizmoPassPipeline {
    obscurability: GizmoObscurability,
    depth_clipping: GizmoDepthClipping,
    mesh_primitive: MeshPrimitive,
    pipeline: wgpu::RenderPipeline,
}

impl GizmoPasses {
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let camera_bind_group_layout = CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let vertex_buffer_layouts = Self::vertex_buffer_layouts();

        let color_target_state = Self::color_target_state(rendering_surface);
        let color_target_states = [Some(color_target_state)];

        let (_, shader) = shader_manager
            .get_or_create_rendering_shader_from_template(graphics_device, &GizmoShaderTemplate);

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[&camera_bind_group_layout],
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
            GizmoInstanceModelViewTransform::BUFFER_LAYOUT.unwrap(),
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
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.depth_tested_pass.record(
            surface_texture_view,
            gpu_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        self.non_depth_tested_pass.record(
            surface_texture_view,
            gpu_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )?;

        log::trace!("Recorded gizmo passes");

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
            GizmoDepthClipping::Enabled,
            MeshPrimitive::Triangle,
        );

        let triangle_pipeline_with_unclipped_depth = GizmoPassPipeline::new(
            graphics_device,
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            obscurability,
            GizmoDepthClipping::Disabled,
            MeshPrimitive::Triangle,
        );

        let line_pipeline = GizmoPassPipeline::new(
            graphics_device,
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            obscurability,
            GizmoDepthClipping::Enabled,
            MeshPrimitive::LineSegment,
        );

        let line_pipeline_with_unclipped_depth = GizmoPassPipeline::new(
            graphics_device,
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            obscurability,
            GizmoDepthClipping::Disabled,
            MeshPrimitive::LineSegment,
        );

        Self {
            obscurability,
            triangle_pipeline,
            triangle_pipeline_with_unclipped_depth,
            line_pipeline,
            line_pipeline_with_unclipped_depth,
        }
    }

    fn color_attachment(
        surface_texture_view: &wgpu::TextureView,
    ) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: surface_texture_view,
            depth_slice: None,
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
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(camera_gpu_resources) = gpu_resources.camera() else {
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

        let (mut render_pass, _timestamp_span_guard) = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            depth_stencil_attachment,
            Cow::Borrowed(label),
        );

        self.triangle_pipeline
            .record(gpu_resources, camera_gpu_resources, &mut render_pass)?;

        self.triangle_pipeline_with_unclipped_depth.record(
            gpu_resources,
            camera_gpu_resources,
            &mut render_pass,
        )?;

        self.line_pipeline
            .record(gpu_resources, camera_gpu_resources, &mut render_pass)?;

        self.line_pipeline_with_unclipped_depth.record(
            gpu_resources,
            camera_gpu_resources,
            &mut render_pass,
        )?;

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
        depth_clipping: GizmoDepthClipping,
        mesh_primitive: MeshPrimitive,
    ) -> Self {
        let depth_stencil_state = match obscurability {
            GizmoObscurability::Obscurable => Some(wgpu::DepthStencilState {
                format: RenderAttachmentQuantity::depth_texture_format(),
                depth_write_enabled: false,
                // Allow equality to avoid discarding geometry flattened onto
                // the far plane due to `GizmoDepthClipping::Disabled`
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            GizmoObscurability::NonObscurable => None,
        };

        let unclipped_depth = depth_clipping == GizmoDepthClipping::Disabled;

        let label = format!(
            "Gizmo pass render pipeline {{ mesh_primitive: {mesh_primitive:?}, obscurability: {obscurability:?}, depth_clipping: {depth_clipping:?} }}"
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
                unclipped_depth,
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
                unclipped_depth,
                &label,
            ),
        };

        Self {
            obscurability,
            depth_clipping,
            mesh_primitive,
            pipeline,
        }
    }

    fn record(
        &self,
        gpu_resources: &impl BasicGPUResources,
        camera_gpu_resources: &CameraGPUResource,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> Result<()> {
        let models = crate::model::select_gizmo_models(
            self.mesh_primitive,
            self.obscurability,
            self.depth_clipping,
        );

        match self.mesh_primitive {
            MeshPrimitive::Triangle => Self::record_for_triangles(
                gpu_resources,
                camera_gpu_resources,
                render_pass,
                &self.pipeline,
                models,
            ),
            MeshPrimitive::LineSegment => Self::record_for_lines(
                gpu_resources,
                camera_gpu_resources,
                render_pass,
                &self.pipeline,
                models,
            ),
        }
    }

    fn record_for_triangles<'a>(
        gpu_resources: &impl BasicGPUResources,
        camera_gpu_resources: &CameraGPUResource,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        models: impl IntoIterator<Item = &'a GizmoModel>,
    ) -> Result<()> {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, camera_gpu_resources.bind_group(), &[]);

        for model in models {
            let transform_buffer = gpu_resources
                .model_instance_buffer()
                .get_model_buffer_for_feature_feature_type::<GizmoInstanceModelViewTransform>(
                    model.model_id(),
                )
                .ok_or_else(|| {
                    anyhow!(
                        "Missing model-view transform GPU buffer for gizmo mesh {}",
                        model.triangle_mesh_id()
                    )
                })?;

            let instance_range = transform_buffer.initial_feature_range();

            if instance_range.is_empty() {
                continue;
            }

            render_pass
                .set_vertex_buffer(0, transform_buffer.vertex_gpu_buffer().valid_buffer_slice());

            let mut vertex_buffer_slot = 1;

            let mesh_id = model.triangle_mesh_id();

            let mesh_gpu_resources = gpu_resources
                .triangle_mesh()
                .get(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

            for vertex_buffer in mesh_gpu_resources.request_vertex_gpu_buffers(
                VertexAttributeSet::POSITION | VertexAttributeSet::COLOR,
            )? {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            render_pass.set_index_buffer(
                mesh_gpu_resources
                    .triangle_mesh_index_gpu_buffer()
                    .valid_buffer_slice(),
                mesh_gpu_resources.triangle_mesh_index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_gpu_resources.n_indices()).unwrap(),
                0,
                instance_range,
            );
        }

        Ok(())
    }

    fn record_for_lines<'a>(
        gpu_resources: &impl BasicGPUResources,
        camera_gpu_resources: &CameraGPUResource,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        models: impl IntoIterator<Item = &'a GizmoModel>,
    ) -> Result<()> {
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, camera_gpu_resources.bind_group(), &[]);

        for model in models {
            let transform_buffer = gpu_resources
                .model_instance_buffer()
                .get_model_buffer_for_feature_feature_type::<GizmoInstanceModelViewTransform>(
                    model.model_id(),
                )
                .ok_or_else(|| {
                    anyhow!(
                        "Missing model-view transform GPU buffer for gizmo mesh {}",
                        model.line_segment_mesh_id()
                    )
                })?;

            let instance_range = transform_buffer.initial_feature_range();

            if instance_range.is_empty() {
                continue;
            }

            render_pass
                .set_vertex_buffer(0, transform_buffer.vertex_gpu_buffer().valid_buffer_slice());

            let mut vertex_buffer_slot = 1;

            let mesh_id = model.line_segment_mesh_id();

            let mesh_gpu_resources = gpu_resources
                .line_segment_mesh()
                .get(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

            for vertex_buffer in mesh_gpu_resources.request_vertex_gpu_buffers(
                VertexAttributeSet::POSITION | VertexAttributeSet::COLOR,
            )? {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            render_pass.draw(
                0..u32::try_from(mesh_gpu_resources.n_vertices()).unwrap(),
                instance_range,
            );
        }

        Ok(())
    }
}
