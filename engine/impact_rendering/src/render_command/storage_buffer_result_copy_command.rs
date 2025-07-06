//! Command for copying the contents of a storage buffer into its associated
//! result buffer.

use anyhow::{Result, anyhow};
use impact_gpu::{
    storage::{StorageBufferID, StorageGPUBufferManager},
    wgpu,
};

/// Recorder for a command copying the contents of a storage buffer into its
/// associated result buffer (which can be mapped to the CPU).
#[derive(Debug)]
pub struct StorageBufferResultCopyCommand {
    buffer_id: StorageBufferID,
}

impl StorageBufferResultCopyCommand {
    /// Creates a new result copy command for the storage buffer with the given
    /// ID.
    pub fn new(buffer_id: StorageBufferID) -> Self {
        Self { buffer_id }
    }

    /// Records the copy pass to the given command encoder.
    ///
    /// # Errors
    /// Returns an error if the storage buffer is not available or does not have
    /// a result buffer.
    pub fn record(
        &self,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let storage_buffer = storage_gpu_buffer_manager
            .get_storage_buffer(self.buffer_id)
            .ok_or_else(|| anyhow!("Missing storage buffer {}", self.buffer_id))?;

        storage_buffer.encode_copy_to_result_buffer(command_encoder)?;

        impact_log::trace!(
            "Recorded result copy command for storage buffer ({})",
            self.buffer_id
        );

        Ok(())
    }
}
