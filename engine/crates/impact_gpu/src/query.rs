//! GPU queries.

use crate::{
    buffer::{self, GPUBuffer, GPUBufferType},
    device::GraphicsDevice,
};
use anyhow::Result;
use std::{borrow::Cow, iter, num::NonZeroU32, time::Duration};

/// Helper for performing timestamp GPU queries.
#[derive(Debug)]
pub struct TimestampQueryManager {
    max_timestamps: NonZeroU32,
    query_set: Option<wgpu::QuerySet>,
    query_resolve_buffer: GPUBuffer,
    timestamp_result_buffer: GPUBuffer,
    timestamp_pairs: Vec<Cow<'static, str>>,
    last_timing_results: Vec<(Cow<'static, str>, Duration)>,
    enabled: bool,
}

/// Helper for registering pairs of timestamp queries for timing render and
/// compute passes. Created by calling
/// [`TimestampQueryManager::create_timestamp_query_registry`]. When all
/// timestamp queries have been registered, the registry should be dropped by
/// calling [`Self::finish`]. After [`wgpu::Queue::submit`] has been called and
/// the render commands have been executed, the timings from the recorded
/// timestamps can be computed by calling
/// [`TimestampQueryManager::load_recorded_timing_results`], and retrieved by
/// calling [`TimestampQueryManager::last_timing_results`].
#[derive(Debug)]
pub struct TimestampQueryRegistry<'a> {
    manager: &'a mut TimestampQueryManager,
}

impl TimestampQueryManager {
    /// Creates a new timestamp query manager, initializing the required GPU
    /// resources with capacity for the given maximum number of timestamps.
    ///
    /// When `enabled` is `false`, the resources will still be initialized, but
    /// the [`TimestampQueryRegistry`] obtained by calling
    /// [`Self::create_timestamp_query_registry`] will not record any timestamp
    /// writes when requested.
    ///
    /// # Panics
    /// - If `max_timestamps` exceeds [`wgpu::QUERY_SET_MAX_QUERIES`].
    /// - If `enabled` is `true` but timestamp queries are not supported by the
    ///   current graphics device.
    pub fn new(
        graphics_device: &GraphicsDevice,
        max_timestamps: NonZeroU32,
        enabled: bool,
    ) -> Self {
        let query_resolve_buffer = GPUBuffer::new_query_buffer(
            graphics_device,
            max_timestamps,
            Cow::Borrowed("Timestamp"),
        );
        let timestamp_result_buffer = GPUBuffer::new_result_buffer(
            graphics_device,
            query_resolve_buffer.buffer_size(),
            Cow::Borrowed("Timestamp"),
        );

        let query_set = if graphics_device.supports_features(wgpu::Features::TIMESTAMP_QUERY) {
            Some(
                graphics_device
                    .device()
                    .create_query_set(&wgpu::QuerySetDescriptor {
                        label: Some("Timestamp query set"),
                        count: max_timestamps.get(),
                        ty: wgpu::QueryType::Timestamp,
                    }),
            )
        } else {
            assert!(
                !enabled,
                "Timestamp queries are not supported by the current graphics device"
            );
            None
        };

        Self {
            max_timestamps,
            query_set,
            query_resolve_buffer,
            timestamp_result_buffer,
            timestamp_pairs: Vec::new(),
            last_timing_results: Vec::new(),
            enabled,
        }
    }

    /// Sets whether timestamp queries are enabled. When disabled, the
    /// [`TimestampQueryRegistry`] obtained by calling
    /// [`Self::create_timestamp_query_registry`] will not register any
    /// timestamp queries when requested.
    ///
    /// # Panics
    /// If `enabled` is `true` but timestamp queries are not supported by the
    /// current graphics device.
    pub fn set_enabled(&mut self, enabled: bool) {
        assert!(
            self.query_set.is_some() || !enabled,
            "Timestamp queries are not supported by the current graphics device"
        );
        self.enabled = enabled;
    }

