//! Pass for filling the G-buffer attachments and the depth and stencil map.

use crate::{
    BasicRenderingConfig,
    attachment::{
        Blending, RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
        RenderAttachmentTextureManager,
    },
    postprocessing::Postprocessor,
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::{self, STANDARD_FRONT_FACE, StencilValue, begin_single_render_pass},
    resource::BasicGPUResources,
    shader_templates::model_geometry::{ModelGeometryShaderInput, ModelGeometryShaderTemplate},
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_camera::gpu_resource::CameraGPUResource;
use impact_containers::{HashMap, HashSet};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    query::TimestampQueryRegistry,
    shader::{ShaderManager, template::SpecificShaderTemplate},
    wgpu,
};
use impact_material::{Material, gpu_resource::GPUMaterialTemplate};
use impact_mesh::VertexAttributeSet;
use impact_model::{InstanceFeature, transform::InstanceModelViewTransformWithPrevious};
use impact_scene::model::ModelID;
use std::{borrow::Cow, collections::hash_map::Entry};

/// Pass for filling the G-buffer attachments and the depth and stencil map.
#[derive(Debug)]
pub struct GeometryPass {
    push_constants: BasicPushConstantGroup,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
    color_target_states: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil_state: wgpu::DepthStencilState,
    polygon_mode: wgpu::PolygonMode,
    model_pipelines: HashMap<ModelGeometryShaderInput, GeometryPassPipeline>,
}

#[derive(Debug)]
struct GeometryPassPipeline {
    shader_template: ModelGeometryShaderTemplate,
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    vertex_attributes: VertexAttributeSet,
    models: HashSet<ModelID>,
}

impl GeometryPass {
    pub fn new(config: &BasicRenderingConfig) -> Self {
        let push_constants = ModelGeometryShaderTemplate::push_constants();
        let output_render_attachments = ModelGeometryShaderTemplate::output_render_attachments();

        let push_constant_ranges = push_constants.create_ranges();

        let color_target_states = Self::create_color_target_states(&output_render_attachments);

        let depth_stencil_state = render_command::depth_stencil_state_for_depth_stencil_write();

        let polygon_mode = if config.wireframe_mode_on {
            wgpu::PolygonMode::Line
        } else {
            wgpu::PolygonMode::Fill
        };

        Self {
            push_constants,
            output_render_attachments,
            push_constant_ranges,
            color_target_states,
            depth_stencil_state,
            polygon_mode,
            model_pipelines: HashMap::default(),
        }
    }

    pub fn color_target_states(&self) -> &[Option<wgpu::ColorTargetState>] {
        &self.color_target_states
    }

    pub fn depth_stencil_state(&self) -> &wgpu::DepthStencilState {
        &self.depth_stencil_state
    }

