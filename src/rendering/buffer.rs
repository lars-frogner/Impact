//! Data buffers for rendering.

mod camera;
mod vertex;

use super::CoreRenderingSystem;
use anyhow::Result;
use bytemuck::Pod;
use wgpu::util::DeviceExt;

/// Represents vertex types that can be buffered to create
/// a vertex buffer.
pub trait BufferableVertex: Pod {
    /// Returns a `wgpu::VertexBufferLayout` describing the layout
    /// of buffers made up of this vertex type.
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static>;
}

/// Represents index types that can be buffered to create
/// an index buffer.
pub trait BufferableIndex: Pod {
    /// Returns the data format of this index type.
    fn index_format() -> wgpu::IndexFormat;
}

/// Represents uniforms that can be buffered and passed
/// to a shader.
pub trait BufferableUniform: Pod {
    /// Returns a descriptor for the layout of the uniform's
    /// bind group.
    fn bind_group_layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static>;
}

/// A buffer containing vertices.
pub struct VertexBuffer {
    layout: wgpu::VertexBufferLayout<'static>,
    buffer: wgpu::Buffer,
    n_vertices: u32,
}

/// A buffer containing vertex indices.
pub struct IndexBuffer {
    format: wgpu::IndexFormat,
    buffer: wgpu::Buffer,
    n_indices: u32,
}

/// A buffer containing uniforms.
pub struct UniformBuffer {
    bind_group_layout_descriptor: wgpu::BindGroupLayoutDescriptor<'static>,
    bind_group_label: String,
    buffer: wgpu::Buffer,
}

impl BufferableIndex for u16 {
    fn index_format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }
}

impl BufferableIndex for u32 {
    fn index_format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}

impl VertexBuffer {
    /// Creates a vertex buffer from the given slice of vertices.
    ///
    /// # Errors
    /// Returns an error if the length of `vertices` can not be
    /// converted to `u32`.
    pub fn new<T: BufferableVertex>(
        core_system: &CoreRenderingSystem,
        vertices: &[T],
        label: &str,
    ) -> Result<Self> {
        let layout = T::buffer_layout();
        let buffer = create_initialized_vertex_buffer(core_system.device(), vertices, label);
        let n_vertices = u32::try_from(vertices.len())?;
        Ok(Self {
            layout,
            buffer,
            n_vertices,
        })
    }

    pub fn layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.layout
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn n_vertices(&self) -> u32 {
        self.n_vertices
    }
}

impl IndexBuffer {
    /// Creates an index buffer from the given slice of indices.
    ///
    /// # Errors
    /// Returns an error if the length of `vertices` can not be
    /// converted to `u32`.
    pub fn new<T: BufferableIndex>(
        core_system: &CoreRenderingSystem,
        indices: &[T],
        label: &str,
    ) -> Result<Self> {
        let format = T::index_format();
        let buffer = create_initialized_index_buffer(core_system.device(), indices, label);
        let n_indices = u32::try_from(indices.len())?;
        Ok(Self {
            format,
            buffer,
            n_indices,
        })
    }

    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn n_indices(&self) -> u32 {
        self.n_indices
    }
}

impl UniformBuffer {
    /// Creates a uniform buffer from the given slice of uniforms.
    pub fn new<T: BufferableUniform>(
        core_system: &CoreRenderingSystem,
        uniforms: &[T],
        label: &str,
    ) -> Self {
        let bind_group_layout_descriptor = T::bind_group_layout_descriptor();
        let bind_group_label = format!("{} bind group", label);
        let buffer = create_initialized_uniform_buffer(core_system.device(), uniforms, label);
        Self {
            bind_group_layout_descriptor,
            bind_group_label,
            buffer,
        }
    }

    /// Creates a bind group for the uniform buffer and returns it
    /// together with its layout.
    pub fn create_bind_group_and_layout(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let bind_group_layout = device.create_bind_group_layout(&self.bind_group_layout_descriptor);
        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &self.buffer,
            &self.bind_group_label,
        );
        (bind_group, bind_group_layout)
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some(label),
        })
    }
}

fn create_initialized_vertex_buffer<T: Pod>(
    device: &wgpu::Device,
    vertices: &[T],
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
        label: Some(label),
    })
}

fn create_initialized_index_buffer<T: Pod>(
    device: &wgpu::Device,
    indices: &[T],
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
        label: Some(label),
    })
}

fn create_initialized_uniform_buffer<T: Pod>(
    device: &wgpu::Device,
    uniforms: &[T],
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytemuck::cast_slice(uniforms),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        label: Some(label),
    })
}
