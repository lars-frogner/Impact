//! Management of storage buffers for GPU computation or rendering.

use crate::gpu::{
    rendering::buffer::{RenderBuffer, RenderBufferType},
    GraphicsDevice,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::stringhash64_newtype;
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    mem,
};

stringhash64_newtype!(
    /// Identifier for a specific storage buffer on the GPU. Wraps a
    /// [`StringHash64`](impact_utils::StringHash64).
    [pub] StorageBufferID
);

/// Owner and manager of a storage buffer and potentially a result buffer that
/// can be read from the CPU.
#[derive(Debug)]
pub struct StorageRenderBuffer {
    storage_buffer: RenderBuffer,
    /// Buffer that the storage buffer can be copied in, which can be mapped for
    /// transferring the data to the CPU.
    _result_buffer: Option<RenderBuffer>,
    is_read_only: bool,
}

/// Manager of storage buffers on the GPU.
#[derive(Debug)]
pub struct StorageRenderBufferManager {
    buffers: HashMap<StorageBufferID, StorageRenderBuffer>,
}

impl StorageRenderBuffer {
    /// Creates a new read-only storage buffer containing the given values.
    ///
    /// The storage buffer can only be read from on the GPU.
    ///
    /// # Panics
    /// - If the given slice is empty.
    /// - If `T` is a zero-sized type.
    pub fn new_read_only<T>(
        graphics_device: &GraphicsDevice,
        values: &[T],
        label: Cow<'static, str>,
    ) -> Self
    where
        T: Zeroable + Pod,
    {
        assert!(
            !values.is_empty(),
            "Tried to create storage buffer from empty slice"
        );
        assert_ne!(
            mem::size_of::<T>(),
            0,
            "Tried to create storage buffer with zero-sized type"
        );

        let storage_buffer =
            RenderBuffer::new_storage_buffer(graphics_device, bytemuck::cast_slice(values), label);

        Self {
            storage_buffer,
            _result_buffer: None,
            is_read_only: true,
        }
    }

    /// Creates a new read-write storage buffer with room for the given number
    /// of bytes.
    ///
    /// The storage buffer will be initialized with zeros, can be written to and
    /// read from on the GPU, but can not be mapped for transferring the data to
    /// the CPU.
    ///
    /// # Panics
    /// If the given number of bytes is zero.
    pub fn new_read_write(
        graphics_device: &GraphicsDevice,
        n_bytes: usize,
        label: Cow<'static, str>,
    ) -> Self {
        assert_ne!(n_bytes, 0, "Tried to create empty storage buffer");

        let storage_buffer =
            RenderBuffer::new_storage_buffer(graphics_device, vec![0; n_bytes].as_slice(), label);

        Self {
            storage_buffer,
            _result_buffer: None,
            is_read_only: false,
        }
    }

    /// Creates a new read-write storage buffer with room for the given number
    /// of bytes.
    ///
    /// The storage buffer will be initialized with zeros, can be written to and
    /// read from on the GPU, and will be accompanied by a result buffer that
    /// the storage buffer can be copied into, which can be mapped for
    /// transferring the data to the CPU.
    ///
    /// # Panics
    /// If the given number of bytes is zero.
    pub fn new_read_write_with_result_on_cpu(
        graphics_device: &GraphicsDevice,
        n_bytes: usize,
        label: Cow<'static, str>,
    ) -> Self {
        assert_ne!(n_bytes, 0, "Tried to create empty storage buffer");

        let storage_buffer = RenderBuffer::new_storage_buffer(
            graphics_device,
            vec![0; n_bytes].as_slice(),
            label.clone(),
        );

        let result_buffer = Some(RenderBuffer::new_result_buffer(
            graphics_device,
            n_bytes,
            label,
        ));

        Self {
            storage_buffer,
            _result_buffer: result_buffer,
            is_read_only: false,
        }
    }

    /// Creates the bind group layout entry for the storage buffer, assigned to
    /// the given binding.
    pub fn create_bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        create_storage_buffer_bind_group_layout_entry(binding, visibility, self.is_read_only)
    }

    /// Creates a bind group entry for the full storage buffer, assigned to the
    /// given binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        self.storage_buffer.create_bind_group_entry(binding)
    }
}

impl StorageRenderBufferManager {
    /// Creates a new manager with no buffers.
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`StorageRenderBuffer`]s.
    pub fn storage_buffers(&self) -> &HashMap<StorageBufferID, StorageRenderBuffer> {
        &self.buffers
    }

    /// Returns the storage buffer with the given ID, or [`None`] if the buffer
    /// does not exist.
    pub fn get_storage_buffer(&self, buffer_id: StorageBufferID) -> Option<&StorageRenderBuffer> {
        self.buffers.get(&buffer_id)
    }

    /// Returns a hashmap entry for the storage buffer with the given ID.
    pub fn storage_buffer_entry(
        &mut self,
        buffer_id: StorageBufferID,
    ) -> Entry<'_, StorageBufferID, StorageRenderBuffer> {
        self.buffers.entry(buffer_id)
    }

    /// Adds the given storage buffers to the manager under the given ID. If a
    /// buffer with the same ID exists, it will be overwritten.
    pub fn add_storage_buffer(
        &mut self,
        buffer_id: StorageBufferID,
        resources: StorageRenderBuffer,
    ) {
        self.buffers.insert(buffer_id, resources);
    }
}

impl Default for StorageRenderBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBuffer {
    /// Creates a storage render buffer initialized with the given bytes.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    pub fn new_storage_buffer(
        graphics_device: &GraphicsDevice,
        bytes: &[u8],
        label: Cow<'static, str>,
    ) -> Self {
        Self::new(
            graphics_device,
            RenderBufferType::Storage,
            bytes,
            bytes.len(),
            label,
        )
    }

    /// Creates a result render buffer with the given size in bytes.
    ///
    /// # Warning
    /// The contents of the buffer are uninitialized, so the buffer should not
    /// be mapped for reading until it has been copied to.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    pub fn new_result_buffer(
        graphics_device: &GraphicsDevice,
        buffer_size: usize,
        label: Cow<'static, str>,
    ) -> Self {
        Self::new_uninitialized(
            graphics_device,
            RenderBufferType::Result,
            buffer_size,
            buffer_size,
            label,
        )
    }
}

/// Creates a [`BindGroupLayoutEntry`](wgpu::BindGroupLayoutEntry) for a storage
/// buffer, using the given binding and visibility for the bind group and
/// whether the buffer should be read-only.
pub const fn create_storage_buffer_bind_group_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
