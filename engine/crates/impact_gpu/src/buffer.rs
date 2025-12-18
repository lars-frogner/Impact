//! GPU buffers for rendering and computation.

use crate::device::GraphicsDevice;
use anyhow::Result;
use parking_lot::Mutex;
use std::{
    borrow::Cow,
    fmt::Display,
    mem,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use wgpu::util::DeviceExt;

/// A buffer containing bytes that can be passed to the GPU.
#[derive(Debug)]
pub struct GPUBuffer {
    buffer: wgpu::Buffer,
    buffer_size: usize,
    n_valid_bytes: AtomicUsize,
    label: Cow<'static, str>,
}

/// A buffer containing bytes that can be passed to the GPU,
/// with an embedded count at the beginning of the buffer
/// representing the number of valid elements contained in
/// the buffer.
#[derive(Debug)]
pub struct CountedGPUBuffer {
    buffer: wgpu::Buffer,
    buffer_size: usize,
    padded_count_size: usize,
    item_size: usize,
    n_valid_bytes: AtomicUsize,
    label: Cow<'static, str>,
}

/// Type of the count embedded in the beginning of a [`CountedGPUBuffer`].
pub type Count = u32;

/// The type of information contained in a GPU buffer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GPUBufferType {
    Vertex,
    Index,
    Uniform,
    Storage,
    Result,
    Indirect,
    Query,
}

impl GPUBuffer {
    /// Creates a GPU buffer with the given usage from the given slice of
    /// bytes. Only the first `n_valid_bytes` in the slice are considered to
    /// actually represent valid data, the rest is just buffer filling that
    /// gives room for writing a larger number of bytes than `n_valid_bytes`
    /// into the buffer at a later point without reallocating.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    /// - If `n_valid_bytes` exceeds the size of the `bytes` slice.
    pub fn new(
        graphics_device: &GraphicsDevice,
        bytes: &[u8],
        n_valid_bytes: usize,
        usage: wgpu::BufferUsages,
        label: Cow<'static, str>,
    ) -> Self {
        assert!(!bytes.is_empty(), "Tried to create empty GPU buffer");

        let buffer_size = bytes.len();
        assert!(n_valid_bytes <= buffer_size);

        let buffer_label = format!("{label} GPU buffer");
        let buffer = Self::create_initialized_buffer(
            graphics_device.device(),
            bytes,
            usage | wgpu::BufferUsages::COPY_DST,
            &buffer_label,
        );

        Self {
            buffer,
            buffer_size,
            n_valid_bytes: AtomicUsize::new(n_valid_bytes),
            label,
        }
    }

    /// Creates a GPU buffer with the given size and usage. The given slice of
    /// valid bytes will be written into the beginning of the buffer.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    /// - If the size of the `valid_bytes` slice exceeds `buffer_size`.
    pub fn new_with_spare_capacity(
        graphics_device: &GraphicsDevice,
        buffer_size: usize,
        valid_bytes: &[u8],
        usage: wgpu::BufferUsages,
        label: Cow<'static, str>,
    ) -> Self {
        assert_ne!(buffer_size, 0, "Tried to create empty GPU buffer");
        assert!(valid_bytes.len() <= buffer_size);

        let buffer_label = format!("{label} GPU buffer");
        let buffer = Self::create_uninitialized_buffer(
            graphics_device.device(),
            buffer_size as u64,
            usage | wgpu::BufferUsages::COPY_DST,
            &buffer_label,
        );
        queue_write_to_buffer(
            graphics_device.queue(),
            &buffer,
            0,
            valid_bytes,
            buffer_size,
        );

        Self {
            buffer,
            buffer_size,
            n_valid_bytes: AtomicUsize::new(valid_bytes.len()),
            label,
        }
    }

