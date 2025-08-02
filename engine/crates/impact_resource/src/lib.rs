//! Resource management.

pub mod gpu;
pub mod registry;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use std::{fmt, hash::Hash};

/// A resource that can be stored in a
/// [`ResourceRegistry`](registry::ResourceRegistry).
pub trait Resource {
    type ID: ResourceID;
}

/// A [`Resource`] that can be mutated.
pub trait MutableResource: Resource {
    type DirtyMask: ResourceDirtyMask;
}

/// A type used as identifiers for a type of resource.
pub trait ResourceID: Copy + Eq + Hash + fmt::Debug {}

/// A bitmask indicating which parts of a resource have been modified.
pub trait ResourceDirtyMask: Copy + Eq + fmt::Debug {
    /// Returns a mask with no dirty flags set.
    fn empty() -> Self;

    /// Returns a mask with all dirty flags set.
    fn full() -> Self;
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

impl ResourceDirtyMask for BinaryDirtyMask {
    fn empty() -> Self {
        Self::empty()
    }

    fn full() -> Self {
        Self::ALL
    }
}
