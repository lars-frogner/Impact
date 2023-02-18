//! Rendering pipelines.

mod tasks;

pub use tasks::SyncRenderPasses;

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        camera::CameraRenderBufferManager, instance::InstanceFeatureRenderBufferManager,
        mesh::MeshRenderBufferManager, resource::SynchronizedRenderResources, CameraShaderInput,
        CoreRenderingSystem, DepthTexture, InstanceFeatureShaderInput, LightShaderInput,
        MaterialRenderResourceManager, MaterialShaderInput, MeshShaderInput, RenderingConfig,
        Shader,
    },
    scene::{MaterialID, MeshID, ModelID, ShaderManager},
};
use anyhow::{anyhow, Result};
use std::{
    collections::{hash_map::Entry, HashMap},
    iter,
};

/// Manager and owner of render passes.
///
/// Holds a pass for clearing the rendering surface,
/// as well as a set of passes for rendering specific
/// models.
#[derive(Debug)]
pub struct RenderPassManager {
    clearing_pass_recorder: RenderPassRecorder,
    model_render_pass_recorders: HashMap<ModelID, RenderPassRecorder>,
}

/// Holds the information describing a specific render pass,
/// including identifiers to the data it involves.
#[derive(Clone, Debug)]
pub struct RenderPassSpecification {
    model_id: Option<ModelID>,
    mesh_id: Option<MeshID>,
    depth_test: bool,
    clear_color: Option<wgpu::Color>,
    clear_depth: Option<f32>,
    label: String,
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    vertex_attribute_requirements: VertexAttributeSet,
    pipeline: Option<wgpu::RenderPipeline>,
    color_load_operation: wgpu::LoadOp<wgpu::Color>,
    depth_operations: wgpu::Operations<f32>,
    disabled: bool,
}

struct BindGroupShaderInput<'a> {
    camera: Option<&'a CameraShaderInput>,
    light: Option<&'a LightShaderInput>,
    material: Option<&'a MaterialShaderInput>,
}

impl RenderPassManager {
    /// Creates a new manager with a pass that clears the
    /// surface with the given color.
    pub fn new(clear_color: wgpu::Color, clear_depth: f32) -> Self {
        Self {
            clearing_pass_recorder: RenderPassRecorder::clearing_pass(clear_color, clear_depth),
            model_render_pass_recorders: HashMap::new(),
        }
    }

    /// Returns an iterator over all render passes, starting with
    /// the clearing pass.
    pub fn recorders(&self) -> impl Iterator<Item = &RenderPassRecorder> {
        iter::once(&self.clearing_pass_recorder).chain(self.model_render_pass_recorders.values())
    }

    /// Returns an iterator over all render passes, excluding the
    /// clearing pass.
    pub fn recorders_no_clear(&self) -> impl Iterator<Item = &RenderPassRecorder> {
        self.model_render_pass_recorders.values()
    }

    /// Deletes the render pass recorder for every model.
    pub fn clear_model_render_pass_recorders(&mut self) {
        self.model_render_pass_recorders.clear();
    }

    /// Ensures that all render passes required for rendering the
    /// entities present in the given render resources with the given
    /// camera are available and configured correctly.
    ///
    /// Render passes whose entities are no longer present in the
    /// resources will be removed, and missing render passes for
    /// new entities will be created.
    fn sync_with_render_resources(
        &mut self,
        core_system: &CoreRenderingSystem,
        config: &RenderingConfig,
        render_resources: &SynchronizedRenderResources,
        shader_manager: &mut ShaderManager,
    ) -> Result<()> {
        let feature_buffer_managers = render_resources.instance_feature_buffer_managers();

        for (&model_id, feature_buffer_manager) in feature_buffer_managers {
            // Avoid rendering the model if there are no instances
            let disable_pass = feature_buffer_manager
                .first()
                .unwrap()
                .vertex_render_buffer()
                .is_empty();

            match self.model_render_pass_recorders.entry(model_id) {
                Entry::Vacant(entry) => {
                    entry.insert(Self::create_render_pass_recorder_for_model(
                        core_system,
                        config,
                        render_resources,
                        shader_manager,
                        model_id,
                        disable_pass,
                    )?);
                }
                Entry::Occupied(mut entry) => {
                    let recorder = entry.get_mut();
                    recorder.set_disabled(disable_pass);
                }
            }
        }

        self.model_render_pass_recorders
            .retain(|model_id, _| feature_buffer_managers.contains_key(model_id));

        Ok(())
    }

