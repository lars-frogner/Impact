//! Data buffers for rendering.

use crate::{hash::ConstStringHash, rendering::CoreRenderingSystem};
use bytemuck::Pod;
use std::{
    mem,
    sync::atomic::{AtomicU32, Ordering},
};
use wgpu::util::DeviceExt;

/// Represents vertex types that can be written to a vertex buffer.
pub trait BufferableVertex: Pod {
    /// The layout of buffers made up of this vertex type.
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static>;
}

/// Represents instance types that can be written to a vertex buffer.
///
/// Since instances are stored in vertex buffers, any type that can be
/// written to a vertex buffer can also be written to an instance buffer.
pub trait BufferableInstance: BufferableVertex {}

/// Represents index types that can be written to an index buffer.
pub trait BufferableIndex: Pod {
    /// The data format of this index type.
    const INDEX_FORMAT: wgpu::IndexFormat;
}

/// Represents uniform types that can be written to a uniform buffer.
pub trait BufferableUniform: Pod {
    /// ID for uniform type.
    const ID: ConstStringHash;

    /// Creates the bind group layout entry for this uniform type,
    /// assigned to the given binding.
    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry;
}

/// A buffer containing vertices.
#[derive(Debug)]
pub struct VertexRenderBuffer {
    layout: wgpu::VertexBufferLayout<'static>,
    buffer: wgpu::Buffer,
    n_vertices: u32,
}

/// A buffer containing model instances.
///
/// Since a vertex buffer is used for storing the instances,
/// this buffer wraps a [`VertexBuffer`]. In addition, it
/// keeps a record of the number of instances, beginning at
/// the start of the buffer, that are considered to have valid
/// values. This enables the buffer to be reused with varying
/// numbers of instances without having to reallocate the buffer
/// every time.
#[derive(Debug)]
pub struct InstanceRenderBuffer {
    vertex_buffer: VertexRenderBuffer,
    n_valid_instances: AtomicU32,
}

/// A buffer containing vertex indices.
#[derive(Debug)]
pub struct IndexRenderBuffer {
    format: wgpu::IndexFormat,
    buffer: wgpu::Buffer,
    n_indices: u32,
}

/// A buffer containing uniforms.
#[derive(Debug)]
pub struct UniformRenderBuffer {
    uniform_id: ConstStringHash,
    buffer: wgpu::Buffer,
    n_uniforms: u32,
}

/// A dynamic buffer containing uniforms.
///
/// This [`UniformBuffer`] wrapper keeps a record of the number
/// of uniforms, beginning at the start of the buffer, that are
/// considered to have valid values. This enables the buffer to be
/// reused with varying numbers of uniforms without having to
/// reallocate the buffer every time.
#[derive(Debug)]
pub struct DynamicUniformRenderBuffer {
    uniform_buffer: UniformRenderBuffer,
    n_valid_uniforms: AtomicU32,
}

impl BufferableIndex for u16 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl BufferableIndex for u32 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl VertexRenderBuffer {
    /// Creates a vertex buffer from the given slice of vertices.
    ///
    /// # Panics
    /// If the length of `vertices` can not be converted to `u32`.
    pub fn new<V: BufferableVertex>(
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        label: &str,
    ) -> Self {
        let buffer_label = format!("{} vertex buffer", label);
        let layout = V::BUFFER_LAYOUT;
        let buffer =
            create_initialized_vertex_buffer(core_system.device(), vertices, &buffer_label);
        let n_vertices = u32::try_from(vertices.len()).unwrap();
        Self {
            layout,
            buffer,
            n_vertices,
        }
    }

