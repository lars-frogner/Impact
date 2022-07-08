//! Data buffers for rendering.

use crate::rendering::CoreRenderingSystem;
use bytemuck::Pod;
use std::mem;
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
    /// Descriptor for the layout of the uniform's bind group.
    const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static>;
}

/// A buffer containing vertices.
pub struct VertexBuffer {
    layout: wgpu::VertexBufferLayout<'static>,
    buffer: wgpu::Buffer,
    n_vertices: u32,
}

/// A buffer containing model instances.
///
/// Since a vertex buffer is used for storing the instances,
/// this is just a thin wrapper around `VertexBuffer`.
pub struct InstanceBuffer(VertexBuffer);

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
    n_uniforms: u32,
}

impl BufferableIndex for u16 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl BufferableIndex for u32 {
    const INDEX_FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl VertexBuffer {
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

    /// Returns the underlying `wgpu` buffer.
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

impl InstanceBuffer {
    /// Creates an instance buffer from the given slice of instances.
    ///
    /// # Panics
    /// If the length of `instances` can not be converted to `u32`.
    pub fn new<INS: BufferableInstance>(
        core_system: &CoreRenderingSystem,
        instances: &[INS],
        label: &str,
    ) -> Self {
        Self(VertexBuffer::new(core_system, instances, label))
    }

    /// Returns the layout of the vertex buffer used for storing
    /// instances.
    pub fn layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        self.0.layout()
    }

    /// Returns the underlying `wgpu` buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.0.buffer()
    }

    /// Returns the number of instances in the buffer.
    pub fn n_instances(&self) -> u32 {
        self.0.n_vertices()
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
    pub fn queue_update_of_instances<INS: BufferableInstance>(
        &self,
        core_system: &CoreRenderingSystem,
        first_updated_instance_idx: u32,
        updated_instances: &[INS],
    ) {
        self.0
            .queue_update_of_vertices(core_system, first_updated_instance_idx, updated_instances);
    }
}

impl IndexBuffer {
    /// Creates an index buffer from the given slice of indices.
    ///
    /// # Panics
    /// If the length of `indices` can not be converted to `u32`.
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

    /// Returns the underlying `wgpu` buffer.
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

impl UniformBuffer {
    /// Creates a uniform buffer from the given slice of uniforms.
    pub fn new<U: BufferableUniform>(
        core_system: &CoreRenderingSystem,
        uniforms: &[U],
        label: &str,
    ) -> Self {
        let buffer_label = format!("{} uniform buffer", label);
        let bind_group_label = format!("{} bind group", &buffer_label);
        let bind_group_layout_descriptor = U::BIND_GROUP_LAYOUT_DESCRIPTOR;
        let buffer =
            create_initialized_uniform_buffer(core_system.device(), uniforms, &buffer_label);
        let n_uniforms = u32::try_from(uniforms.len()).unwrap();
        Self {
            bind_group_layout_descriptor,
            bind_group_label,
            buffer,
            n_uniforms,
        }
    }

    /// Returns the underlying `wgpu` buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the number of uniforms in the buffer.
    pub fn n_uniforms(&self) -> u32 {
        self.n_uniforms
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
        let descriptor = U::BIND_GROUP_LAYOUT_DESCRIPTOR;
        let original_descriptor = self.bind_group_layout_descriptor.clone();
        assert!(
            descriptor.label == original_descriptor.label
                && descriptor.entries.len() == original_descriptor.entries.len()
                && descriptor
                    .entries
                    .iter()
                    .zip(original_descriptor.entries.iter())
                    .all(|(entry, original_entry)| entry == original_entry),
            "Updated uniforms do not have original bind group descriptor"
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

fn create_initialized_vertex_buffer(
    device: &wgpu::Device,
    vertices: &[impl Pod],
    label: &str,
) -> wgpu::Buffer {
    create_initialized_buffer(device, vertices, wgpu::BufferUsages::VERTEX, label)
}

fn create_initialized_index_buffer(
    device: &wgpu::Device,
    indices: &[impl Pod],
    label: &str,
) -> wgpu::Buffer {
    create_initialized_buffer(device, indices, wgpu::BufferUsages::INDEX, label)
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