    fn create_render_pass_recorder_for_model(
        core_system: &CoreRenderingSystem,
        config: &RenderingConfig,
        render_resources: &SynchronizedRenderResources,
        shader_manager: &mut ShaderManager,
        model_id: ModelID,
        disabled: bool,
    ) -> Result<RenderPassRecorder> {
        let specification = RenderPassSpecification::for_model(model_id)?;
        RenderPassRecorder::new(
            core_system,
            config,
            render_resources,
            shader_manager,
            specification,
            disabled,
        )
    }
}

impl RenderPassSpecification {
    /// Creates the specification for the render pass that
    /// will render the model with the given ID.
    pub fn for_model(model_id: ModelID) -> Result<Self> {
        Ok(Self {
            model_id: Some(model_id),
            mesh_id: Some(model_id.mesh_id()),
            depth_test: true,
            clear_color: None,
            clear_depth: None,
            label: model_id.to_string(),
        })
    }

    /// Creates the specification for the render pass that will
    /// clear the rendering surface with the given color.
    pub fn clearing_pass(clear_color: wgpu::Color, clear_depth: f32) -> Self {
        Self {
            model_id: None,
            mesh_id: None,
            depth_test: true,
            clear_color: Some(clear_color),
            clear_depth: Some(clear_depth),
            label: "Clearing pass".to_string(),
        }
    }

    fn has_model(&self) -> bool {
        self.model_id.is_some()
    }

    /// Obtains the vertex buffer layouts for the required mesh vertex
    /// attributes and instance features involved in the render pass, as well as
    /// the associated shader inputs.
    ///
    /// The order of the layouts is:
    /// 1. Mesh vertex attribute buffers.
    /// 2. Instance feature buffers.
    fn get_vertex_buffer_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
        vertex_attribute_requirements: VertexAttributeSet,
    ) -> Result<(
        Vec<wgpu::VertexBufferLayout<'static>>,
        Option<&'a MeshShaderInput>,
        Vec<&'a InstanceFeatureShaderInput>,
    )> {
        let mut layouts = Vec::with_capacity(2);
        let mut mesh_shader_input = None;
        let mut instance_feature_shader_inputs = Vec::with_capacity(1);

        if let Some(mesh_id) = self.mesh_id {
            let mesh_buffer_manager = Self::get_mesh_buffer_manager(render_resources, mesh_id)?;

            layouts.extend(
                mesh_buffer_manager.request_vertex_buffer_layouts(vertex_attribute_requirements)?,
            );
            mesh_shader_input = Some(mesh_buffer_manager.shader_input());

            if let Some(model_id) = self.model_id {
                if let Some(buffers) =
                    render_resources.get_instance_feature_buffer_managers(model_id)
                {
                    for buffer in buffers {
                        layouts.push(buffer.vertex_buffer_layout().clone());
                        instance_feature_shader_inputs.push(buffer.shader_input());
                    }
                }
            }
        }

        Ok((layouts, mesh_shader_input, instance_feature_shader_inputs))
    }

    /// Obtains the bind group layouts for any camera, material or lights
    /// involved in the render pass, as well as the associated shader
    /// inputs and the vertex attribute requirements of the material.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Material textures.
    fn get_bind_group_layouts_and_shader_inputs<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<(
        Vec<&'a wgpu::BindGroupLayout>,
        BindGroupShaderInput<'a>,
        VertexAttributeSet,
    )> {
        let mut layouts = Vec::with_capacity(3);

        let mut shader_input = BindGroupShaderInput {
            camera: None,
            light: None,
            material: None,
        };
        let mut vertex_attribute_requirements = VertexAttributeSet::empty();

        if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
            layouts.push(camera_buffer_manager.bind_group_layout());
            shader_input.camera = Some(CameraRenderBufferManager::shader_input());
        }

        if let Some(light_buffer_manager) = render_resources.get_light_buffer_manager() {
            layouts.push(light_buffer_manager.bind_group_layout());
            shader_input.light = Some(light_buffer_manager.shader_input());
        }

        if let Some(model_id) = self.model_id {
            let material_resource_manager =
                Self::get_material_resource_manager(render_resources, model_id.material_id())?;

            if let Some(layout) = material_resource_manager.texture_bind_group_layout() {
                layouts.push(layout);
            }
            shader_input.material = Some(material_resource_manager.shader_input());

            vertex_attribute_requirements =
                material_resource_manager.vertex_attribute_requirements();
        }

        Ok((layouts, shader_input, vertex_attribute_requirements))
    }

    /// Obtains all bind groups involved in the render pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Lights.
    /// 3. Material textures.
    fn get_bind_groups<'a>(
        &self,
        render_resources: &'a SynchronizedRenderResources,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let mut bind_groups = Vec::with_capacity(3);

        if let Some(camera_buffer_manager) = render_resources.get_camera_buffer_manager() {
            bind_groups.push(camera_buffer_manager.bind_group());
        }

        if let Some(light_buffer_manager) = render_resources.get_light_buffer_manager() {
            bind_groups.push(light_buffer_manager.bind_group());
        }

        if let Some(model_id) = self.model_id {
            if let Some(bind_group) =
                Self::get_material_resource_manager(render_resources, model_id.material_id())?
                    .texture_bind_group()
            {
                bind_groups.push(bind_group);
            }
        }

        Ok(bind_groups)
    }

    fn determine_color_load_operation(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some(clear_color) => wgpu::LoadOp::Clear(clear_color),
            None => wgpu::LoadOp::Load,
        }
    }

    fn determine_depth_loperations(&self) -> wgpu::Operations<f32> {
        if let Some(clear_depth) = self.clear_depth {
            wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_depth),
                store: true,
            }
        } else {
            wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            }
        }
    }

    fn get_mesh_buffer_manager(
        render_resources: &SynchronizedRenderResources,
        mesh_id: MeshID,
    ) -> Result<&MeshRenderBufferManager> {
        render_resources
            .get_mesh_buffer_manager(mesh_id)
            .ok_or_else(|| anyhow!("Missing render buffer for mesh {}", mesh_id))
    }

    fn get_instance_feature_buffer_managers(
        render_resources: &SynchronizedRenderResources,
        model_id: ModelID,
    ) -> Result<impl Iterator<Item = &InstanceFeatureRenderBufferManager>> {
        render_resources
            .get_instance_feature_buffer_managers(model_id)
            .map(|buffers| buffers.iter())
            .ok_or_else(|| anyhow!("Missing instance render buffer for model {}", model_id))
    }

    fn get_material_resource_manager(
        render_resources: &SynchronizedRenderResources,
        material_id: MaterialID,
    ) -> Result<&MaterialRenderResourceManager> {
        render_resources
            .get_material_resource_manager(material_id)
            .ok_or_else(|| anyhow!("Missing resource manager for material {}", material_id))
    }
}

