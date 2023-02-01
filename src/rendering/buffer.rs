//! Data buffers for rendering.

use crate::rendering::{CoreRenderingSystem, MeshShaderInput};
use bytemuck::Pod;
use impact_utils::ConstStringHash64;
use std::{
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};
use wgpu::util::DeviceExt;

/// Represents types that can be written to a vertex buffer.
pub trait VertexBufferable: Pod {
    /// The layout of buffers made up of this vertex type.
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static>;

    /// The input required for accessing the vertex attributes
    /// in a shader.
    const SHADER_INPUT: MeshShaderInput;
}

/// Represents types that can be written to an index buffer.
pub trait IndexBufferable: Pod {
    /// The data format of the index type.
    const INDEX_FORMAT: wgpu::IndexFormat;
}

impl IndexBufferable for u16 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl IndexBufferable for u32 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

/// Represents types that can be written to a uniform buffer.
pub trait UniformBufferable: Pod {
    /// ID for uniform type.
    const ID: ConstStringHash64;

    /// Creates the bind group layout entry for this uniform type,
    /// assigned to the given binding.
    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry;
}

/// A buffer containing bytes that can be passed to the GPU.
#[derive(Debug)]
pub struct RenderBuffer {
    buffer: wgpu::Buffer,
    buffer_size: usize,
    n_valid_bytes: AtomicUsize,
}

/// The type of information contained in a render buffer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderBufferType {
    Vertex,
    Index,
    Uniform,
}

impl RenderBuffer {
    /// Creates a vertex render buffer initialized with the given vertex
    /// data.
    pub fn new_full_vertex_buffer<V>(
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        label: &str,
    ) -> Self
    where
        V: VertexBufferable,
    {
        let bytes = bytemuck::cast_slice(vertices);
        Self::new(
            core_system,
            RenderBufferType::Vertex,
            bytes,
            bytes.len(),
            &format!("{} vertex", label),
        )
    }

    /// Creates an index render buffer initialized with the given index
    /// data.
    pub fn new_full_index_buffer<I>(
        core_system: &CoreRenderingSystem,
        indices: &[I],
        label: &str,
    ) -> Self
    where
        I: IndexBufferable,
    {
        let bytes = bytemuck::cast_slice(indices);
        Self::new(
            core_system,
            RenderBufferType::Index,
            bytes,
            bytes.len(),
            &format!("{} index", label),
        )
    }

    /// Creates a uniform render buffer initialized with the given uniform
    /// data.
    pub fn new_full_uniform_buffer<U>(
        core_system: &CoreRenderingSystem,
        uniforms: &[U],
        label: &str,
    ) -> Self
    where
        U: UniformBufferable,
    {
        let bytes = bytemuck::cast_slice(uniforms);
        Self::new(
            core_system,
            RenderBufferType::Uniform,
            bytes,
            bytes.len(),
            &format!("{} uniform", label),
        )
    }

    /// Creates a render buffer of the given type from the given slice of
    /// bytes. Only the first `n_valid_bytes` in the slice are considered
    /// to actually represent valid data, the rest is just buffer filling
    /// that gives room for writing a larger number of bytes than `n_valid_bytes`
    /// into the buffer at a later point without reallocating.
    ///
    /// # Panics
    /// - If `n_valid_bytes` exceeds the size of the `bytes` slice.
    pub fn new(
        core_system: &CoreRenderingSystem,
        buffer_type: RenderBufferType,
        bytes: &[u8],
        n_valid_bytes: usize,
        label: &str,
    ) -> Self {
        let buffer_size = bytes.len();
        assert!(n_valid_bytes <= buffer_size);

        let buffer_label = format!("{} render buffer", label);
        let buffer = create_initialized_buffer_of_type(
            core_system.device(),
            buffer_type,
            bytes,
            &buffer_label,
        );

        Self {
            buffer,
            buffer_size,
            n_valid_bytes: AtomicUsize::new(n_valid_bytes),
        }
    }