    /// Creates a GPU buffer with the given size and usage. A write of the given
    /// slice of valid bytes into the beginning of the buffer will be encoded
    /// via the given staging belt.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    /// - If the size of the `valid_bytes` slice exceeds `buffer_size`.
    pub fn new_with_spare_capacity_and_encoded_initialization(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        buffer_size: usize,
        valid_bytes: &[u8],
        usage: wgpu::BufferUsages,
        label: Cow<'static, str>,
    ) -> Self {
        assert_ne!(buffer_size, 0, "Tried to create empty GPU buffer");
        assert!(valid_bytes.len() <= buffer_size);

        let buffer_label = format!("{label} GPU buffer");
        let buffer = Self::create_uninitialized_buffer(
            graphics_device.device(),
            buffer_size as u64,
            usage | wgpu::BufferUsages::COPY_DST,
            &buffer_label,
        );
        encode_write_to_buffer_via_staging_belt(
            graphics_device.device(),
            staging_belt,
            command_encoder,
            &buffer,
            0,
            valid_bytes,
            buffer_size,
        );

        Self {
            buffer,
            buffer_size,
            n_valid_bytes: AtomicUsize::new(valid_bytes.len()),
            label,
        }
    }

    /// Creates an uninitialized GPU buffer of the given type with room for
    /// `buffer_size` bytes.
    ///
    /// # Panics
    /// - If `buffer_size` is zero.
    pub fn new_uninitialized(
        graphics_device: &GraphicsDevice,
        buffer_size: usize,
        usage: wgpu::BufferUsages,
        label: Cow<'static, str>,
    ) -> Self {
        assert_ne!(buffer_size, 0, "Tried to create empty GPU buffer");

        let buffer_label = format!("{label} GPU buffer");
        let buffer = Self::create_uninitialized_buffer(
            graphics_device.device(),
            buffer_size as u64,
            usage,
            &buffer_label,
        );

        Self {
            buffer,
            buffer_size,
            n_valid_bytes: AtomicUsize::new(0),
            label,
        }
    }

    /// Returns a reference to the buffer label.
    pub fn label(&self) -> &Cow<'static, str> {
        &self.label
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

