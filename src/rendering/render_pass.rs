//! Rendering pipelines.

use crate::geometry::GeomIdent;

use super::{
    asset::{AssetIdent, Assets},
    buffer::{IndexBuffer, InstanceBuffer, VertexBuffer},
    world::RenderData,
    CoreRenderingSystem,
};
use anyhow::{anyhow, Result};

/// Holds the information describing a specific render pass,
/// including identifiers to the data it involves.
#[derive(Clone, Debug)]
pub struct RenderPassSpecification {
    shader: Option<AssetIdent>,
    image_textures: Vec<AssetIdent>,
    camera: Option<GeomIdent>,
    mesh: Option<GeomIdent>,
    mesh_instances: Option<GeomIdent>,
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

impl RenderPassSpecification {
    /// Creates a new empty render pass descriptor.
    pub fn new(label: String) -> Self {
        Self {
            shader: None,
            image_textures: Vec::new(),
            camera: None,
            mesh: None,
            mesh_instances: None,
            clear_color: None,
            label,
        }
    }

    /// Uses the given shader for the render pass.
    ///
    /// Without a shader the render pass will only involve
    /// a clear or load operation on the rendering surface.
    pub fn with_shader(mut self, shader: Option<AssetIdent>) -> Self {
        self.shader = shader;
        self
    }

    /// Includes the given image textures in the render pass.
    pub fn with_image_textures(mut self, image_textures: Vec<AssetIdent>) -> Self {
        self.image_textures = image_textures;
        self
    }

    /// Includes the given image texture in the render pass.
    pub fn add_image_texture(mut self, image_texture: AssetIdent) -> Self {
        self.image_textures.push(image_texture);
        self
    }

    /// Uses the given camera for the render pass.
    pub fn with_camera(mut self, camera: Option<GeomIdent>) -> Self {
        self.camera = camera;
        self
    }

    /// Uses the given mesh for the render pass.
    ///
    /// Without a mesh the render pass will only involve
    /// a clear or load operation on the rendering surface.
    pub fn with_mesh(mut self, mesh: Option<GeomIdent>) -> Self {
        self.mesh = mesh;
        self
    }

    /// Includes the given instances of the included mesh for
    /// rendering in the render pass.
    ///
    /// Without a mesh this will do nothing.
    pub fn with_mesh_instances(mut self, mesh_instances: Option<GeomIdent>) -> Self {
        self.mesh_instances = mesh_instances;
        self
    }

    /// Uses the given color to clear the rendering surface before
    /// performing the render pass.
    ///
    /// If not specified a load operation will be performed instead.
    pub fn with_clear_color(mut self, clear_color: Option<wgpu::Color>) -> Self {
        self.clear_color = clear_color;
        self
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
        render_data: &'a RenderData,
    ) -> Result<Vec<&'a wgpu::BindGroupLayout>> {
        let mut layouts;
        if let Some(ref camera) = self.camera {
            layouts = Vec::with_capacity(self.image_textures.len() + 1);
            layouts.push(
                render_data
                    .get_camera_data(camera)
                    .ok_or_else(|| anyhow!("Camera {} missing from render data", camera))?
                    .bind_group_layout(),
            );
        } else {
            layouts = Vec::with_capacity(self.image_textures.len());
        }
        for image_texture in &self.image_textures {
            layouts.push(
                assets
                    .image_textures
                    .get(image_texture)
                    .ok_or_else(|| anyhow!("Image texture {} missing from assets", image_texture))?
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
        render_data: &'a RenderData,
    ) -> Result<Vec<&'a wgpu::BindGroup>> {
        let mut layouts;
        if let Some(ref camera) = self.camera {
            layouts = Vec::with_capacity(self.image_textures.len() + 1);
            layouts.push(
                render_data
                    .get_camera_data(camera)
                    .ok_or_else(|| anyhow!("Camera {} missing from render data", camera))?
                    .bind_group(),
            );
        } else {
            layouts = Vec::with_capacity(self.image_textures.len());
        }
        for image_texture in &self.image_textures {
            layouts.push(
                assets
                    .image_textures
                    .get(image_texture)
                    .ok_or_else(|| anyhow!("Image texture {} missing from assets", image_texture))?
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
        render_data: &'a RenderData,
    ) -> Result<Vec<wgpu::VertexBufferLayout<'static>>> {
        let mut layouts = Vec::with_capacity(2);
        if let Some(ref mesh) = self.mesh {
            layouts.push(
                render_data
                    .get_mesh_data(mesh)
                    .ok_or_else(|| anyhow!("Mesh {} missing from render data", mesh))?
                    .vertex_buffer()
                    .layout()
                    .clone(),
            );
        }
        if let Some(ref mesh_instances) = self.mesh_instances {
            layouts.push(
                render_data
                    .get_mesh_instance_data(mesh_instances)
                    .ok_or_else(|| {
                        anyhow!(
                            "Mesh instance group {} missing from render data",
                            mesh_instances
                        )
                    })?
                    .instance_buffer()
                    .layout()
                    .clone(),
            );
        }
        Ok(layouts)
    }

    fn determine_load_operation(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self.clear_color {
            Some(clear_color) => wgpu::LoadOp::Clear(clear_color),
            None => wgpu::LoadOp::Load,
        }
    }

    fn get_shader_module<'a>(
        assets: &'a Assets,
        shader: &AssetIdent,
    ) -> Result<&'a wgpu::ShaderModule> {
        assets
            .shaders
            .get(shader)
            .map(|shader| shader.module())
            .ok_or_else(|| anyhow!("Shader {} missing from assets", shader))
    }

