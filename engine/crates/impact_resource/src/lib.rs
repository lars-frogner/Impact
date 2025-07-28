//! Resource management.

#[macro_use]
pub mod macros;

pub mod gpu;
pub mod index;
pub mod indexed_registry;
pub mod registry;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_containers::SlotKey;
use std::{fmt, hash::Hash};

/// A resource that can be stored in a
/// [`ResourceRegistry`](registry::ResourceRegistry).
pub trait Resource {
    type Handle: ResourceHandle;
}

/// A [`Resource`] that can be mutated.
pub trait MutableResource: Resource {
    type DirtyMask: ResourceDirtyMask;
}

/// A persistent identifier for a resource that remains valid across sessions.
pub trait ResourcePID: Copy + Eq + Hash + fmt::Display {}

/// A handle to a resource within a
/// [`ResourceRegistry`](registry::ResourceRegistry).
pub trait ResourceHandle:
    Copy + From<SlotKey> + Into<SlotKey> + Eq + Hash + fmt::Debug + fmt::Display
{
}

/// A bitmask indicating which parts of a resource have been modified.
pub trait ResourceDirtyMask: Copy + Eq + fmt::Debug {
    /// Returns a mask with no dirty flags set.
    fn empty() -> Self;

    /// Returns a mask with all dirty flags set.
    fn full() -> Self;
}

/// Provides human-readable labels for resource handles.
pub trait ResourceLabelProvider<H> {
    /// Creates a label for the given resource handle.
    fn create_label(&self, handle: H) -> String;
}

bitflags! {
    /// A simple dirty mask with a single flag indicating that the entire
    /// resource is dirty.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct BinaryDirtyMask: u8 {
        const ALL = 1 << 0;
    }
}

/// A label provider that uses the handle's [`Display`](fmt::Display)
/// implementation.
#[derive(Clone, Copy, Debug)]
pub struct HandleLabelProvider;

impl ResourceDirtyMask for BinaryDirtyMask {
    fn empty() -> Self {
        Self::empty()
    }

    fn full() -> Self {
        Self::ALL
    }
}

impl<H: fmt::Display> ResourceLabelProvider<H> for HandleLabelProvider {
    fn create_label(&self, handle: H) -> String {
        handle.to_string()
    }
}