    /// Queues a write of the given slice of bytes to the existing buffer,
    /// starting at the given byte offset. All bytes in the buffer that were
    /// previously considered valid will still be consider valid, along with any
    /// bytes up to the last byte written by this call.
    ///
    /// # Panics
    /// If the end of the region of updated bytes would exceed the total size of
    /// the buffer.
    pub fn update_bytes_from_offset(
        &self,
        graphics_device: &GraphicsDevice,
        byte_offset: usize,
        updated_bytes: &[u8],
    ) {
        let end_offset = byte_offset + updated_bytes.len();
        assert!(end_offset <= self.buffer_size());

        if end_offset > self.n_valid_bytes() {
            self.set_n_valid_bytes(end_offset);
        }

        queue_write_to_buffer(
            graphics_device.queue(),
            self.buffer(),
            byte_offset,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Encodes a write of the given slice of bytes to the existing buffer via
    /// the given staging belt, starting at the given byte offset. All bytes in
    /// the buffer that were previously considered valid will still be consider
    /// valid, along with any bytes up to the last byte written by this call.
    ///
    /// # Panics
    /// If the end of the region of updated bytes would exceed the total size of
    /// the buffer.
    pub fn encode_update_of_bytes_from_offset(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        byte_offset: usize,
        updated_bytes: &[u8],
    ) {
        let end_offset = byte_offset + updated_bytes.len();
        assert!(end_offset <= self.buffer_size());

        if end_offset > self.n_valid_bytes() {
            self.set_n_valid_bytes(end_offset);
        }

        encode_write_to_buffer_via_staging_belt(
            graphics_device.device(),
            staging_belt,
            command_encoder,
            self.buffer(),
            byte_offset,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Queues a write of the given slice of bytes to the existing
    /// buffer, starting at the beginning of the buffer. Any existing
    /// bytes in the buffer that are not overwritten are from then
    /// on considered invalid.
    ///
    /// # Panics
    /// If the slice of updated bytes exceeds the total size of the
    /// buffer.
    pub fn update_valid_bytes(&self, graphics_device: &GraphicsDevice, updated_bytes: &[u8]) {
        self.set_n_valid_bytes(updated_bytes.len());

        queue_write_to_buffer(
            graphics_device.queue(),
            self.buffer(),
            0,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Encodes a write of the given slice of bytes to the existing buffer via
    /// the given staging belt, starting at the beginning of the buffer. Any
    /// existing bytes in the buffer that are not overwritten are from then on
    /// considered invalid.
    ///
    /// # Panics
    /// If the slice of updated bytes exceeds the total size of the
    /// buffer.
    pub fn encode_update_of_valid_bytes(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        updated_bytes: &[u8],
    ) {
        self.set_n_valid_bytes(updated_bytes.len());

        encode_write_to_buffer_via_staging_belt(
            graphics_device.device(),
            staging_belt,
            command_encoder,
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
    pub fn update_all_bytes(&self, graphics_device: &GraphicsDevice, updated_bytes: &[u8]) {
        assert_eq!(updated_bytes.len(), self.buffer_size());
        self.update_valid_bytes(graphics_device, updated_bytes);
    }

    /// Encodes a write of the given slice of bytes to the existing buffer via
    /// the given staging belt, starting at the beginning of the buffer. The
    /// slice must have the same size as the buffer.
    ///
    /// # Panics
    /// If the slice of updated bytes does not match the total size of
    /// the buffer.
    pub fn encode_update_of_all_bytes(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        updated_bytes: &[u8],
    ) {
        assert_eq!(updated_bytes.len(), self.buffer_size());
        self.encode_update_of_valid_bytes(
            graphics_device,
            staging_belt,
            command_encoder,
            updated_bytes,
        );
    }

    /// Queues a write of the given slice of bytes to the existing buffer,
    /// starting at the beginning of the buffer. The number of valid bytes in
    /// the buffer does not change.
    ///
    /// # Panics
    /// If the slice of updated bytes exceeds the total number of valid bytes in
    /// the buffer.
    pub fn update_first_bytes(&self, graphics_device: &GraphicsDevice, updated_bytes: &[u8]) {
        assert!(updated_bytes.len() <= self.n_valid_bytes());

        queue_write_to_buffer(
            graphics_device.queue(),
            self.buffer(),
            0,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Encodes a write of the given slice of bytes to the existing buffer via
    /// the given staging belt, starting at the beginning of the buffer. The
    /// number of valid bytes in the buffer does not change.
    ///
    /// # Panics
    /// If the slice of updated bytes exceeds the total number of valid bytes in
    /// the buffer.
    pub fn encode_update_of_first_bytes(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        updated_bytes: &[u8],
    ) {
        assert!(updated_bytes.len() <= self.n_valid_bytes());

        encode_write_to_buffer_via_staging_belt(
            graphics_device.device(),
            staging_belt,
            command_encoder,
            self.buffer(),
            0,
            updated_bytes,
            self.buffer_size(),
        );
    }

    /// Creates a [`BindGroupEntry`](wgpu::BindGroupEntry) with the given
    /// binding for the full GPU buffer.
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

    /// Maps the buffer from the to the CPU, calls the given closure with the
    /// mapped bytes, unmaps the buffer and returns the result of the closure.
    ///
    /// # Errors
    /// Returns an error if the mapping operation fails.
    pub fn map_and_process_buffer_bytes<T>(
        &self,
        graphics_device: &GraphicsDevice,
        process_bytes: impl FnOnce(&[u8]) -> T,
    ) -> Result<T> {
        let view = map_buffer_slice_to_cpu(graphics_device.device(), self.valid_buffer_slice())?;
        let processed = process_bytes(&view);
        drop(view);
        self.buffer().unmap();
        Ok(processed)
    }

    /// Marks the given number of bytes from the start of the buffer as valid.
    ///
    /// # Panics
    /// If `n_valid_bytes` is larger than the buffer size.
    pub fn set_n_valid_bytes(&self, n_valid_bytes: usize) {
        assert!(n_valid_bytes <= self.buffer_size);
        self.n_valid_bytes.store(n_valid_bytes, Ordering::Release);
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

    fn create_uninitialized_buffer(
        device: &wgpu::Device,
        size: u64,
        usage: wgpu::BufferUsages,
        label: &str,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            size,
            usage,
            mapped_at_creation: false,
            label: Some(label),
        })
    }
}

impl CountedGPUBuffer {
    /// Creates a GPU buffer of the given type from the given slice of bytes,
    /// and embed a count at the beginning of the buffer. Only the first
    /// `n_valid_bytes` in the buffer (including the count and its padding) are
    /// considered to actually represent valid data, the rest is just buffer
    /// filling that gives room for writing a larger number of bytes than
    /// `n_valid_bytes` into the buffer at a later point without reallocating.
    ///
    /// # Panics
    /// - If `bytes` is empty.
    /// - If `n_valid_bytes` exceeds the combined size of the padded count and
    ///   the `bytes` slice.
    pub fn new(
        graphics_device: &GraphicsDevice,
        buffer_type: GPUBufferType,
        count: Count,
        bytes: &[u8],
        padded_count_size: usize,
        item_size: usize,
        n_valid_bytes: usize,
        label: Cow<'static, str>,
    ) -> Self {
        assert!(
            !bytes.is_empty(),
            "Tried to create empty counted GPU buffer"
        );

        let buffer_size = Self::compute_size_including_count(padded_count_size, bytes.len());
        assert!(n_valid_bytes <= buffer_size);

        let buffer_label = format!("{} {} GPU buffer", label, &buffer_type);
        let buffer = Self::create_initialized_counted_buffer_of_type(
            graphics_device.device(),
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
            label,
        }
    }

    /// Returns a reference to the buffer label.
    pub fn label(&self) -> &Cow<'static, str> {
        &self.label
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
        graphics_device: &GraphicsDevice,
        updated_bytes: &[u8],
        new_count: Option<Count>,
    ) {
        self.n_valid_bytes.store(
            Self::compute_size_including_count(self.padded_count_size, updated_bytes.len()),
            Ordering::Release,
        );

        Self::queue_writes_to_counted_buffer(
            graphics_device.queue(),
            self.buffer(),
            new_count,
            updated_bytes,
            self.buffer_size,
            self.padded_count_size,
        );
    }

    /// Encodes a write of the given slice of bytes to the existing buffer via
    /// the given staging belt, starting just after the padded count at the
    /// beginning of the buffer. Any existing bytes in the buffer that are not
    /// overwritten are from then on considered invalid. If `new_count` is
    /// [`Some`], the count at the beginning of the buffer will be updated to
    /// the specified value.
    ///
    /// # Panics
    /// If the combined size of the padded count and the slice of updated bytes
    /// exceeds the total size of the buffer.
    pub fn encode_update_of_valid_bytes(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        updated_bytes: &[u8],
        new_count: Option<Count>,
    ) {
        self.n_valid_bytes.store(
            Self::compute_size_including_count(self.padded_count_size, updated_bytes.len()),
            Ordering::Release,
        );

        Self::encode_writes_to_counted_buffer_via_staging_belt(
            graphics_device.device(),
            staging_belt,
            command_encoder,
            self.buffer(),
            new_count,
            updated_bytes,
            self.buffer_size,
            self.padded_count_size,
        );
    }

    /// Creates a [`BindGroupEntry`](wgpu::BindGroupEntry) with the given
    /// binding for the full counted GPU buffer.
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

    pub fn compute_size_including_count(padded_count_size: usize, n_bytes: usize) -> usize {
        padded_count_size.checked_add(n_bytes).unwrap()
    }

    fn create_initialized_counted_buffer_of_type(
        device: &wgpu::Device,
        buffer_type: GPUBufferType,
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

        // Block to make `buffer_slice` and `mapped_memory` drop after we are done with
        // them
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

    fn encode_writes_to_counted_buffer_via_staging_belt(
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        buffer: &wgpu::Buffer,
        count: Option<Count>,
        bytes: &[u8],
        buffer_size: usize,
        padded_count_size: usize,
    ) {
        // Write actual data starting just after the padded count
        encode_write_to_buffer_via_staging_belt(
            device,
            staging_belt,
            command_encoder,
            buffer,
            padded_count_size,
            bytes,
            buffer_size,
        );

        // Update the count if needed
        if let Some(count) = count {
            encode_write_to_buffer_via_staging_belt(
                device,
                staging_belt,
                command_encoder,
                buffer,
                0,
                bytemuck::bytes_of(&count),
                buffer_size,
            );
        }
    }
}

impl GPUBufferType {
    /// Returns the default [`wgpu::BufferUsages`] for this buffer type.
    pub fn usage(&self) -> wgpu::BufferUsages {
        match self {
            Self::Vertex => wgpu::BufferUsages::VERTEX,
            Self::Index => wgpu::BufferUsages::INDEX,
            Self::Uniform => wgpu::BufferUsages::UNIFORM,
            Self::Storage => {
                wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST
            }
            Self::Result => wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            Self::Indirect => wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE,
            Self::Query => wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        }
    }
}

impl Display for GPUBufferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Vertex => "vertex",
                Self::Index => "index",
                Self::Uniform => "uniform",
                Self::Storage => "storage",
                Self::Result => "result",
                Self::Indirect => "indirect",
                Self::Query => "query",
            }
        )
    }
}

/// Encodes the command for copying the valid bytes from the given source buffer
/// to the given destination buffer, and updates the range of valid bytes in the
/// destination buffer accordingly.
///
/// # Warning
/// The number of valid bytes in the destination buffer will be updated
/// immediately, while the actual copy will not be perform until the command is
/// submitted.
pub fn encode_buffer_to_buffer_copy_command(
    command_encoder: &mut wgpu::CommandEncoder,
    source: &GPUBuffer,
    destination: &GPUBuffer,
) {
    let n_valid_bytes = source.n_valid_bytes();
    assert!(n_valid_bytes <= destination.buffer_size());

    command_encoder.copy_buffer_to_buffer(
        source.buffer(),
        0,
        destination.buffer(),
        0,
        n_valid_bytes as u64,
    );

    destination.set_n_valid_bytes(n_valid_bytes);
}

/// Maps the given buffer slice from the GPU to the CPU and returns the mapped
/// view.
///
/// # Errors
/// Returns an error if the mapping operation fails.
pub fn map_buffer_slice_to_cpu<'a>(
    device: &wgpu::Device,
    buffer_slice: wgpu::BufferSlice<'a>,
) -> Result<wgpu::BufferView> {
    let map_result_sender = Arc::new(Mutex::new(None));
    let map_result_receiver = Arc::clone(&map_result_sender);

    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        *map_result_sender.lock() = Some(result);
    });

    device.poll(wgpu::PollType::wait_indefinitely())?;

    map_result_receiver.lock().take().unwrap()?;

    Ok(buffer_slice.get_mapped_range())
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

    let mut view = queue
        .write_buffer_with(
            buffer,
            byte_offset as wgpu::BufferAddress,
            wgpu::BufferSize::new(bytes.len() as u64).unwrap(),
        )
        .unwrap();

    view.copy_from_slice(bytes);
}

fn encode_write_to_buffer_via_staging_belt(
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    command_encoder: &mut wgpu::CommandEncoder,
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

    let mut view = staging_belt.write_buffer(
        command_encoder,
        buffer,
        byte_offset as wgpu::BufferAddress,
        wgpu::BufferSize::new(bytes.len() as u64).unwrap(),
        device,
    );

    view.copy_from_slice(bytes);
}
