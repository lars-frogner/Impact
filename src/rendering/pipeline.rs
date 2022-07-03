//! Rendering pipelines.

use super::{CoreRenderingSystem, ImageTexture, IndexBuffer, Shader, VertexBuffer};
use anyhow::Result;
use std::{collections::HashMap, rc::Rc};

/// Builder for producing a `RenderingPipeline`.
pub struct RenderingPipelineBuilder<'a> {
    core_system: &'a CoreRenderingSystem,
    shader: &'a Shader,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    bind_group_layout_map: HashMap<&'static str, usize>,
    bind_groups: Vec<wgpu::BindGroup>,
    vertex_buffers: Vec<Rc<VertexBuffer>>,
    index_buffer: Option<Rc<IndexBuffer>>,
    n_vertices: u32,
    label: String,
}

/// Pipeline and associated data required for performing
/// one or more render passes.
pub struct RenderingPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_groups: Vec<wgpu::BindGroup>,
    vertex_buffers: Vec<Rc<VertexBuffer>>,
    index_buffer: Option<Rc<IndexBuffer>>,
    n_vertices: u32,
    clear_color: wgpu::Color,
}

impl<'a> RenderingPipelineBuilder<'a> {
    /// Creates a new builder for a rendering pipeline.
    pub fn new(core_system: &'a CoreRenderingSystem, shader: &'a Shader, label: String) -> Self {
        Self {
            core_system,
            shader,
            bind_group_layouts: Vec::new(),
            bind_group_layout_map: HashMap::new(),
            bind_groups: Vec::new(),
            vertex_buffers: Vec::new(),
            index_buffer: None,
            n_vertices: 0,
            label,
        }
    }

    /// Adds the given image texture to the pipeline.
    pub fn add_image_texture(mut self, texture: &ImageTexture) -> Self {
        // Check if an appropriate layout already exists
        let layout_idx = *self
            .bind_group_layout_map
            .entry("ImageTexture")
            .or_insert_with(|| {
                // Since no layout exists for this texture type, we create the
                // layout and register it in the layout map
                let next_idx = self.bind_group_layouts.len();
                self.bind_group_layouts
                    .push(ImageTexture::create_bind_group_layout(
                        self.core_system.device(),
                    ));
                next_idx
            });

        // Create and add the bind group for this texture
        self.bind_groups.push(ImageTexture::create_bind_group(
            self.core_system.device(),
            &self.bind_group_layouts[layout_idx],
            texture,
        ));

        self
    }

    /// Adds the given buffer of vertices to the pipeline.
    pub fn add_vertex_buffer(mut self, vertex_buffer: Rc<VertexBuffer>) -> Self {
        self.n_vertices += vertex_buffer.n_vertices();
        self.vertex_buffers.push(vertex_buffer);
        self
    }

    /// Specifies the given buffer of indices for the pipeline.
    pub fn with_index_buffer(mut self, index_buffer: Rc<IndexBuffer>) -> Self {
        self.index_buffer = Some(index_buffer);
        self
    }

    /// Creates the `RenderingPipeline`.
    pub fn build(self) -> RenderingPipeline {
        let RenderingPipelineBuilder {
            core_system,
            shader,
            bind_group_layouts,
            bind_group_layout_map: _,
            bind_groups,
            vertex_buffers,
            index_buffer,
            n_vertices,
            label,
        } = self;

        let render_pipeline_layout = Self::create_render_pipeline_layout(
            core_system.device(),
            &bind_group_layouts.iter().collect::<Vec<_>>(),
            format!("{} render pipeline layout", &label).as_str(),
        );

        let vertex_buffer_layouts = vertex_buffers
            .iter()
            .map(|buffer| buffer.layout().clone())
            .collect::<Vec<_>>();

        let render_pipeline = Self::create_render_pipeline(
            core_system.device(),
            &render_pipeline_layout,
            shader.module(),
            &vertex_buffer_layouts,
            core_system.surface_config().format,
            format!("{} render pipeline", &label).as_str(),
        );

        RenderingPipeline::new(
            render_pipeline,
            bind_groups,
            vertex_buffers,
            index_buffer,
            n_vertices,
        )
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
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout],
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

impl RenderingPipeline {
    fn new(
        render_pipeline: wgpu::RenderPipeline,
        bind_groups: Vec<wgpu::BindGroup>,
        vertex_buffers: Vec<Rc<VertexBuffer>>,
        index_buffer: Option<Rc<IndexBuffer>>,
        n_vertices: u32,
    ) -> Self {
        Self {
            render_pipeline,
            bind_groups,
            vertex_buffers,
            index_buffer,
            n_vertices,
            clear_color: wgpu::Color::BLACK,
        }
    }

    /// Records the render passes for this pipeline to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if a bind group index or a vertex buffer slot can not
    /// be converted to `u32`.
    pub fn record_render_passes(
        &self,
        view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            // A `[[location(i)]]` directive in the fragment shader output targets color attachment `i` here
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            label: Some("Render pass"),
        });
        render_pass.set_pipeline(&self.render_pipeline);

        for (index, bind_group) in self.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(u32::try_from(index)?, bind_group, &[]);
        }

        for (slot, vertex_buffer) in self.vertex_buffers.iter().enumerate() {
            render_pass.set_vertex_buffer(u32::try_from(slot)?, vertex_buffer.buffer().slice(..));
        }

        if let Some(index_buffer) = &self.index_buffer {
            render_pass.set_index_buffer(index_buffer.buffer().slice(..), index_buffer.format());
            render_pass.draw_indexed(0..index_buffer.n_indices(), 0, 0..1);
        } else {
            render_pass.draw(0..self.n_vertices, 0..1);
        }
        Ok(())
    }

    pub fn set_clear_color(&mut self, color: wgpu::Color) {
        self.clear_color = color;
    }
}
