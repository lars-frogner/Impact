//! Management of postprocessing data for rendering.

use crate::gpu::rendering::fre;
use std::mem;

/// Manager of render resources for postprocessing effects.
#[derive(Clone, Debug)]
pub struct PostprocessingResourceManager {
    exposure: fre,
}

impl PostprocessingResourceManager {
    pub const EXPOSURE_PUSH_CONSTANT_SIZE: u32 = mem::size_of::<fre>() as u32;

    /// Returns the exposure value.
    pub fn exposure(&self) -> fre {
        self.exposure
    }

    /// Sets the exposure value.
    pub fn set_exposure(&mut self, exposure: fre) {
        self.exposure = exposure;
    }
}

impl Default for PostprocessingResourceManager {
    fn default() -> Self {
        Self { exposure: 1.0 }
    }
}
