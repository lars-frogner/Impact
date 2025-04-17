//! Management of storage buffers for GPU computation or rendering.

use crate::gpu::{
    GraphicsDevice,
    buffer::{self, GPUBuffer, GPUBufferType},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::stringhash64_newtype;
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    mem,
};

stringhash64_newtype!(
    /// Identifier for a specific storage buffer on the GPU. Wraps a
    /// [`StringHash64`](impact_math::StringHash64).
    [pub] StorageBufferID
);

/// Owner and manager of a storage buffer and potentially a result buffer that
/// can be read from the CPU.
#[derive(Debug)]
pub struct StorageGPUBuffer {
    storage_buffer: GPUBuffer,
    /// Buffer that the storage buffer can be copied in, which can be mapped for
    /// transferring the data to the CPU.
    result_buffer: Option<GPUBuffer>,
    is_read_only: bool,
}

/// Manager of storage buffers on the GPU.
#[derive(Debug)]
pub struct StorageGPUBufferManager {
    buffers: HashMap<StorageBufferID, StorageGPUBuffer>,
}

impl StorageGPUBuffer {
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

        let storage_buffer = GPUBuffer::new_storage_buffer_with_bytes(
            graphics_device,
            bytemuck::cast_slice(values),
            label,
        );

        Self {
            storage_buffer,
            result_buffer: None,
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

        let storage_buffer = GPUBuffer::new_storage_buffer_with_bytes(
            graphics_device,
            vec![0; n_bytes].as_slice(),
            label,
        );

        Self {
            storage_buffer,
            result_buffer: None,
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

        let storage_buffer = GPUBuffer::new_storage_buffer_with_bytes(
            graphics_device,
            vec![0; n_bytes].as_slice(),
            label.clone(),
        );

        let result_buffer = Some(GPUBuffer::new_result_buffer(
            graphics_device,
            n_bytes,
            label,
        ));

        Self {
            storage_buffer,
            result_buffer,
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

    /// Encodes the command to copy the contents of the storage buffer to the
    /// accompanying result buffer.
    ///
    /// # Errors
    /// Returns an error if the storage buffer has no accompanying result
    /// buffer.
    pub fn encode_copy_to_result_buffer(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.result_buffer
            .as_ref()
            .map(|result_buffer| {
                buffer::encode_buffer_to_buffer_copy_command(
                    command_encoder,
                    &self.storage_buffer,
                    result_buffer,
                );
            })
            .ok_or_else(|| anyhow!("No result buffer to copy storage buffer to"))
    }

    /// Maps the result buffer from the GPU to the CPU, calls the given closure
    /// with the mapped bytes and returns the result of the closure. If there is
    /// no result buffer accompanying the storage, nothing is done and [`None`]
    /// is returned.
    ///
    /// # Errors
    /// Returns an error if the mapping operation fails.
    pub fn load_result<T>(
        &self,
        graphics_device: &GraphicsDevice,
        process_bytes: impl FnOnce(&[u8]) -> T,
    ) -> Option<Result<T>> {
        self.result_buffer.as_ref().map(|result_buffer| {
            result_buffer.map_and_process_buffer_bytes(graphics_device, process_bytes)
        })
    }
}

impl StorageGPUBufferManager {
    /// Creates a new manager with no buffers.
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`StorageGPUBuffer`]s.
    pub fn storage_buffers(&self) -> &HashMap<StorageBufferID, StorageGPUBuffer> {
        &self.buffers
    }

    /// Returns the storage buffer with the given ID, or [`None`] if the buffer
    /// does not exist.
    pub fn get_storage_buffer(&self, buffer_id: StorageBufferID) -> Option<&StorageGPUBuffer> {
        self.buffers.get(&buffer_id)
    }

    /// Returns a hashmap entry for the storage buffer with the given ID.
    pub fn storage_buffer_entry(
        &mut self,
        buffer_id: StorageBufferID,
    ) -> Entry<'_, StorageBufferID, StorageGPUBuffer> {
        self.buffers.entry(buffer_id)
    }

    /// Adds the given storage buffers to the manager under the given ID. If a
    /// buffer with the same ID exists, it will be overwritten.
    pub fn add_storage_buffer(&mut self, buffer_id: StorageBufferID, resources: StorageGPUBuffer) {
        self.buffers.insert(buffer_id, resources);
    }
}

impl Default for StorageGPUBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GPUBuffer {
    /// Creates a storage GPU buffer initialized with the given values.
    ///
    /// # Panics
    /// - If `values` is empty.
    pub fn new_storage_buffer<T: Pod>(
        graphics_device: &GraphicsDevice,
        values: &[T],
        label: Cow<'static, str>,
    ) -> Self {
        let bytes = bytemuck::cast_slice(values);
        Self::new_storage_buffer_with_bytes(graphics_device, bytes, label)
    }

    /// Creates a storage GPU buffer with capacity for the given number of
    /// values, with the start of the buffer initialized with the given values.
    ///
    /// # Panics
    /// - If `total_value_capacity` is zero.
    /// - If the length of the `initial_values` slice exceeds
    ///   `total_value_capacity`.
    pub fn new_storage_buffer_with_spare_capacity<T: Pod>(
        graphics_device: &GraphicsDevice,
        total_value_capacity: usize,
        initial_values: &[T],
        label: Cow<'static, str>,
    ) -> Self {
        let buffer_size = mem::size_of::<T>()
            .checked_mul(total_value_capacity)
            .unwrap();
        let valid_bytes = bytemuck::cast_slice(initial_values);
        Self::new_storage_buffer_with_bytes_and_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            label,
        )
    }

    /// Creates a storage GPU buffer initialized with the given bytes.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    pub fn new_storage_buffer_with_bytes(
        graphics_device: &GraphicsDevice,
        bytes: &[u8],
        label: Cow<'static, str>,
    ) -> Self {
        Self::new(
            graphics_device,
            bytes,
            bytes.len(),
            GPUBufferType::Storage.usage(),
            label,
        )
    }

    /// Creates a storage GPU buffer with the given size. The given slice of
    /// valid bytes will be written into the beginning of the buffer.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    /// - If the size of the `valid_bytes` slice exceeds `buffer_size`.
    pub fn new_storage_buffer_with_bytes_and_spare_capacity(
        graphics_device: &GraphicsDevice,
        buffer_size: usize,
        valid_bytes: &[u8],
        label: Cow<'static, str>,
    ) -> Self {
        Self::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            GPUBufferType::Storage.usage(),
            label,
        )
    }

    /// Creates a result GPU buffer with the given size in bytes.
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
            buffer_size,
            GPUBufferType::Result.usage(),
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