impl RenderPassRecorder {
    /// Creates a new recorder for the render pass defined by
    /// the given specification.
    ///
    /// Shader inputs extracted from the specification are used
    /// to build or fetch the appropriate shader.
    pub fn new(
        core_system: &CoreRenderingSystem,
        config: &RenderingConfig,
        render_resources: &SynchronizedRenderResources,
        shader_manager: &mut ShaderManager,
        specification: RenderPassSpecification,
        disabled: bool,
    ) -> Result<Self> {
        let (pipeline, vertex_attribute_requirements) = if specification.has_model() {
            let (bind_group_layouts, bind_group_shader_input, vertex_attribute_requirements) =
                specification.get_bind_group_layouts_and_shader_inputs(render_resources)?;

            let (vertex_buffer_layouts, mesh_shader_input, instance_feature_shader_inputs) =
                specification.get_vertex_buffer_layouts_and_shader_inputs(
                    render_resources,
                    vertex_attribute_requirements,
                )?;

            let shader = shader_manager.obtain_shader(
                core_system,
                bind_group_shader_input.camera,
                mesh_shader_input,
                bind_group_shader_input.light,
                &instance_feature_shader_inputs,
                bind_group_shader_input.material,
                vertex_attribute_requirements,
            )?;

            let pipeline_layout = Self::create_render_pipeline_layout(
                core_system.device(),
                &bind_group_layouts,
                &format!("{} render pipeline layout", &specification.label),
            );

            let depth_texture_format = if specification.depth_test {
                Some(DepthTexture::FORMAT)
            } else {
                None
            };

            let pipeline = Some(Self::create_render_pipeline(
                core_system.device(),
                &pipeline_layout,
                shader,
                &vertex_buffer_layouts,
                core_system.surface_config().format,
                depth_texture_format,
                config,
                &format!("{} render pipeline", &specification.label),
            ));

            (pipeline, vertex_attribute_requirements)
        } else {
            // If we don't have vertices and a material we don't need a pipeline
            (None, VertexAttributeSet::empty())
        };

        let color_load_operation = specification.determine_color_load_operation();
        let depth_operations = specification.determine_depth_loperations();

        Ok(Self {
            specification,
            vertex_attribute_requirements,
            pipeline,
            color_load_operation,
            depth_operations,
            disabled,
        })
    }

    pub fn clearing_pass(clear_color: wgpu::Color, clear_depth: f32) -> Self {
        let specification = RenderPassSpecification::clearing_pass(clear_color, clear_depth);
        let color_load_operation = specification.determine_color_load_operation();
        let depth_operations = specification.determine_depth_loperations();
        Self {
            specification,
            vertex_attribute_requirements: VertexAttributeSet::empty(),
            pipeline: None,
            color_load_operation,
            depth_operations,
            disabled: false,
        }
    }