    /// Returns the layout of the vertex buffer.
    pub fn layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.layout
    }

    /// Returns the underlying [`wgpu::Buffer`].
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the number of vertices in the buffer.
    pub fn n_vertices(&self) -> u32 {
        self.n_vertices
    }

    /// Queues a write of the given slice of vertices to the existing
    /// buffer, starting at the given vertex index.
    ///
    /// # Panics
    /// - If the updated vertex type has a buffer layout different from
    ///   the original layout.
    /// - If the offset slice of updated vertices exceeds the bounds
    ///   of the original vertex buffer.
    /// - If integer overflow occurs.
    pub fn queue_update_of_vertices<V: BufferableVertex>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_vertex_idx: u32,
        updated_vertices: &[V],
    ) {
        assert!(
            &V::BUFFER_LAYOUT == self.layout(),
            "Updated vertices do not have original buffer layout"
        );

        queue_write_to_buffer(
            core_system.queue(),
            self.buffer(),
            first_updated_vertex_idx,
            updated_vertices,
            self.n_vertices(),
        );
    }
}

impl InstanceRenderBuffer {
    /// Creates an instance buffer from the given slice of instances.
    /// Only the first `n_valid_instances` in the slice are considered
    /// to actually represent valid values, the rest is just buffer
    /// filling that gives room for writing a larger number of instances
    /// than `n_valid_instances` into the buffer at a later point without
    /// reallocating.
    ///
    /// # Panics
    /// - If the length of `instance_buffer` can not be converted to [`u32`].
    /// - If `n_valid_instances` exceeds the length of `instance_buffer`.
    pub fn new<INS: BufferableInstance>(
        core_system: &CoreRenderingSystem,
        instance_buffer: &[INS],
        n_valid_instances: u32,
        label: &str,
    ) -> Self {
        assert!(n_valid_instances as usize <= instance_buffer.len());
        Self {
            vertex_buffer: VertexRenderBuffer::new(core_system, instance_buffer, label),
            n_valid_instances: AtomicU32::new(n_valid_instances),
        }
    }

    /// Returns the layout of the vertex buffer used for storing
    /// instances.
    pub fn layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        self.vertex_buffer.layout()
    }

    /// Returns the underlying [`wgpu::Buffer`].
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.vertex_buffer.buffer()
    }

    /// Returns the maximum number of instances the buffer has room
    /// for.
    pub fn max_instances(&self) -> u32 {
        self.vertex_buffer.n_vertices()
    }

    /// Returns the number of instances, starting from the beginning
    /// of the buffer, that have valid values.
    pub fn n_valid_instances(&self) -> u32 {
        self.n_valid_instances.load(Ordering::Acquire)
    }

    /// Queues a write of the given slice of instances to the existing
    /// buffer, starting at the beginning of the buffer. Any existing
    /// instances in the buffer that are not overwritten are from then
    /// on considered invalid.
    ///
    /// # Panics
    /// - If the updated instance type has a buffer layout different from
    ///   the original layout.
    /// - If the slice of updated instances exceeds the bounds of the
    ///   original instance buffer.
    /// - If integer overflow occurs.
    pub fn update_valid_instances<INS: BufferableInstance>(
        &self,
        core_system: &CoreRenderingSystem,
        updated_instances: &[INS],
    ) {
        let n_updated_instances = u32::try_from(updated_instances.len()).unwrap();
        self.n_valid_instances
            .store(n_updated_instances, Ordering::Release);
        self.queue_update_of_instances(core_system, 0, updated_instances);
    }

    /// Queues a write of the given slice of instances to the existing
    /// buffer, starting at the given instance index.
    ///
    /// # Panics
    /// - If the updated instance type has a buffer layout different from
    ///   the original layout.
    /// - If the offset slice of updated instances exceeds the bounds
    ///   of the original instance buffer.
    /// - If integer overflow occurs.
    fn queue_update_of_instances<INS: BufferableInstance>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_instance_idx: u32,
        updated_instances: &[INS],
    ) {
        self.vertex_buffer.queue_update_of_vertices(
            core_system,
            first_updated_instance_idx,
            updated_instances,
        );
    }
}

