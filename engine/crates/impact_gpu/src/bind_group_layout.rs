//! Centralized registry for bind group layouts.

use impact_containers::NoHashMap;
use impact_math::Hash64;
use parking_lot::RwLock;

/// A registry for bind group layouts that provides caching and proper cleanup.
#[derive(Debug)]
pub struct BindGroupLayoutRegistry {
    layouts: RwLock<NoHashMap<Hash64, wgpu::BindGroupLayout>>,
}

impl BindGroupLayoutRegistry {
    /// Creates a new bind group layout registry.
    pub fn new() -> Self {
        Self {
            layouts: RwLock::new(NoHashMap::default()),
        }
    }

    /// Gets or creates a bind group layout for the given ID and creation
    /// function.
    ///
    /// The creation function will only be called if the layout doesn't already
    /// exist.
    pub fn get_or_create_layout<F>(&self, id: Hash64, create_fn: F) -> wgpu::BindGroupLayout
    where
        F: FnOnce() -> wgpu::BindGroupLayout,
    {
        // First try to get with read lock
        {
            let layouts = self.layouts.read();
            if let Some(layout) = layouts.get(&id) {
                return layout.clone();
            }
        }

        // If not found, get write lock and create
        let mut layouts = self.layouts.write();

        // Check again in case another thread created it while we waited for write lock
        if let Some(layout) = layouts.get(&id) {
            return layout.clone();
        }

        // Create and insert the layout
        let layout = create_fn();
        layouts.insert(id, layout.clone());
        layout
    }

    /// Returns the layout with the given ID if it exists.
    pub fn get_layout(&self, id: Hash64) -> Option<wgpu::BindGroupLayout> {
        self.layouts.read().get(&id).cloned()
    }

    /// Removes and retruns the layout with the given ID if it exists.
    pub fn remove_layout(&self, id: Hash64) -> Option<wgpu::BindGroupLayout> {
        self.layouts.write().remove(&id)
    }
}

impl Default for BindGroupLayoutRegistry {
    fn default() -> Self {
        Self::new()
    }
}
