//! Rendering pipelines.

mod tasks;

use crate::{
    geometry::{CameraID, MeshID, ModelID, ModelInstance},
    rendering::{
        buffer::{BufferableVertex, IndexBuffer, InstanceBuffer, VertexBuffer},
        buffer_sync::SynchronizedRenderBuffers,
        Assets, CoreRenderingSystem, ModelLibrary, ShaderID, TextureID,
    },
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
    camera_id: Option<CameraID>,
    clearing_pass_recorder: RenderPassRecorder,
    model_render_pass_recorders: HashMap<ModelID, RenderPassRecorder>,
}

/// Holds the information describing a specific render pass,
/// including identifiers to the data it involves.
#[derive(Clone, Debug)]
pub struct RenderPassSpecification {
    shader_id: Option<ShaderID>,
    image_texture_ids: Vec<TextureID>,
    camera_id: Option<CameraID>,
    mesh_id: Option<MeshID>,
    model_id: Option<ModelID>,
    clear_color: Option<wgpu::Color>,
    label: String,
}

/// Recorder for a specific render pass.
#[derive(Debug)]
pub struct RenderPassRecorder {
    specification: RenderPassSpecification,
    pipeline: Option<wgpu::RenderPipeline>,
    load_operation: wgpu::LoadOp<wgpu::Color>,
}

impl RenderPassManager {
    /// Creates a new manager with a pass that clears the
    /// surface with the given color.
    ///
    /// Render passes created for models by this manager will
    /// use the camera with the given ID if specified, otherwise
    /// the models will be rendered without a camera.
    pub fn new(clear_color: wgpu::Color, camera_id: Option<CameraID>) -> Self {
        Self {
            camera_id,
            clearing_pass_recorder: RenderPassRecorder::clearing_pass(clear_color),
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

    /// Ensures that all render passes required for rendering the
    /// entities present in the given render buffers exist.
    ///
    /// Render passes whose entities are no longer present in the
    /// buffers will be removed, and missing render passes for
    /// new entities will be created.
    fn sync_with_render_buffers(
        &mut self,
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        model_library: &ModelLibrary,
        render_buffers: &SynchronizedRenderBuffers,
    ) -> Result<()> {
        let model_instance_buffers = render_buffers.model_instance_buffers();

        for &model_id in model_instance_buffers.keys() {
            if let Entry::Vacant(entry) = self.model_render_pass_recorders.entry(model_id) {
                entry.insert(Self::create_render_pass_recorder_for_model(
                    core_system,
                    assets,
                    model_library,
                    render_buffers,
                    self.camera_id,
                    model_id,
                )?);
            }
        }

        self.model_render_pass_recorders
            .retain(|model_id, _| model_instance_buffers.contains_key(model_id));

        Ok(())
    }

    fn create_render_pass_recorder_for_model(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        model_library: &ModelLibrary,
        render_buffers: &SynchronizedRenderBuffers,
        camera_id: Option<CameraID>,
        model_id: ModelID,
    ) -> Result<RenderPassRecorder> {
        let specification = RenderPassSpecification::for_model(model_library, camera_id, model_id)?;
        RenderPassRecorder::new(core_system, assets, render_buffers, specification)
    }
}

impl RenderPassSpecification {
    /// Creates a new empty render pass specification.
    pub fn new(label: String) -> Self {
        Self {
            shader_id: None,
            image_texture_ids: Vec::new(),
            mesh_id: None,
            camera_id: None,
            model_id: None,
            clear_color: None,
            label,
        }
    }

    /// Creates the specification for the render pass that
    /// will render the model with the given ID, using the
    /// camera with the given ID if specified.
    pub fn for_model(
        model_library: &ModelLibrary,
        camera_id: Option<CameraID>,
        model_id: ModelID,
    ) -> Result<Self> {
        let model_spec = model_library
            .get_model(model_id)
            .ok_or_else(|| anyhow!("Model {} missing from model library", model_id))?;

        let material_spec = model_library
            .material_library()
            .get_material(model_spec.material_id)
            .ok_or_else(|| {
                anyhow!(
                    "Material {} missing from material library",
                    model_spec.material_id
                )
            })?;

        Ok(Self {
            shader_id: Some(material_spec.shader_id),
            image_texture_ids: material_spec.image_texture_ids.clone(),
            camera_id,
            mesh_id: Some(model_spec.mesh_id),
            model_id: Some(model_id),
            clear_color: None,
            label: model_id.to_string(),
        })
    }

    /// Creates the specification for the render pass that will
    /// clear the rendering surface with the given color.
    pub fn clearing_pass(clear_color: wgpu::Color) -> Self {
        Self {
            shader_id: None,
            image_texture_ids: Vec::new(),
            camera_id: None,
            mesh_id: None,
            model_id: None,
            clear_color: Some(clear_color),
            label: "Clearing pass".to_string(),
        }
    }

    /// Obtains the layouts of all bind groups involved in the render
    /// pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Image textures in same order as in the `image_textures` vector.
    fn get_bind_group_layouts<'a>(
        &self,
        assets: &'a Assets,
        render_buffers: &'a SynchronizedRenderBuffers,
    ) -> Result<Vec<&'a wgpu::BindGroupLayout>> {
        let mut layouts;
        if let Some(camera_id) = self.camera_id {
            layouts = Vec::with_capacity(self.image_texture_ids.len() + 1);
            layouts.push(
                render_buffers
                    .get_camera_buffer(camera_id)
                    .ok_or_else(|| anyhow!("Missing render buffer for camera {}", camera_id))?
                    .bind_group_layout(),
            );
        } else {
            layouts = Vec::with_capacity(self.image_texture_ids.len());
        }
        for image_texture_id in &self.image_texture_ids {
            layouts.push(
                assets
                    .image_textures
                    .get(image_texture_id)
                    .ok_or_else(|| {
                        anyhow!("Image texture {} missing from assets", image_texture_id)
                    })?
                    .bind_group_layout(),
            );
        }
        Ok(layouts)
    }