impl IndexRenderBuffer {
    /// Creates an index buffer from the given slice of indices.
    ///
    /// # Panics
    /// If the length of `indices` can not be converted to [`u32`].
    pub fn new<IDX: BufferableIndex>(
        core_system: &CoreRenderingSystem,
        indices: &[IDX],
        label: &str,
    ) -> Self {
        let buffer_label = format!("{} index buffer", label);
        let format = IDX::INDEX_FORMAT;
        let buffer = create_initialized_index_buffer(core_system.device(), indices, &buffer_label);
        let n_indices = u32::try_from(indices.len()).unwrap();
        Self {
            format,
            buffer,
            n_indices,
        }
    }

    /// Returns the format of the indices in the buffer.
    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    /// Returns the underlying [`wgpu::Buffer`].
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the number of indices in the buffer.
    pub fn n_indices(&self) -> u32 {
        self.n_indices
    }

    /// Queues a write of the given slice of indices to the existing
    /// buffer, starting at the given index.
    ///
    /// # Panics
    /// - If the updated index type is different from the original type.
    /// - If the offset slice of updated indices exceeds the bounds
    ///   of the original index buffer.
    /// - If integer overflow occurs.
    pub fn queue_update_of_indices<IDX: BufferableIndex>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_index_idx: u32,
        updated_indices: &[IDX],
    ) {
        assert!(
            IDX::INDEX_FORMAT == self.format(),
            "Updated indices do not have original type"
        );

        queue_write_to_buffer(
            core_system.queue(),
            self.buffer(),
            first_updated_index_idx,
            updated_indices,
            self.n_indices(),
        );
    }
}

impl UniformRenderBuffer {
    /// Creates a uniform buffer from the given slice of uniforms.
    pub fn new<U: BufferableUniform>(core_system: &CoreRenderingSystem, uniforms: &[U]) -> Self {
        let uniform_id = U::ID;

        let buffer = create_initialized_uniform_buffer(
            core_system.device(),
            uniforms,
            &format!("{} uniform buffer", uniform_id),
        );

        let n_uniforms = u32::try_from(uniforms.len()).unwrap();

        Self {
            uniform_id,
            buffer,
            n_uniforms,
        }
    }

    /// Creates the bind group entry for the uniform buffer,
    /// assigned to the given binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.buffer().as_entire_binding(),
        }
    }

    /// Returns the underlying [`wgpu::Buffer`].
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the number of uniforms in the buffer.
    pub fn n_uniforms(&self) -> u32 {
        self.n_uniforms
    }

    /// Queues a write of the given slice of uniforms to the existing
    /// buffer, starting at the given uniform index.
    ///
    /// # Panics
    /// - If the updated uniform bind group description is different from
    ///   the original description.
    /// - If the offset slice of updated uniforms exceeds the bounds
    ///   of the original uniform buffer.
    /// - If integer overflow occurs.
    pub fn queue_update_of_uniforms<U: BufferableUniform>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_uniform_idx: u32,
        updated_uniforms: &[U],
    ) {
        assert_eq!(
            U::ID,
            self.uniform_id,
            "Updated uniforms do not have original ID"
        );
        queue_write_to_buffer(
            core_system.queue(),
            self.buffer(),
            first_updated_uniform_idx,
            updated_uniforms,
            self.n_uniforms(),
        );
    }
}

impl DynamicUniformRenderBuffer {
    /// Creates a dynamic uniform buffer from the given slice of uniforms.
    /// Only the first `n_valid_uniforms` in the slice are considered
    /// to actually represent valid values, the rest is just buffer
    /// filling that gives room for writing a larger number of uniforms
    /// than `n_valid_uniforms` into the buffer at a later point without
    /// reallocating.
    ///
    /// # Panics
    /// - If the length of `uniform_buffer` can not be converted to [`u32`].
    /// - If `n_valid_uniforms` exceeds the length of `uniform_buffer`.
    pub fn new<U: BufferableUniform>(
        core_system: &CoreRenderingSystem,
        uniform_buffer: &[U],
        n_valid_uniforms: u32,
    ) -> Self {
        assert!(n_valid_uniforms as usize <= uniform_buffer.len());
        Self {
            uniform_buffer: UniformRenderBuffer::new(core_system, uniform_buffer),
            n_valid_uniforms: AtomicU32::new(n_valid_uniforms),
        }
    }

