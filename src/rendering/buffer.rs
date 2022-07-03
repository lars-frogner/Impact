//! Data buffers for rendering.

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

pub struct VertexBuffer {
    layout: wgpu::VertexBufferLayout<'static>,
    buffer: wgpu::Buffer,
    n_vertices: u32,
}

pub struct IndexBuffer {
    format: wgpu::IndexFormat,
    buffer: wgpu::Buffer,
    n_indices: u32,
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