    fn get_mesh_buffers<'a>(
        render_data: &'a RenderData,
        mesh: &GeomIdent,
    ) -> Result<(&'a VertexBuffer, &'a IndexBuffer)> {
        render_data
            .get_mesh_data(mesh)
            .map(|mesh_data| (mesh_data.vertex_buffer(), mesh_data.index_buffer()))
            .ok_or_else(|| anyhow!("Mesh {} missing from render data", mesh))
    }

    fn get_mesh_instance_buffer<'a>(
        render_data: &'a RenderData,
        mesh_instances: &GeomIdent,
    ) -> Result<&'a InstanceBuffer> {
        render_data
            .get_mesh_instance_data(mesh_instances)
            .map(|mesh_instance_data| mesh_instance_data.instance_buffer())
            .ok_or_else(|| {
                anyhow!(
                    "Mesh instance group {} missing from render data",
                    mesh_instances
                )
            })
    }
}

impl RenderPassRecorder {
    /// Creates a new recorder for the render pass defined by
    /// the given specification.
    pub fn new(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        render_data: &RenderData,
        specification: RenderPassSpecification,
    ) -> Result<Self> {
        let vertex_buffer_layouts = specification.get_vertex_buffer_layouts(render_data)?;

        let pipeline = if vertex_buffer_layouts.is_empty() || specification.shader.is_none() {
            // If we don't have vertices and a shader we don't need a pipeline
            None
        } else {
            let shader_module = RenderPassSpecification::get_shader_module(
                assets,
                specification.shader.as_ref().unwrap(),
            )?;

            let bind_group_layouts = specification.get_bind_group_layouts(assets, render_data)?;

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

    /// Records the render pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the assets or render data used
    /// in this render pass is no longer available.
    pub fn record_render_pass(
        &self,
        assets: &Assets,
        render_data: &RenderData,
        view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        // Make sure all data is available before doing anything else
        let bind_groups = self.specification.get_bind_groups(assets, render_data)?;
        let mesh_buffers = match self.specification.mesh {
            Some(ref mesh) => Some(RenderPassSpecification::get_mesh_buffers(
                render_data,
                mesh,
            )?),
            _ => None,
        };
        let mesh_instance_buffer = match self.specification.mesh_instances {
            Some(ref mesh_instances) => Some(RenderPassSpecification::get_mesh_instance_buffer(
                render_data,
                mesh_instances,
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

            let n_instances = if let Some(instance_buffer) = mesh_instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.buffer().slice(..));
                instance_buffer.n_instances()
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