    /// Obtains all bind groups involved in the render pass.
    ///
    /// The order of the bind groups is:
    /// 1. Camera.
    /// 2. Image textures in same order as in the `image_textures` vector.
    fn get_bind_groups<'a>(
        &self,
        assets: &'a Assets,
        render_buffers: &'a SynchronizedRenderBuffers,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let mut layouts;
        if let Some(camera_id) = self.camera_id {
            layouts = Vec::with_capacity(self.image_texture_ids.len() + 1);
            layouts.push(
                render_buffers
                    .get_camera_buffer(camera_id)
                    .ok_or_else(|| anyhow!("Missing render buffer for camera {}", camera_id))?
                    .bind_group(),
            );
        } else {
            layouts = Vec::with_capacity(self.image_texture_ids.len());
        }
        for image_texture_id in &self.image_texture_ids {
            layouts.push(
                assets
                    .image_textures
                    .get(image_texture_id)
                    .ok_or_else(|| {
                        anyhow!("Image texture {} missing from assets", image_texture_id)
                    })?
                    .bind_group(),
            );
        }
        Ok(layouts)
    }

    /// Obtains the layout of all vertex buffers involved in the render pass.
    ///
    /// The order of the layouts is:
    /// 1. Mesh vertex buffer.
    /// 2. Mesh instance buffer.
    fn get_vertex_buffer_layouts<'a>(
        &self,
        render_buffers: &'a SynchronizedRenderBuffers,
    ) -> Result<Vec<wgpu::VertexBufferLayout<'static>>> {
        let mut layouts = Vec::with_capacity(2);
        if let Some(mesh_id) = self.mesh_id {
            layouts.push(
                render_buffers
                    .get_mesh_buffer(mesh_id)
                    .ok_or_else(|| anyhow!("Missing render buffer for mesh {}", mesh_id))?
                    .vertex_buffer()
                    .layout()
                    .clone(),
            );
            // Assume that we have model instances if we have a model ID
            if self.model_id.is_some() {
                layouts.push(ModelInstance::<f32>::BUFFER_LAYOUT);
            }
        }
        Ok(layouts)
    }

    fn determine_load_operation(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some(clear_color) => wgpu::LoadOp::Clear(clear_color),
            None => wgpu::LoadOp::Load,
        }
    }

    fn get_shader_module(assets: &Assets, shader_id: ShaderID) -> Result<&wgpu::ShaderModule> {
        assets
            .shaders
            .get(&shader_id)
            .map(|shader| shader.module())
            .ok_or_else(|| anyhow!("Shader {} missing from assets", shader_id))
    }

    fn get_mesh_buffers(
        render_buffers: &SynchronizedRenderBuffers,
        mesh_id: MeshID,
    ) -> Result<(&VertexBuffer, &IndexBuffer)> {
        let (vertex_buffer, index_buffer) = render_buffers
            .get_mesh_buffer(mesh_id)
            .map(|mesh_data| (mesh_data.vertex_buffer(), mesh_data.index_buffer()))
            .ok_or_else(|| anyhow!("Missing render buffer for mesh {}", mesh_id))?;

        Ok((vertex_buffer, index_buffer))
    }

    fn get_model_instance_buffer(
        render_buffers: &SynchronizedRenderBuffers,
        model_id: ModelID,
    ) -> Result<&InstanceBuffer> {
        render_buffers
            .get_model_instance_buffer(model_id)
            .map(|instance_data| instance_data.instance_buffer())
            .ok_or_else(|| anyhow!("Missing instance render buffer for model {}", model_id))
    }
}