    /// Creates a [`TimestampQueryRegistry`] for registering timestamp queries
    /// for timing render and compute passes.
    ///
    /// The registry holds an exclusive reference to this manager until it is
    /// dropped, which should be done by calling
    /// [`TimestampQueryRegistry::finish`] when all timestamp queries have been
    /// registered. After [`wgpu::Queue::submit`] has been called and the render
    /// commands have been executed, the timings from the recorded timestamps
    /// can be computed by calling [`Self::load_recorded_timing_results`], and
    /// retrieved by calling [`Self::last_timing_results`].
    pub fn create_timestamp_query_registry(&mut self) -> TimestampQueryRegistry<'_> {
        self.timestamp_pairs.clear();
        TimestampQueryRegistry { manager: self }
    }

    /// Loads the timestamps pairs registered in the [`TimestampQueryRegistry`]
    /// obtained by the last call to [`Self::create_timestamp_query_registry`]
    /// after they have been recorded on the GPU and computes the duration
    /// between each timestamp pair. The results can be obtained by calling
    /// [`Self::last_timing_results`].
    ///
    /// This method must be called after [`wgpu::Queue::submit`] in order for
    /// the recorded timestamps to be available.
    ///
    /// # Errors
    /// Returns an error if the recorded timestamps could not be read from the
    /// GPU buffer.
    pub fn load_recorded_timing_results(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        self.last_timing_results.clear();

        if self.timestamp_pairs.is_empty() {
            return Ok(());
        }

        self.last_timing_results
            .reserve(self.timestamp_pairs.len() + 2);

        let timestamps = self.timestamp_result_buffer.map_and_process_buffer_bytes(
            graphics_device,
            |bytes| {
                let mut timestamps = vec![0_u64; 2 * self.timestamp_pairs.len()];
                let timestamp_bytes = bytemuck::cast_slice_mut(&mut timestamps);
                timestamp_bytes.copy_from_slice(bytes);
                timestamps
            },
        )?;

        let timestamp_period = f64::from(graphics_device.queue().get_timestamp_period());

        let mut aggregate_duration_nanos = 0.0;

        for (tag, start_and_end) in self
            .timestamp_pairs
            .drain(..)
            .zip(timestamps.chunks_exact(2))
        {
            let duration_nanos =
                timestamp_period * start_and_end[1].wrapping_sub(start_and_end[0]) as f64;
            aggregate_duration_nanos += duration_nanos;
            self.last_timing_results
                .push((tag, Duration::from_nanos(duration_nanos.round() as u64)));
        }

        self.last_timing_results.push((
            Cow::Borrowed("Aggregate"),
            Duration::from_nanos(aggregate_duration_nanos.round() as u64),
        ));

        let start_to_end_duration_nanos =
            timestamp_period * timestamps.last().unwrap().wrapping_sub(timestamps[0]) as f64;

        self.last_timing_results.push((
            Cow::Borrowed("Start to end"),
            Duration::from_nanos(start_to_end_duration_nanos.round() as u64),
        ));

        Ok(())
    }

    /// Returns the tag and duration of each timestamp pair as computed in the
    /// last call to [`Self::load_recorded_timing_results`].
    ///
    /// The last two entries are the aggregate duration of all timestamp pairs
    /// and the duration between the first and last of all the timestamps.
    pub fn last_timing_results(&self) -> &[(Cow<'static, str>, Duration)] {
        &self.last_timing_results
    }

    fn finish_recording(&mut self, command_encoder: &mut wgpu::CommandEncoder) {
        if !self.enabled || self.timestamp_pairs.is_empty() {
            return;
        }

        let query_range = 0..(2 * self.timestamp_pairs.len() as u32);

        let n_valid_bytes = query_range.end * wgpu::QUERY_SIZE;
        self.query_resolve_buffer
            .set_n_valid_bytes(n_valid_bytes as usize);

        command_encoder.resolve_query_set(
            self.query_set.as_ref().unwrap(),
            query_range,
            self.query_resolve_buffer.buffer(),
            0,
        );
        buffer::encode_buffer_to_buffer_copy_command(
            command_encoder,
            &self.query_resolve_buffer,
            &self.timestamp_result_buffer,
        );
    }

    fn register_writes_and_get_query_indices(
        &mut self,
        tag: Cow<'static, str>,
    ) -> Option<(u32, u32)> {
        if !self.enabled {
            return None;
        }

        let idx = self.next_timestamp_pair_idx_if_valid();

        self.timestamp_pairs.push(tag);

        Some((2 * idx, 2 * idx + 1))
    }

    fn next_timestamp_pair_idx_if_valid(&self) -> u32 {
        let idx = self.timestamp_pairs.len() as u32;
        self.assert_timestamp_pair_idx_is_valid(idx);
        idx
    }

    fn assert_timestamp_pair_idx_is_valid(&self, idx: u32) {
        assert!(
            2 * idx < self.max_timestamps.get(),
            "Tried to write too many timestamps (max timestamps: {})",
            self.max_timestamps
        );
    }
}

