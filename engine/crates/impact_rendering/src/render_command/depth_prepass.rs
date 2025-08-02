//! Pass for filling the depth and stencil map.

use crate::{
    BasicRenderingConfig,
    attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::{self, STANDARD_FRONT_FACE, StencilValue, begin_single_render_pass},
    resource::{BasicGPUResources, BasicResourceRegistries},
    shader_templates::model_depth_prepass::ModelDepthPrepassShaderTemplate,
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::CameraGPUResource;
use impact_containers::HashSet;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    shader::{ShaderManager, template::SpecificShaderTemplate},
    wgpu,
};
use impact_material::MaterialTextureBindingLocations;
use impact_mesh::{VertexAttributeSet, VertexPosition, gpu_resource::VertexBufferable};
use impact_model::{InstanceFeature, transform::InstanceModelViewTransformWithPrevious};
use impact_scene::model::ModelID;
use std::borrow::Cow;

/// Pass for filling the depth and stencil map.
#[derive(Debug)]
pub struct DepthPrepass {
    push_constants: BasicPushConstantGroup,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    models: HashSet<ModelID>,
    write_stencil_value: StencilValue,
}

impl DepthPrepass {
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        write_stencil_value: StencilValue,
        config: &BasicRenderingConfig,
    ) -> Self {
        shader_manager.get_or_create_rendering_shader_from_template(
            graphics_device,
            &ModelDepthPrepassShaderTemplate,
        );

        let push_constants = ModelDepthPrepassShaderTemplate::push_constants();

        let camera_bind_group_layout = CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let pipeline_layout = render_command::create_render_pipeline_layout(
            graphics_device.device(),
            &[&camera_bind_group_layout],
            &push_constants.create_ranges(),
            "Depth prepass render pipeline layout",
        );

        let pipeline =
            Self::create_pipeline(graphics_device, shader_manager, &pipeline_layout, config);

        Self {
            push_constants,
            pipeline_layout,
            pipeline,
            models: HashSet::default(),
            write_stencil_value,
        }
    }

    fn create_pipeline(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        pipeline_layout: &wgpu::PipelineLayout,
        config: &BasicRenderingConfig,
    ) -> wgpu::RenderPipeline {
        let shader =
            &shader_manager.rendering_shaders[&ModelDepthPrepassShaderTemplate.shader_id()];

        render_command::create_render_pipeline(
            graphics_device.device(),
            pipeline_layout,
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
            Some(render_command::depth_stencil_state_for_depth_stencil_write()),
            "Depth prepass render pipeline",
        )
    }

    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.pipeline = Self::create_pipeline(
            graphics_device,
            shader_manager,
            &self.pipeline_layout,
            config,
        );
    }

    pub fn sync_with_render_resources_for_non_physical_models(
        &mut self,
        resource_registries: &impl BasicResourceRegistries,
        gpu_resources: &impl BasicGPUResources,
    ) {
        let model_instance_buffers = gpu_resources.model_instance_buffer();

        self.models
            .retain(|model_id| model_instance_buffers.contains(model_id));

        for model_id in model_instance_buffers.model_ids() {
            if self.models.contains(model_id) {
                continue;
            }
            if let Some(material_template) = resource_registries
                .material()
                .get(model_id.material_id())
                .and_then(|material| {
                    resource_registries
                        .material_template()
                        .get(material.template_id)
                })
            {
                if let MaterialTextureBindingLocations::Fixed(_) =
                    material_template.texture_binding_locations
                {
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
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.models.is_empty() {
            return Ok(());
        }

        let Some(camera_gpu_resources) = gpu_resources.camera() else {
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

        render_pass.set_bind_group(0, camera_gpu_resources.bind_group(), &[]);

        for model_id in &self.models {
            let transform_buffer = gpu_resources.model_instance_buffer()
                .get_model_buffer_for_feature_feature_type::<InstanceModelViewTransformWithPrevious>(model_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing model-view transform GPU buffer for model {}",
                        model_id
                    )
                })?;

            let transform_range = transform_buffer.initial_feature_range();

            if transform_range.is_empty() {
                continue;
            }

            render_pass
                .set_vertex_buffer(0, transform_buffer.vertex_gpu_buffer().valid_buffer_slice());

            let mesh_id = model_id.triangle_mesh_id();

            let mesh_gpu_resources = gpu_resources
                .triangle_mesh()
                .get(mesh_id)
                .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

            let position_buffer = mesh_gpu_resources
                .request_vertex_gpu_buffers(VertexAttributeSet::POSITION)?
                .next()
                .unwrap();

            render_pass.set_vertex_buffer(1, position_buffer.valid_buffer_slice());

            render_pass.set_index_buffer(
                mesh_gpu_resources
                    .triangle_mesh_index_gpu_buffer()
                    .valid_buffer_slice(),
                mesh_gpu_resources.triangle_mesh_index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_gpu_resources.n_indices()).unwrap(),
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
