//! Data buffers for rendering.

use crate::rendering::{CoreRenderingSystem, MeshShaderInput};
use bytemuck::Pod;
use impact_utils::{Alignment, ConstStringHash64};
use std::{
    mem,
    num::NonZeroU64,
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

/// A buffer containing bytes that can be passed to the GPU,
/// with an embedded count at the beginning of the buffer
/// representing the number of valid elements contained in
/// the buffer.
#[derive(Debug)]
pub struct CountedRenderBuffer {
    buffer: wgpu::Buffer,
    buffer_size: usize,
    padded_count_size: usize,
    item_size: usize,
    n_valid_bytes: AtomicUsize,
}

/// Type of the count embedded in the beginning of a [`CountedRenderBuffer`].
pub type Count = u32;

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
        let buffer = Self::create_initialized_buffer_of_type(
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

    fn create_initialized_buffer_of_type(
        device: &wgpu::Device,
        buffer_type: RenderBufferType,
        bytes: &[u8],
        label: &str,
    ) -> wgpu::Buffer {
        let usage = buffer_type.usage() | wgpu::BufferUsages::COPY_DST;
        Self::create_initialized_buffer(device, bytes, usage, label)
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
}

impl CountedRenderBuffer {
    /// Creates a counted uniform render buffer initialized with the given
    /// uniform data, with the first `n_valid_uniforms` considered valid data.
    ///
    /// # Panics
    /// - If `uniforms` is empty.
    /// - If the size of a single uniform is not a multiple of 16 (the minimum
    ///   required uniform alignment).
    /// - If `n_valid_uniforms` exceeds the number of items in the `uniforms`
    ///   slice.
    pub fn new_uniform_buffer<U>(
        core_system: &CoreRenderingSystem,
        uniforms: &[U],
        n_valid_uniforms: usize,
        label: &str,
    ) -> Self
    where
        U: UniformBufferable,
    {
        // Uniforms have a minimum size of 16 bytes
        let padded_count_size = 16;

        let item_size = mem::size_of::<U>();

        assert!(
            Alignment::SIXTEEN.is_aligned(item_size),
            "Tried to create uniform buffer with uniform size that \
             causes invalid alignment (uniform buffer item stride \
             must be a multiple of 16)"
        );

        let count = Count::try_from(n_valid_uniforms).unwrap();

        let n_valid_bytes = Self::compute_size_including_count(
            padded_count_size,
            item_size.checked_mul(n_valid_uniforms).unwrap(),
        );

        let bytes = bytemuck::cast_slice(uniforms);

        Self::new(
            core_system,
            RenderBufferType::Uniform,
            count,
            bytes,
            padded_count_size,
            item_size,
            n_valid_bytes,
            &format!("{} uniform", label),
        )
    }

    /// Creates a render buffer of the given type from the given slice of bytes,
    /// and embed a count at the beginning of the buffer. Only the first
    /// `n_valid_bytes` in the buffer (including the count and its padding) are
    /// considered to actually represent valid data, the rest is just buffer
    /// filling that gives room for writing a larger number of bytes than
    /// `n_valid_bytes` into the buffer at a later point without reallocating.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    /// - If `n_valid_bytes` exceeds the combined size of the padded count and the
    ///   `bytes` slice.
    fn new(
        core_system: &CoreRenderingSystem,
        buffer_type: RenderBufferType,
        count: Count,
        bytes: &[u8],
        padded_count_size: usize,
        item_size: usize,
        n_valid_bytes: usize,
        label: &str,
    ) -> Self {
        assert!(
            !bytes.is_empty(),
            "Tried to create empty counted render buffer"
        );

        let buffer_size = Self::compute_size_including_count(padded_count_size, bytes.len());
        assert!(n_valid_bytes <= buffer_size);

        let buffer_label = format!("{} render buffer", label);
        let buffer = Self::create_initialized_counted_buffer_of_type(
            core_system.device(),
            buffer_type,
            count,
            bytes,
            padded_count_size,
            &buffer_label,
        );

        Self {
            buffer,
            buffer_size,
            padded_count_size,
            item_size,
            n_valid_bytes: AtomicUsize::new(n_valid_bytes),
        }
    }

    /// Returns the maximum number of items that can fit in the buffer (not
    /// including the embedded count).
    pub fn max_item_count(&self) -> usize {
        self.buffer_size
            .checked_sub(self.padded_count_size)
            .unwrap()
            .checked_div(self.item_size)
            .unwrap()
    }

    /// Returns a slice of the underlying [`wgpu::Buffer`] containing only valid
    /// bytes.
    pub fn valid_buffer_slice(&self) -> wgpu::BufferSlice<'_> {
        let upper_address = self.n_valid_bytes() as wgpu::BufferAddress;
        self.buffer.slice(..upper_address)
    }

    /// Returns the number of bytes, starting from the beginning of the buffer,
    /// that is considered to contain valid data (this includes the padded count
    /// at the beginning of the buffer).
    pub fn n_valid_bytes(&self) -> usize {
        self.n_valid_bytes.load(Ordering::Acquire)
    }

    /// Whether the buffer is empty, meaning that it does not contain any valid
    /// data apart from the count.
    pub fn is_empty(&self) -> bool {
        self.n_valid_bytes() == self.padded_count_size
    }

    /// Whether the given number of bytes would exceed the capacity of
    /// the buffer (when the padded count at the beginning of the buffer is
    /// taken into account).
    pub fn bytes_exceed_capacity(&self, n_bytes: usize) -> bool {
        Self::compute_size_including_count(self.padded_count_size, n_bytes) > self.buffer_size
    }

    /// Queues a write of the given slice of bytes to the existing buffer,
    /// starting just after the padded count at the beginning of the buffer. Any
    /// existing bytes in the buffer that are not overwritten are from then on
    /// considered invalid. If `new_count` is [`Some`], the count at the
    /// beginning of the buffer will be updated to the specified value.
    ///
    /// # Panics
    /// If the combined size of the padded count and the slice of updated bytes
    /// exceeds the total size of the buffer.
    pub fn update_valid_bytes(
        &self,
        core_system: &CoreRenderingSystem,
        updated_bytes: &[u8],
        new_count: Option<Count>,
    ) {
        self.n_valid_bytes.store(
            Self::compute_size_including_count(self.padded_count_size, updated_bytes.len()),
            Ordering::Release,
        );

        Self::queue_writes_to_counted_buffer(
            core_system.queue(),
            self.buffer(),
            new_count,
            updated_bytes,
            self.buffer_size,
            self.padded_count_size,
        );
    }

    /// Queues a write of the given slice of bytes to the existing buffer,
    /// starting just after the padded count at the beginning of the buffer. The
    /// slice must have the same size as the part of the buffer after the padded
    /// count. If `new_count` is [`Some`], the count at the beginning of the
    /// buffer will be updated to the specified value.
    ///
    /// # Panics
    /// If the combined size of the padded count and the slice of updated bytes
    /// is not the same as the total size of the buffer.
    pub fn update_all_bytes(
        &self,
        core_system: &CoreRenderingSystem,
        updated_bytes: &[u8],
        new_count: Option<Count>,
    ) {
        assert_eq!(
            Self::compute_size_including_count(self.padded_count_size, updated_bytes.len()),
            self.buffer_size
        );
        self.update_valid_bytes(core_system, updated_bytes, new_count);
    }

    /// Creates a [`BindGroupEntry`](wgpu::BindGroupEntry) with the given
    /// binding for the full counted uniform buffer.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: self.buffer().as_entire_binding(),
        }
    }

    /// Returns the underlying [`wgpu::Buffer`].
    fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn compute_size_including_count(padded_count_size: usize, n_bytes: usize) -> usize {
        padded_count_size.checked_add(n_bytes).unwrap()
    }

    fn create_initialized_counted_buffer_of_type(
        device: &wgpu::Device,
        buffer_type: RenderBufferType,
        count: Count,
        bytes: &[u8],
        padded_count_size: usize,
        label: &str,
    ) -> wgpu::Buffer {
        let usage = buffer_type.usage() | wgpu::BufferUsages::COPY_DST;
        Self::create_initialized_counted_buffer(
            device,
            count,
            bytes,
            padded_count_size,
            usage,
            label,
        )
    }

    fn create_initialized_counted_buffer(
        device: &wgpu::Device,
        count: Count,
        bytes: &[u8],
        padded_count_size: usize,
        usage: wgpu::BufferUsages,
        label: &str,
    ) -> wgpu::Buffer {
        let buffer_size = Self::compute_size_including_count(padded_count_size, bytes.len());

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: buffer_size as u64,
            usage,
            mapped_at_creation: true,
            label: Some(label),
        });

        // Block to make `buffer_slice` and `mapped_memory` drop after we are done with them
        {
            let buffer_slice = buffer.slice(..);
            let mut mapped_memory = buffer_slice.get_mapped_range_mut();

            // Write count to beginning, followed by actual data after the count padding
            mapped_memory[0..mem::size_of::<Count>()].copy_from_slice(bytemuck::bytes_of(&count));
            mapped_memory[padded_count_size..].copy_from_slice(bytes);
        }

        buffer.unmap();

        buffer
    }

    fn queue_writes_to_counted_buffer(
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        count: Option<Count>,
        bytes: &[u8],
        buffer_size: usize,
        padded_count_size: usize,
    ) {
        // Write actual data starting just after the padded count
        queue_write_to_buffer(queue, buffer, padded_count_size, bytes, buffer_size);

        // Update the count if needed
        if let Some(count) = count {
            queue_write_to_buffer(queue, buffer, 0, bytemuck::bytes_of(&count), buffer_size);
        }
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

impl RenderBufferType {
    fn usage(&self) -> wgpu::BufferUsages {
        match self {
            Self::Vertex => wgpu::BufferUsages::VERTEX,
            Self::Index => wgpu::BufferUsages::INDEX,
            Self::Uniform => wgpu::BufferUsages::UNIFORM,
        }
    }
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
