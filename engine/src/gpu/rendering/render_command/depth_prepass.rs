//! Pass for filling the depth and stencil map.

use crate::gpu::{
    GraphicsDevice,
    rendering::{
        BasicRenderingConfig,
        attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
        push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
        render_command::{STANDARD_FRONT_FACE, StencilValue, begin_single_render_pass},
        resource::BasicRenderResources,
        shader_templates::model_depth_prepass::ModelDepthPrepassShaderTemplate,
        surface::RenderingSurface,
    },
};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_containers::HashSet;
use impact_gpu::{query::TimestampQueryRegistry, shader::ShaderManager};
use impact_material::{MaterialLibrary, MaterialShaderInput};
use impact_mesh::{VertexAttributeSet, VertexPosition, buffer::VertexBufferable};
use impact_model::{InstanceFeature, transform::InstanceModelViewTransformWithPrevious};
use impact_scene::model::ModelID;
use std::borrow::Cow;

/// Pass for filling the depth and stencil map.
#[derive(Debug)]
pub struct DepthPrepass {
    push_constants: BasicPushConstantGroup,
    pipeline: wgpu::RenderPipeline,
    models: HashSet<ModelID>,
    write_stencil_value: StencilValue,
}

impl DepthPrepass {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        write_stencil_value: StencilValue,
        config: &BasicRenderingConfig,
    ) -> Self {
        let (_, shader) = shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &ModelDepthPrepassShaderTemplate,
        );

        let push_constants = ModelDepthPrepassShaderTemplate::push_constants();

        let pipeline_layout = super::create_render_pipeline_layout(
            graphics_device.device(),
            &[CameraGPUBufferManager::get_or_create_bind_group_layout(
                graphics_device,
            )],
            &push_constants.create_ranges(),
            "Depth prepass render pipeline layout",
        );

        let pipeline = super::create_render_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            &[
                InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT.unwrap(),
                VertexPosition::BUFFER_LAYOUT,
            ],
            &[],
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            if config.wireframe_mode_on {
                wgpu::PolygonMode::Line
            } else {
                wgpu::PolygonMode::Fill
            },
            Some(super::depth_stencil_state_for_depth_stencil_write()),
            "Depth prepass render pipeline",
        );

        Self {
            push_constants,
            pipeline,
            models: HashSet::default(),
            write_stencil_value,
        }
    }

    pub fn sync_with_render_resources_for_non_physical_models(
        &mut self,
        material_library: &MaterialLibrary,
        render_resources: &impl BasicRenderResources,
    ) {
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        self.models
            .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));

        for model_id in instance_feature_buffer_managers.keys() {
            if self.models.contains(model_id) {
                continue;
            }
            if let Some(material_specification) = material_library
                .get_material_specification(model_id.material_handle().material_id())
            {
                if let MaterialShaderInput::Fixed(_) = material_specification.shader_input() {
                    self.models.insert(*model_id);
                }
            }
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
        rendering_surface: &RenderingSurface,
        frame_counter: u32,
    ) {
        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::InverseWindowDimensions,
                || rendering_surface.inverse_window_dimensions_push_constant(),
            );

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::FrameCounter,
                || frame_counter,
            );
    }

    pub fn record(
        &self,
        rendering_surface: &RenderingSurface,
        render_resources: &impl BasicRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.models.is_empty() {
            return Ok(());
        }

        let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
            return Ok(());
        };

        let depth_stencil_attachment =
            Self::depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[],
            Some(depth_stencil_attachment),
            Cow::Borrowed("Depth prepass"),
        );

        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_stencil_reference(self.write_stencil_value as u32);

        self.set_push_constants(&mut render_pass, rendering_surface, frame_counter);

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for model_id in &self.models {
            let transform_buffer_manager = render_resources
                .get_instance_feature_buffer_manager_for_feature_type::<InstanceModelViewTransformWithPrevious>(model_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing model-view transform GPU buffer for model {}",
                        model_id
                    )
                })?;

            let transform_range = transform_buffer_manager.initial_feature_range();

            if transform_range.is_empty() {
                continue;
            }

            render_pass.set_vertex_buffer(
                0,
                transform_buffer_manager
                    .vertex_gpu_buffer()
                    .valid_buffer_slice(),
            );

            let mesh_id = model_id.mesh_id();

            let mesh_buffer_manager = render_resources
                .get_triangle_mesh_buffer_manager(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

            let position_buffer = mesh_buffer_manager
                .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                .next()
                .unwrap();

            render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

            render_pass.set_index_buffer(
                mesh_buffer_manager
                    .triangle_mesh_index_gpu_buffer()
                    .valid_buffer_slice(),
                mesh_buffer_manager.triangle_mesh_index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                transform_range,
            );
        }

        impact_log::trace!(
            "Recorded depth prepass for {} models ({} draw calls)",
            self.models.len(),
            self.models.len()
        );

        Ok(())
    }
}