impl RenderPassRecorder {
    /// Creates a new recorder for the render pass defined by
    /// the given specification.
    pub fn new(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        render_buffers: &SynchronizedRenderBuffers,
        specification: RenderPassSpecification,
    ) -> Result<Self> {
        let vertex_buffer_layouts = specification.get_vertex_buffer_layouts(render_buffers)?;

        let pipeline = if vertex_buffer_layouts.is_empty() || specification.shader_id.is_none() {
            // If we don't have vertices and a shader we don't need a pipeline
            None
        } else {
            let shader_module = RenderPassSpecification::get_shader_module(
                assets,
                specification.shader_id.unwrap(),
            )?;

            let bind_group_layouts =
                specification.get_bind_group_layouts(assets, render_buffers)?;

            let pipeline_layout = Self::create_render_pipeline_layout(
                core_system.device(),
                &bind_group_layouts,
                &format!("{} render pipeline layout", &specification.label),
            );

            Some(Self::create_render_pipeline(
                core_system.device(),
                &pipeline_layout,
                shader_module,
                &vertex_buffer_layouts,
                core_system.surface_config().format,
                &format!("{} render pipeline", &specification.label),
            ))
        };

        let load_operation = specification.determine_load_operation();

        Ok(Self {
            specification,
            pipeline,
            load_operation,
        })
    }

    pub fn clearing_pass(clear_color: wgpu::Color) -> Self {
        let specification = RenderPassSpecification::clearing_pass(clear_color);
        let load_operation = specification.determine_load_operation();
        Self {
            specification,
            pipeline: None,
            load_operation,
        }
    }

    /// Records the render pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the assets or render buffers
    /// used in this render pass are no longer available.
    pub fn record_render_pass(
        &self,
        assets: &Assets,
        render_buffers: &SynchronizedRenderBuffers,
        view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        // Make sure all data is available before doing anything else
        let bind_groups = self.specification.get_bind_groups(assets, render_buffers)?;

        let mesh_buffers = match self.specification.mesh_id {
            Some(mesh_id) => Some(RenderPassSpecification::get_mesh_buffers(
                render_buffers,
                mesh_id,
            )?),
            _ => None,
        };

        let instance_buffer = match self.specification.model_id {
            Some(model_id) => Some(RenderPassSpecification::get_model_instance_buffer(
                render_buffers,
                model_id,
            )?),
            _ => None,
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `[[location(i)]]` directive in the fragment shader output targets color attachment `i` here
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: self.load_operation,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            label: Some(&self.specification.label),
        });

        if let Some(ref pipeline) = self.pipeline {
            let (vertex_buffer, index_buffer) = mesh_buffers.expect("Has pipeline but no vertices");

            render_pass.set_pipeline(pipeline);

            for (index, &bind_group) in bind_groups.iter().enumerate() {
                render_pass.set_bind_group(u32::try_from(index).unwrap(), bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, vertex_buffer.buffer().slice(..));

            let n_instances = if let Some(instance_buffer) = instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.buffer().slice(..));
                instance_buffer.n_valid_instances()
            } else {
                1
            };

            render_pass.set_index_buffer(index_buffer.buffer().slice(..), index_buffer.format());

            render_pass.draw_indexed(0..index_buffer.n_indices(), 0, 0..n_instances);
        }

        Ok(())
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
        shader_module: &wgpu::ShaderModule,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'_>],
        texture_format: wgpu::TextureFormat,
        label: &str,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: "vs_main", // Vertex shader function in shader file
                buffers: vertex_buffer_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: "fs_main", // Fragment shader function in shader file
                targets: &[wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
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