    /// Creates the bind group entry for the uniform buffer,
    /// assigned to the given binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        self.uniform_buffer.create_bind_group_entry(binding)
    }

    /// Returns the underlying [`wgpu::Buffer`].
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.uniform_buffer.buffer()
    }

    /// Returns the maximum number of uniforms the buffer has room
    /// for.
    pub fn max_uniforms(&self) -> u32 {
        self.uniform_buffer.n_uniforms()
    }

    /// Returns the number of uniforms, starting from the beginning
    /// of the buffer, that have valid values.
    pub fn n_valid_uniforms(&self) -> u32 {
        self.n_valid_uniforms.load(Ordering::Acquire)
    }

    /// Queues a write of the given slice of uniforms to the existing
    /// buffer, starting at the beginning of the buffer. Any existing
    /// uniforms in the buffer that are not overwritten are from then
    /// on considered invalid.
    ///
    /// # Panics
    /// - If the updated uniform type has a buffer layout different from
    ///   the original layout.
    /// - If the slice of updated uniforms exceeds the bounds of the
    ///   original uniform buffer.
    /// - If integer overflow occurs.
    pub fn update_valid_uniforms<U: BufferableUniform>(
        &self,
        core_system: &CoreRenderingSystem,
        updated_uniforms: &[U],
    ) {
        let n_updated_uniforms = u32::try_from(updated_uniforms.len()).unwrap();
        self.n_valid_uniforms
            .store(n_updated_uniforms, Ordering::Release);
        self.queue_update_of_uniforms(core_system, 0, updated_uniforms);
    }

    /// Queues a write of the given slice of uniforms to the existing
    /// buffer, starting at the given uniform index.
    ///
    /// # Panics
    /// - If the updated uniform type has a buffer layout different from
    ///   the original layout.
    /// - If the offset slice of updated uniforms exceeds the bounds
    ///   of the original uniform buffer.
    /// - If integer overflow occurs.
    fn queue_update_of_uniforms<U: BufferableUniform>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_uniform_idx: u32,
        updated_uniforms: &[U],
    ) {
        self.uniform_buffer.queue_update_of_uniforms(
            core_system,
            first_updated_uniform_idx,
            updated_uniforms,
        );
    }
}

fn create_initialized_vertex_buffer(
    device: &wgpu::Device,
    vertices: &[impl Pod],
    label: &str,
) -> wgpu::Buffer {
    create_initialized_buffer(
        device,
        vertices,
        wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        label,
    )
}

fn create_initialized_index_buffer(
    device: &wgpu::Device,
    indices: &[impl Pod],
    label: &str,
) -> wgpu::Buffer {
    create_initialized_buffer(
        device,
        indices,
        wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        label,
    )
}

fn create_initialized_uniform_buffer(
    device: &wgpu::Device,
    uniforms: &[impl Pod],
    label: &str,
) -> wgpu::Buffer {
    create_initialized_buffer(
        device,
        uniforms,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        label,
    )
}

fn create_initialized_buffer(
    device: &wgpu::Device,
    data: &[impl Pod],
    usage: wgpu::BufferUsages,
    label: &str,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: bytemuck::cast_slice(data),
        usage,
        label: Some(label),
    })
}

fn queue_write_to_buffer<T: Pod>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    first_element_idx: u32,
    elements: &[T],
    n_original_elements: u32,
) {
    let n_updated_elements = u32::try_from(elements.len()).unwrap();
    if n_updated_elements == 0 {
        return;
    }

    assert!(
        first_element_idx.checked_add(n_updated_elements).unwrap() <= n_original_elements,
        "Elements to write do not fit in original buffer"
    );

    let byte_offset = (mem::size_of::<T>() as u64)
        .checked_mul(u64::from(first_element_idx))
        .unwrap();

    queue.write_buffer(
        buffer,
        byte_offset as wgpu::BufferAddress,
        bytemuck::cast_slice(elements),
    );
}