    pub fn sync_with_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        config: &BasicRenderingConfig,
    ) {
        self.polygon_mode = if config.wireframe_mode_on {
            wgpu::PolygonMode::Line
        } else {
            wgpu::PolygonMode::Fill
        };

        for pipeline in self.model_pipelines.values_mut() {
            pipeline.pipeline = Self::create_pipeline(
                graphics_device,
                shader_manager,
                &pipeline.shader_template,
                &pipeline.pipeline_layout,
                &pipeline.vertex_buffer_layouts,
                &self.color_target_states,
                self.polygon_mode,
                self.depth_stencil_state.clone(),
            );
        }
    }

    pub fn sync_with_render_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<()> {
        let model_instance_buffers = gpu_resources.model_instance_buffer();

        for pipeline in self.model_pipelines.values_mut() {
            pipeline
                .models
                .retain(|model_id| model_instance_buffers.contains(model_id));
        }

        let added_models: Vec<_> = model_instance_buffers
            .iter()
            .filter_map(|(model_id, instance_feature_buffers)| {
                for pipeline in self.model_pipelines.values() {
                    if pipeline.models.contains(model_id) {
                        return None;
                    }
                }
                // We only add a pipeline for the model if it actually has
                // buffered transforms, otherwise it will not be rendered
                // anyway
                if instance_feature_buffers
                    .iter()
                    .find(|buffer| {
                        buffer.is_for_feature_type::<InstanceModelViewTransformWithPrevious>()
                    })
                    .is_some_and(|buffer| buffer.has_features_in_initial_range())
                {
                    Some(*model_id)
                } else {
                    None
                }
            })
            .collect();

        self.add_models(
            graphics_device,
            shader_manager,
            gpu_resources,
            bind_group_layout_registry,
            &added_models,
        )
    }

    fn add_models<'a>(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl BasicGPUResources,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        models: impl IntoIterator<Item = &'a ModelID>,
    ) -> Result<()> {
        let camera_bind_group_layout = CameraGPUResource::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        for model_id in models {
            let Some(material) = gpu_resources.material().get(model_id.material_id()) else {
                continue;
            };
            let Some(material_template) =
                gpu_resources.material_template().get(material.template_id)
            else {
                continue;
            };
            let Some(input) =
                ModelGeometryShaderInput::for_material_template(&material_template.template)
            else {
                continue;
            };

            match self.model_pipelines.entry(input.clone()) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().models.insert(*model_id);
                }
                Entry::Vacant(entry) => {
                    let shader_template = ModelGeometryShaderTemplate::new(input);

                    shader_manager.get_or_create_rendering_shader_from_template(
                        graphics_device,
                        &shader_template,
                    );

                    let vertex_attributes = shader_template.input().vertex_attributes;

                    let mut bind_group_layouts = vec![&camera_bind_group_layout];

                    if let Some(bind_group_layout) = gpu_resources
                        .material_template()
                        .get(material.template_id)
                        .and_then(GPUMaterialTemplate::bind_group_layout)
                    {
                        bind_group_layouts.push(bind_group_layout);
                    }

                    let pipeline_layout = render_command::create_render_pipeline_layout(
                        graphics_device.device(),
                        &bind_group_layouts,
                        &self.push_constant_ranges,
                        &format!(
                            "Geometry pass render pipeline layout for shader: {:?}",
                            &shader_template
                        ),
                    );

                    let vertex_buffer_layouts = Self::create_vertex_buffer_layouts(
                        gpu_resources,
                        model_id,
                        vertex_attributes,
                    )?;

                    let pipeline = Self::create_pipeline(
                        graphics_device,
                        shader_manager,
                        &shader_template,
                        &pipeline_layout,
                        &vertex_buffer_layouts,
                        &self.color_target_states,
                        self.polygon_mode,
                        self.depth_stencil_state.clone(),
                    );

                    let mut models = HashSet::with_capacity_and_hasher(4, Default::default());
                    models.insert(*model_id);

                    entry.insert(GeometryPassPipeline {
                        shader_template,
                        vertex_buffer_layouts,
                        pipeline_layout,
                        pipeline,
                        vertex_attributes,
                        models,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_pipeline(
        graphics_device: &GraphicsDevice,
        shader_manager: &ShaderManager,
        shader_template: &ModelGeometryShaderTemplate,
        pipeline_layout: &wgpu::PipelineLayout,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        color_target_states: &[Option<wgpu::ColorTargetState>],
        polygon_mode: wgpu::PolygonMode,
        depth_stencil_state: wgpu::DepthStencilState,
    ) -> wgpu::RenderPipeline {
        let shader = &shader_manager.rendering_shaders[&shader_template.shader_id()];

        render_command::create_render_pipeline(
            graphics_device.device(),
            pipeline_layout,
            shader,
            vertex_buffer_layouts,
            color_target_states,
            STANDARD_FRONT_FACE,
            Some(wgpu::Face::Back),
            polygon_mode,
            Some(depth_stencil_state),
            &format!("Geometry pass render pipeline for shader: {shader_template:?}"),
        )
    }

    fn create_vertex_buffer_layouts(
        gpu_resources: &impl BasicGPUResources,
        model_id: &ModelID,
        vertex_attributes: VertexAttributeSet,
    ) -> Result<Vec<wgpu::VertexBufferLayout<'static>>> {
        let mut layouts = Vec::with_capacity(8);

        let instance_feature_buffers = gpu_resources
            .model_instance_buffer()
            .get_model_buffers(model_id)
            .ok_or_else(|| anyhow!("Missing instance GPU buffers for model {}", model_id))?;

        layouts.push(InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT.unwrap());

        if let Some(material_property_values_feature_type_id) = gpu_resources
            .material()
            .get(model_id.material_id())
            .and_then(|material| {
                material
                    .property_values
                    .instance_feature_type_id_if_applicable()
            })
        {
            let material_property_buffer = instance_feature_buffers
                .iter()
                .find(|buffer| {
                    buffer.is_for_feature_type_with_id(material_property_values_feature_type_id)
                })
                .ok_or_else(|| anyhow!("Missing material GPU buffer for model {}", model_id))?;

            layouts.push(material_property_buffer.vertex_buffer_layout().clone());
        }

        let mesh_id = model_id.triangle_mesh_id();

        let mesh_gpu_resources = gpu_resources
            .triangle_mesh()
            .get(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

        layouts.extend(mesh_gpu_resources.request_vertex_buffer_layouts(vertex_attributes)?);

        Ok(layouts)
    }

    fn create_color_target_states(
        output_render_attachments: &RenderAttachmentOutputDescriptionSet,
    ) -> Vec<Option<wgpu::ColorTargetState>> {
        RenderAttachmentQuantity::all()
            .iter()
            .filter_map(|quantity| {
                if output_render_attachments
                    .quantities()
                    .contains(quantity.flag())
                {
                    let description = output_render_attachments
                        .only_description_for_quantity(*quantity)
                        .unwrap();

                    let blend_state = match description.blending() {
                        Blending::Replace => wgpu::BlendState::REPLACE,
                        Blending::Additive => render_command::additive_blend_state(),
                    };

                    Some(Some(wgpu::ColorTargetState {
                        format: quantity.texture_format(),
                        blend: Some(blend_state),
                        write_mask: description.write_mask(),
                    }))
                } else {
                    None
                }
            })
            .collect()
    }

    fn create_color_attachments<'a, 'b: 'a>(
        &self,
        render_attachment_texture_manager: &'b RenderAttachmentTextureManager,
    ) -> Vec<Option<wgpu::RenderPassColorAttachment<'a>>> {
        let mut color_attachments = Vec::with_capacity(self.color_target_states.len());
        color_attachments.extend(
            render_attachment_texture_manager
                .request_render_attachment_textures(self.output_render_attachments.quantities())
                .map(|texture| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: texture.base_texture_view(),
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

    fn create_depth_stencil_attachment(
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
        postprocessor: &Postprocessor,
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

        self.push_constants
            .set_push_constant_for_render_pass_if_present(
                render_pass,
                BasicPushConstantVariant::Exposure,
                || postprocessor.capturing_camera().exposure_push_constant(),
            );
    }

    pub fn record<'a>(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &'a mut wgpu::CommandEncoder,
    ) -> Result<Option<wgpu::RenderPass<'a>>> {
        let Some(camera_gpu_resources) = gpu_resources.camera() else {
            return Ok(None);
        };

        let color_attachments = self.create_color_attachments(render_attachment_texture_manager);

        let depth_stencil_attachment =
            Self::create_depth_stencil_attachment(render_attachment_texture_manager);

        let mut render_pass = begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &color_attachments,
            Some(depth_stencil_attachment),
            Cow::Borrowed("Geometry pass"),
        );

        render_pass.set_stencil_reference(StencilValue::PhysicalModel as u32);

        render_pass.set_bind_group(0, camera_gpu_resources.bind_group(), &[]);

        for pipeline in self.model_pipelines.values() {
            render_pass.set_pipeline(&pipeline.pipeline);

            self.set_push_constants(
                &mut render_pass,
                rendering_surface,
                postprocessor,
                frame_counter,
            );

            for model_id in &pipeline.models {
                let instance_feature_buffers = gpu_resources
                    .model_instance_buffer()
                    .get_model_buffers(model_id)
                    .ok_or_else(|| {
                        anyhow!("Missing instance GPU buffers for model {}", model_id)
                    })?;

                let transform_buffer = instance_feature_buffers
                    .iter()
                    .find(|buffer| {
                        buffer.is_for_feature_type::<InstanceModelViewTransformWithPrevious>()
                    })
                    .ok_or_else(|| {
                        anyhow!(
                            "Missing model-view transform GPU buffer for model {}",
                            model_id
                        )
                    })?;

                let instance_range = transform_buffer.initial_feature_range();

                if instance_range.is_empty() {
                    continue;
                }

                let material = gpu_resources.material().get(model_id.material_id());

                if let Some(material_texture_bind_group) = material
                    .and_then(Material::texture_group_id_if_non_empty)
                    .and_then(|texture_group_id| {
                        gpu_resources.material_texture_group().get(texture_group_id)
                    })
                {
                    render_pass.set_bind_group(1, &material_texture_bind_group.bind_group, &[]);
                }

                render_pass.set_vertex_buffer(
                    0,
                    transform_buffer.vertex_gpu_buffer().valid_buffer_slice(),
                );

                let mut vertex_buffer_slot = 1;

                if let Some(material_property_values_feature_type_id) =
                    material.and_then(|material| {
                        material
                            .property_values
                            .instance_feature_type_id_if_applicable()
                    })
                {
                    let material_property_buffer = instance_feature_buffers
                        .iter()
                        .find(|buffer| {
                            buffer.is_for_feature_type_with_id(
                                material_property_values_feature_type_id,
                            )
                        })
                        .ok_or_else(|| {
                            anyhow!("Missing material GPU buffer for model {}", model_id)
                        })?;

                    render_pass.set_vertex_buffer(
                        vertex_buffer_slot,
                        material_property_buffer
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );
                    vertex_buffer_slot += 1;
                }

                let mesh_id = model_id.triangle_mesh_id();

                let mesh_gpu_resources = gpu_resources
                    .triangle_mesh()
                    .get(mesh_id)
                    .ok_or_else(|| anyhow!("Missing GPU resources for mesh {}", mesh_id))?;

                for vertex_buffer in
                    mesh_gpu_resources.request_vertex_gpu_buffers(pipeline.vertex_attributes)?
                {
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
        }

        let n_models: usize = self
            .model_pipelines
            .values()
            .map(|pipeline| pipeline.models.len())
            .product();

        impact_log::trace!(
            "Recorded geometry pass for {} models ({} pipelines, {} draw calls)",
            n_models,
            self.model_pipelines.len(),
            n_models
        );

        Ok(Some(render_pass))
    }
}