    /// Returns a slice of the underlying [`wgpu::Buffer`]
    /// containing only valid bytes.
    pub fn valid_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        let upper_address = self.n_valid_bytes() as wgpu::BufferAddress;
        self.buffer.slice(..upper_address)
    }

    /// Returns the total size of the buffer in bytes.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Returns the number of bytes, starting from the beginning
    /// of the buffer, that is considered to contain valid data.
    pub fn n_valid_bytes(&self) -> usize {
        self.n_valid_bytes.load(Ordering::Acquire)
    }

    /// Whether the buffer is empty, meaning that it does not
    /// contain any valid data.
    pub fn is_empty(&self) -> bool {
        self.n_valid_bytes() == 0
    }

    /// Queues a write of the given slice of bytes to the existing
    /// buffer, starting at the beginning of the buffer. Any existing
    /// bytes in the buffer that are not overwritten are from then
    /// on considered invalid.
    ///
    /// # Panics
    /// If the slice of updated bytes exceeds the total size of the
    /// buffer.
    pub fn update_valid_bytes(&self, core_system: &CoreRenderingSystem, updated_bytes: &[u8]) {
        self.n_valid_bytes
            .store(updated_bytes.len(), Ordering::Release);

        queue_write_to_buffer(
            core_system.queue(),
            self.buffer(),
            0,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Queues a write of the given slice of bytes to the existing
    /// buffer, starting at the beginning of the buffer. The slice
    /// must have the same size as the buffer.
    ///
    /// # Panics
    /// If the slice of updated bytes does not match the total size of
    /// the buffer.
    pub fn update_all_bytes(&self, core_system: &CoreRenderingSystem, updated_bytes: &[u8]) {
        assert_eq!(updated_bytes.len(), self.buffer_size());
        self.update_valid_bytes(core_system, updated_bytes);
    }

    /// Returns the underlying [`wgpu::Buffer`].
    fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

/// Creates a [`VertexBufferLayout`](wgpu::VertexBufferLayout) for
/// vertex data of type `T`, with data layout defined by the given
/// vertex attributes.
pub const fn create_vertex_buffer_layout_for_vertex<T>(
    attributes: &'static [wgpu::VertexAttribute],
) -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<T>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

/// Creates a [`VertexBufferLayout`](wgpu::VertexBufferLayout) for
/// instance data of type `T`, with data layout defined by the given
/// instance attributes.
pub const fn create_vertex_buffer_layout_for_instance<T>(
    attributes: &'static [wgpu::VertexAttribute],
) -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<T>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes,
    }
}

/// Creates a [`BindGroupLayoutEntry`](wgpu::BindGroupLayoutEntry) for
/// a uniform buffer, using the given binding and visibility for the
/// bind group.
pub const fn create_uniform_buffer_bind_group_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Creates a [`BindGroupEntry`](wgpu::BindGroupEntry) with the given
/// binding for the given uniform buffer representing a single uniform.
pub fn create_single_uniform_bind_group_entry(
    binding: u32,
    render_buffer: &RenderBuffer,
) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: render_buffer.buffer().as_entire_binding(),
    }
}

fn create_initialized_buffer_of_type(
    device: &wgpu::Device,
    buffer_type: RenderBufferType,
    bytes: &[u8],
    label: &str,
) -> wgpu::Buffer {
    let usage = match buffer_type {
        RenderBufferType::Vertex => wgpu::BufferUsages::VERTEX,
        RenderBufferType::Index => wgpu::BufferUsages::INDEX,
        RenderBufferType::Uniform => wgpu::BufferUsages::UNIFORM,
    } | wgpu::BufferUsages::COPY_DST;

    create_initialized_buffer(device, bytes, usage, label)
}

fn create_initialized_buffer(
    device: &wgpu::Device,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytes,
        usage,
        label: Some(label),
    })
}

fn queue_write_to_buffer(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    byte_offset: usize,
    bytes: &[u8],
    buffer_size: usize,
) {
    let n_updated_bytes = bytes.len();
    if n_updated_bytes == 0 {
        return;
    }

    assert!(
        byte_offset.checked_add(n_updated_bytes).unwrap() <= buffer_size,
        "Bytes to write do not fit in original buffer"
    );

    queue.write_buffer(buffer, byte_offset as wgpu::BufferAddress, bytes);
}
