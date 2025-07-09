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
    resource::BasicRenderResources,
    shader_templates::model_geometry::{ModelGeometryShaderInput, ModelGeometryShaderTemplate},
    surface::RenderingSurface,
};
use anyhow::{Result, anyhow};
use impact_camera::buffer::CameraGPUBufferManager;
use impact_containers::{HashMap, HashSet};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryRegistry, shader::ShaderManager, wgpu,
};
use impact_material::MaterialLibrary;
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

    pub fn sync_with_render_resources<R>(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        material_library: &MaterialLibrary,
        render_resources: &R,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<()>
    where
        R: BasicRenderResources,
    {
        let instance_feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        for pipeline in self.model_pipelines.values_mut() {
            pipeline
                .models
                .retain(|model_id| instance_feature_buffer_managers.contains_key(model_id));
        }

        let added_models: Vec<_> = instance_feature_buffer_managers
            .iter()
            .filter_map(|(model_id, instance_feature_buffer_manager)| {
                for pipeline in self.model_pipelines.values() {
                    if pipeline.models.contains(model_id) {
                        return None;
                    }
                }
                // We only add a pipeline for the model if it actually has
                // buffered transforms, otherwise it will not be rendered
                // anyway
                if instance_feature_buffer_manager
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
            material_library,
            render_resources,
            bind_group_layout_registry,
            &added_models,
        )
    }

    fn add_models<'a>(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        material_library: &MaterialLibrary,
        render_resources: &impl BasicRenderResources,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        models: impl IntoIterator<Item = &'a ModelID>,
    ) -> Result<()> {
        let camera_bind_group_layout = CameraGPUBufferManager::get_or_create_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        for model_id in models {
            let material_handle = model_id.material_handle();
            if let Some(material_specification) =
                material_library.get_material_specification(material_handle.material_id())
            {
                if let Some(input) = ModelGeometryShaderInput::for_material(material_specification)
                {
                    match self.model_pipelines.entry(input.clone()) {
                        Entry::Occupied(mut entry) => {
                            entry.get_mut().models.insert(*model_id);
                        }
                        Entry::Vacant(entry) => {
                            let shader_template = ModelGeometryShaderTemplate::new(input);
                            let (_, shader) = shader_manager
                                .get_or_create_rendering_shader_from_template(
                                    graphics_device,
                                    &shader_template,
                                );

                            let vertex_attributes = shader_template.input().vertex_attributes;

                            let material_texture_bind_group_layout = material_handle
                                .material_property_texture_group_id()
                                .and_then(|texture_group_id| {
                                    material_library
                                        .get_material_property_texture_group(texture_group_id)
                                })
                                .map(|material_property_texture_group| {
                                    material_property_texture_group.bind_group_layout()
                                });

                            let mut bind_group_layouts = vec![&camera_bind_group_layout];
                            if let Some(material_texture_bind_group_layout) =
                                material_texture_bind_group_layout
                            {
                                bind_group_layouts.push(material_texture_bind_group_layout);
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
                                render_resources,
                                model_id,
                                vertex_attributes,
                            )?;

                            let pipeline = render_command::create_render_pipeline(
                                graphics_device.device(),
                                &pipeline_layout,
                                shader,
                                &vertex_buffer_layouts,
                                &self.color_target_states,
                                STANDARD_FRONT_FACE,
                                Some(wgpu::Face::Back),
                                self.polygon_mode,
                                Some(self.depth_stencil_state.clone()),
                                &format!(
                                    "Geometry pass render pipeline for shader: {:?}",
                                    &shader_template
                                ),
                            );

                            let mut models =
                                HashSet::with_capacity_and_hasher(4, Default::default());
                            models.insert(*model_id);

                            entry.insert(GeometryPassPipeline {
                                pipeline,
                                vertex_attributes,
                                models,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn create_vertex_buffer_layouts(
        render_resources: &impl BasicRenderResources,
        model_id: &ModelID,
        vertex_attributes: VertexAttributeSet,
    ) -> Result<Vec<wgpu::VertexBufferLayout<'static>>> {
        let mut layouts = Vec::with_capacity(8);

        let instance_feature_buffer_managers = render_resources
            .get_instance_feature_buffer_managers(model_id)
            .ok_or_else(|| anyhow!("Missing instance GPU buffers for model {}", model_id))?;

        layouts.push(InstanceModelViewTransformWithPrevious::BUFFER_LAYOUT.unwrap());

        // If the material has a buffer of per-instance features, it will be directly
        // after the transform buffers
        if model_id
            .material_handle()
            .material_property_feature_id()
            .is_some()
        {
            let material_property_buffer_manager = instance_feature_buffer_managers
                .get(2)
                .ok_or_else(|| anyhow!("Missing material GPU buffer for model {}", model_id))?;

            layouts.push(
                material_property_buffer_manager
                    .vertex_buffer_layout()
                    .clone(),
            );
        }

        let mesh_id = model_id.triangle_mesh_id();
        let mesh_buffer_manager = render_resources
            .get_triangle_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

        layouts.extend(mesh_buffer_manager.request_vertex_buffer_layouts(vertex_attributes)?);

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

    pub fn record<'a, R>(
        &self,
        rendering_surface: &RenderingSurface,
        material_library: &MaterialLibrary,
        render_resources: &R,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &'a mut wgpu::CommandEncoder,
    ) -> Result<Option<wgpu::RenderPass<'a>>>
    where
        R: BasicRenderResources,
    {
        let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() else {
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

        render_pass.set_bind_group(0, camera_buffer_manager.bind_group(), &[]);

        for pipeline in self.model_pipelines.values() {
            render_pass.set_pipeline(&pipeline.pipeline);

            self.set_push_constants(
                &mut render_pass,
                rendering_surface,
                postprocessor,
                frame_counter,
            );

            for model_id in &pipeline.models {
                let instance_feature_buffer_managers = render_resources
                    .get_instance_feature_buffer_managers(model_id)
                    .ok_or_else(|| {
                        anyhow!("Missing instance GPU buffers for model {}", model_id)
                    })?;

                let transform_buffer_manager = instance_feature_buffer_managers
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

                let instance_range = transform_buffer_manager.initial_feature_range();

                if instance_range.is_empty() {
                    continue;
                }

                if let Some(material_property_texture_group) = model_id
                    .material_handle()
                    .material_property_texture_group_id()
                    .and_then(|texture_group_id| {
                        material_library.get_material_property_texture_group(texture_group_id)
                    })
                {
                    render_pass.set_bind_group(
                        1,
                        material_property_texture_group.bind_group(),
                        &[],
                    );
                }

                render_pass.set_vertex_buffer(
                    0,
                    transform_buffer_manager
                        .vertex_gpu_buffer()
                        .valid_buffer_slice(),
                );

                let mut vertex_buffer_slot = 1;

                if let Some(material_property_feature_id) =
                    model_id.material_handle().material_property_feature_id()
                {
                    let material_property_buffer_manager = instance_feature_buffer_managers
                        .iter()
                        .find(|buffer| {
                            buffer.is_for_feature_type_with_id(
                                material_property_feature_id.feature_type_id(),
                            )
                        })
                        .ok_or_else(|| {
                            anyhow!("Missing material GPU buffer for model {}", model_id)
                        })?;

                    render_pass.set_vertex_buffer(
                        vertex_buffer_slot,
                        material_property_buffer_manager
                            .vertex_gpu_buffer()
                            .valid_buffer_slice(),
                    );
                    vertex_buffer_slot += 1;
                }

                let mesh_id = model_id.triangle_mesh_id();

                let mesh_buffer_manager = render_resources
                    .get_triangle_mesh_buffer_manager(mesh_id)
                    .ok_or_else(|| anyhow!("Missing GPU buffer for mesh {}", mesh_id))?;

                for vertex_buffer in
                    mesh_buffer_manager.request_vertex_gpu_buffers(pipeline.vertex_attributes)?
                {
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