    /// Records the render pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the render resources
    /// used in this render pass are no longer available.
    pub fn record_render_pass(
        &self,
        render_resources: &SynchronizedRenderResources,
        surface_texture_view: &wgpu::TextureView,
        depth_texture: &DepthTexture,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if self.disabled() {
            return Ok(());
        }

        // Make sure all data is available before doing anything else
        let bind_groups = self.specification.get_bind_groups(render_resources)?;

        let mesh_buffer_manager = match self.specification.mesh_id {
            Some(mesh_id) => Some(RenderPassSpecification::get_mesh_buffer_manager(
                render_resources,
                mesh_id,
            )?),
            _ => None,
        };

        let feature_buffer_managers = match self.specification.model_id {
            Some(model_id) => Some(
                RenderPassSpecification::get_instance_feature_buffer_managers(
                    render_resources,
                    model_id,
                )?,
            ),
            _ => None,
        };

        let depth_texure_view = if self.specification.depth_test {
            Some(depth_texture.view())
        } else {
            None
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `[[location(i)]]` directive in the fragment shader output targets color attachment `i` here
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: self.color_load_operation,
                    store: true,
                },
            })],
            depth_stencil_attachment: depth_texure_view.map(|depth_texure_view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texure_view,
                    depth_ops: Some(self.depth_operations),
                    stencil_ops: None,
                }
            }),
            label: Some(&self.specification.label),
        });

        if let Some(ref pipeline) = self.pipeline {
            let mesh_buffer_manager = mesh_buffer_manager.expect("Has pipeline but no vertices");

            render_pass.set_pipeline(pipeline);

            for (index, &bind_group) in bind_groups.iter().enumerate() {
                render_pass.set_bind_group(u32::try_from(index).unwrap(), bind_group, &[]);
            }

            let mut vertex_buffer_slot = 0;

            for vertex_buffer in mesh_buffer_manager
                .request_vertex_render_buffers(self.vertex_attribute_requirements)?
            {
                render_pass
                    .set_vertex_buffer(vertex_buffer_slot, vertex_buffer.valid_buffer_slice());

                vertex_buffer_slot += 1;
            }

            let n_instances = if let Some(feature_buffer_managers) = feature_buffer_managers {
                let mut n_instances = 0;

                for feature_buffer_manager in feature_buffer_managers {
                    render_pass.set_vertex_buffer(
                        vertex_buffer_slot,
                        feature_buffer_manager
                            .vertex_render_buffer()
                            .valid_buffer_slice(),
                    );

                    n_instances = feature_buffer_manager.n_features();

                    vertex_buffer_slot += 1;
                }
                n_instances
            } else {
                1
            };

            render_pass.set_index_buffer(
                mesh_buffer_manager
                    .index_render_buffer()
                    .valid_buffer_slice(),
                mesh_buffer_manager.index_format(),
            );

            render_pass.draw_indexed(
                0..u32::try_from(mesh_buffer_manager.n_indices()).unwrap(),
                0,
                0..u32::try_from(n_instances).unwrap(),
            );
        }

        Ok(())
    }

    /// Whether the render pass should be skipped.
    pub fn disabled(&self) -> bool {
        self.disabled
    }

    /// Set whether the render pass should be skipped.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    fn create_render_pipeline_layout(
        device: &wgpu::Device,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        label: &str,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts,
            push_constant_ranges: &[],
            label: Some(label),
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &Shader,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        surface_texture_format: wgpu::TextureFormat,
        depth_texture_format: Option<wgpu::TextureFormat>,
        config: &RenderingConfig,
        label: &str,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader.vertex_module(),
                entry_point: shader.vertex_entry_point_name(), // Vertex shader function in shader file
                buffers: vertex_buffer_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: shader.fragment_module(),
                entry_point: shader.fragment_entry_point_name(), // Fragment shader function in shader file
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // Because we flip the x-axis of the projection matrix in order
                // to avoid a projected view that is mirrored compared to world
                // coordinates, the face orientations become reversed in
                // framebuffer space compared to world space. We therefore
                // define front faces as having clockwise winding order in
                // framebuffer space, which corresponds to anti-clockwise
                // winding order in world space.
                front_face: wgpu::FrontFace::Cw,
                cull_mode: config.cull_mode,
                polygon_mode: config.polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_texture_format.map(|depth_texture_format| {
                wgpu::DepthStencilState {
                    format: depth_texture_format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            label: Some(label),
        })
    }
}