impl TimestampQueryRegistry<'_> {
    /// Registers a pair of timestamp writes for a render pass, one at the
    /// beginning of the pass and one at the end. Returns the `timestamp_writes`
    /// parameter to use in the [`wgpu::RenderPassDescriptor`] for the pass.
    pub fn register_timestamp_writes_for_single_render_pass(
        &mut self,
        tag: Cow<'static, str>,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let (start_idx, end_idx) = self.manager.register_writes_and_get_query_indices(tag)?;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: self.manager.query_set.as_ref().unwrap(),
            beginning_of_pass_write_index: Some(start_idx),
            end_of_pass_write_index: Some(end_idx),
        })
    }

    /// Registers a pair of timestamp writes for a sequence of the given number
    /// of render passes, one at the beginning of the first pass and one at
    /// the end of the last pass. Returns the two `timestamp_writes`
    /// parameters to use in the [`wgpu::RenderPassDescriptor`]s for the
    /// first and last passes.
    ///
    /// # Panics
    /// If `n_passes` is zero.
    pub fn register_timestamp_writes_for_first_and_last_of_render_passes(
        &mut self,
        n_passes: usize,
        tag: Cow<'static, str>,
    ) -> [Option<wgpu::RenderPassTimestampWrites<'_>>; 2] {
        assert!(n_passes > 0);
        if let Some((start_idx, end_idx)) = self.manager.register_writes_and_get_query_indices(tag)
        {
            let (end_of_first_pass_write_index, end_of_last_pass_write_index) = if n_passes == 1 {
                (Some(end_idx), None)
            } else {
                (None, Some(end_idx))
            };
            let query_set = self.manager.query_set.as_ref().unwrap();
            [
                Some(wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(start_idx),
                    end_of_pass_write_index: end_of_first_pass_write_index,
                }),
                Some(wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: end_of_last_pass_write_index,
                }),
            ]
        } else {
            [None, None]
        }
    }

    /// Registers a pair of timestamp writes for a compute pass, one at the
    /// beginning of the pass and one at the end. Returns the `timestamp_writes`
    /// parameter to use in the [`wgpu::ComputePassDescriptor`] for the pass.
    pub fn register_timestamp_writes_for_single_compute_pass(
        &mut self,
        tag: Cow<'static, str>,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        let (start_idx, end_idx) = self.manager.register_writes_and_get_query_indices(tag)?;
        Some(wgpu::ComputePassTimestampWrites {
            query_set: self.manager.query_set.as_ref().unwrap(),
            beginning_of_pass_write_index: Some(start_idx),
            end_of_pass_write_index: Some(end_idx),
        })
    }

    /// Registers a pair of timestamp writes for a sequence of the given number
    /// of compute passes, one at the beginning of the first pass and one at
    /// the end of the last pass. Returns the two `timestamp_writes`
    /// parameters to use in the [`wgpu::ComputePassDescriptor`]s for the
    /// first and last passes.
    ///
    /// # Panics
    /// If `n_passes` is zero.
    pub fn register_timestamp_writes_for_first_and_last_of_compute_passes(
        &mut self,
        n_passes: usize,
        tag: Cow<'static, str>,
    ) -> [Option<wgpu::ComputePassTimestampWrites<'_>>; 2] {
        if let Some((start_idx, end_idx)) = self.manager.register_writes_and_get_query_indices(tag)
        {
            let (end_of_first_pass_write_index, end_of_last_pass_write_index) = if n_passes == 1 {
                (Some(end_idx), None)
            } else {
                (None, Some(end_idx))
            };
            let query_set = self.manager.query_set.as_ref().unwrap();
            [
                Some(wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(start_idx),
                    end_of_pass_write_index: end_of_first_pass_write_index,
                }),
                Some(wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: end_of_last_pass_write_index,
                }),
            ]
        } else {
            [None, None]
        }
    }

    /// Drops this registry and records the required commands for resolving the
    /// registered timestamp queries and making the recorded timestamps
    /// available.
    pub fn finish(self, command_encoder: &mut wgpu::CommandEncoder) {
        self.manager.finish_recording(command_encoder);
    }
}

impl GPUBuffer {
    /// Creates a new query GPU buffer with capacity for the given number of
    /// query results.
    ///
    /// # Panics
    /// If `n_queries` exceeds [`wgpu::QUERY_SET_MAX_QUERIES`].
    pub fn new_query_buffer(
        graphics_device: &GraphicsDevice,
        n_queries: NonZeroU32,
        label: Cow<'static, str>,
    ) -> Self {
        assert!(n_queries.get() <= wgpu::QUERY_SET_MAX_QUERIES);
        let buffer_size = (n_queries.get() * wgpu::QUERY_SIZE) as usize;
        Self::new_uninitialized(
            graphics_device,
            buffer_size,
            GPUBufferType::Query.usage(),
            label,
        )
    }
}

/// Prints a nicely formatted table of the given timings obtained from
/// [`TimestampQueryManager::load_recorded_timing_results`].
pub fn print_timing_results(timings: &[(Cow<'_, str>, Duration)]) {
    if timings.is_empty() {
        return;
    }

    let longest_tag_len = timings.iter().map(|(tag, _)| tag.len()).max().unwrap();
    let total_width = longest_tag_len + 11;

    if !timings.is_empty() {
        let title_text = " GPU timing results ";
        let asterisks_per_side = (total_width - title_text.len()) / 2;
        let mut title = String::with_capacity(total_width);
        title.extend(iter::repeat_n('*', asterisks_per_side));
        title.push_str(title_text);
        title.extend(iter::repeat_n('*', asterisks_per_side));
        if title.len() < total_width {
            title.push('*');
        }
        println!("{title}");
    }
    for (tag, duration) in timings {
        println!(
            "{:_<width$}_{:_>7.1} µs",
            tag,
            1e6 * duration.as_secs_f64(),
            width = longest_tag_len
        );
    }
}
