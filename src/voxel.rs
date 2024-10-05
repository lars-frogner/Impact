//! Voxels.

pub mod chunks;
pub mod components;
pub mod entity;
pub mod generation;
pub mod mesh;
pub mod render_commands;
pub mod resource;
pub mod utils;
pub mod voxel_types;

pub use entity::register_voxel_feature_types;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use mesh::MeshedChunkedVoxelObject;
use std::{collections::HashMap, fmt};
use utils::{Dimension, Side};
use voxel_types::{VoxelType, VoxelTypeRegistry};

/// A compact encoding of a signed distance for a voxel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VoxelSignedDistance {
    encoded: i8,
}

bitflags! {
    /// Bitflags encoding a set of potential binary states for a voxel.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct VoxelFlags: u8 {
        /// The voxel is empty.
        const IS_EMPTY          = 1 << 0;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// x-direction.
        const HAS_ADJACENT_X_DN = 1 << 2;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// y-direction.
        const HAS_ADJACENT_Y_DN = 1 << 3;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// z-direction.
        const HAS_ADJACENT_Z_DN = 1 << 4;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// x-direction.
        const HAS_ADJACENT_X_UP = 1 << 5;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// y-direction.
        const HAS_ADJACENT_Y_UP = 1 << 6;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// z-direction.
        const HAS_ADJACENT_Z_UP = 1 << 7;
    }
}

/// A voxel, which may either be be empty or filled with a material with
/// specific properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voxel {
    voxel_type: VoxelType,
    signed_distance: VoxelSignedDistance,
    flags: VoxelFlags,
}

/// Identifier for a [`ChunkedVoxelObject`] in a [`VoxelManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct VoxelObjectID(u32);

/// Manager of all [`ChunkedVoxelObject`]s in a scene.
#[derive(Debug)]
pub struct VoxelManager {
    voxel_type_registry: VoxelTypeRegistry,
    voxel_objects: HashMap<VoxelObjectID, MeshedChunkedVoxelObject>,
    voxel_object_id_counter: u32,
}

impl VoxelSignedDistance {
    const QUANTIZATION_STEP_SIZE: f32 = 0.02;
    const INVERSE_QUANTIZATION_STEP_SIZE: f32 = 1.0 / Self::QUANTIZATION_STEP_SIZE;

    const MAX_F32: f32 = Self::QUANTIZATION_STEP_SIZE * i8::MAX as f32;
    const MIN_F32: f32 = Self::QUANTIZATION_STEP_SIZE * i8::MIN as f32;

    /// The maximum signed distance that can be represented by a
    /// [`VoxelSignedDistance`].
    pub const fn max_f32() -> f32 {
        Self::MAX_F32
    }

    /// The minimum (most negative) signed distance that can be represented by a
    /// [`VoxelSignedDistance`].
    pub const fn min_f32() -> f32 {
        Self::MIN_F32
    }

    /// A `SignedDistance` for a voxel that is maximally outside the object.
    pub const fn maximally_outside() -> Self {
        Self::from_encoded(i8::MAX)
    }

    /// A `SignedDistance` for a voxel that is maximally inside the object.
    pub const fn maximally_inside() -> Self {
        Self::from_encoded(i8::MIN)
    }

    const fn from_encoded(encoded: i8) -> Self {
        Self { encoded }
    }

    /// Encodes the given `f32` signed distance as a `VoxelSignedDistance`.
    /// The value will be clamped to [`Self::min_f32`] and [`Self::max_f32`].
    pub fn from_f32(value: f32) -> Self {
        // We don't need to clamp the value before casting to `i8` since
        // `as` will do that for us (for Rust 1.45+). `NaN` will result in `0`.
        Self::from_encoded((value * Self::INVERSE_QUANTIZATION_STEP_SIZE) as i8)
    }

    /// Decodes the `VoxelSignedDistance` to an `f32` signed distance.
    pub fn to_f32(self) -> f32 {
        f32::from(self.encoded) * Self::QUANTIZATION_STEP_SIZE
    }

    /// Whether the signed distance is strictly negative.
    pub const fn is_negative(self) -> bool {
        self.encoded.is_negative()
    }
}

impl From<f32> for VoxelSignedDistance {
    fn from(value: f32) -> Self {
        Self::from_f32(value)
    }
}

impl From<VoxelSignedDistance> for f32 {
    fn from(value: VoxelSignedDistance) -> Self {
        value.to_f32()
    }
}

impl fmt::Display for VoxelSignedDistance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_f32().fmt(f)
    }
}

impl VoxelFlags {
    const fn new() -> Self {
        Self::empty()
    }

    const fn full_adjacency() -> Self {
        Self::HAS_ADJACENT_X_DN
            .union(Self::HAS_ADJACENT_X_UP)
            .union(Self::HAS_ADJACENT_Y_DN)
            .union(Self::HAS_ADJACENT_Y_UP)
            .union(Self::HAS_ADJACENT_Z_DN)
            .union(Self::HAS_ADJACENT_Z_UP)
    }

    const fn adjacency_for_face(face_dim: Dimension, face_side: Side) -> Self {
        const FLAGS: [VoxelFlags; 6] = [
            VoxelFlags::HAS_ADJACENT_X_DN,
            VoxelFlags::HAS_ADJACENT_X_UP,
            VoxelFlags::HAS_ADJACENT_Y_DN,
            VoxelFlags::HAS_ADJACENT_Y_UP,
            VoxelFlags::HAS_ADJACENT_Z_DN,
            VoxelFlags::HAS_ADJACENT_Z_UP,
        ];
        FLAGS[2 * face_dim.idx() + face_side.idx()]
    }
}

impl Voxel {
    /// Creates a new voxel with the given type, state flags and signed
    /// distance.
    const fn new(
        voxel_type: VoxelType,
        flags: VoxelFlags,
        signed_distance: VoxelSignedDistance,
    ) -> Self {
        Self {
            voxel_type,
            flags,
            signed_distance,
        }
    }

    /// Creates a new non-empty voxel of the given typewith the given signed
    /// distance.
    pub const fn non_empty(voxel_type: VoxelType, signed_distance: VoxelSignedDistance) -> Self {
        Self::new(voxel_type, VoxelFlags::new(), signed_distance)
    }

    /// Creates a new empty voxel with the given signed distance.
    pub const fn empty(signed_distance: VoxelSignedDistance) -> Self {
        Self::new(VoxelType::dummy(), VoxelFlags::IS_EMPTY, signed_distance)
    }

    /// Creates a new voxel with the given type that is maximally inside the
    /// object.
    pub const fn maximally_inside(voxel_type: VoxelType) -> Self {
        Self::new(
            voxel_type,
            VoxelFlags::new(),
            VoxelSignedDistance::maximally_inside(),
        )
    }

    /// Creates a new empty voxel that is maximally outside the object.
    pub const fn maximally_outside() -> Self {
        Self::new(
            VoxelType::dummy(),
            VoxelFlags::IS_EMPTY,
            VoxelSignedDistance::maximally_outside(),
        )
    }

    /// Whether the voxel is empty.
    pub fn is_empty(&self) -> bool {
        self.flags.contains(VoxelFlags::IS_EMPTY)
    }

    /// Returns the type of the voxel.
    pub fn voxel_type(&self) -> VoxelType {
        self.voxel_type
    }

    /// Returns the flags encoding the state of the voxel.
    pub fn flags(&self) -> VoxelFlags {
        self.flags
    }

    /// Returns the signed distance from the center of the voxel to the
    /// nearest surface of the object.
    pub fn signed_distance(&self) -> VoxelSignedDistance {
        self.signed_distance
    }

    /// Sets the given state flags for the voxel (this will not clear any
    /// existing flags).
    fn add_flags(&mut self, flags: VoxelFlags) {
        self.flags.insert(flags);
    }

    /// Unsets the given state flags for the voxel.
    fn remove_flags(&mut self, flags: VoxelFlags) {
        self.flags.remove(flags);
    }
}

#[cfg(test)]
impl VoxelObjectID {
    /// Creates a dummy [`ChunkedVoxelObjectID`] that will never match an actual
    /// ID returned from the [`VoxelManager`]. Used for testing purposes.
    pub fn dummy() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for VoxelObjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VoxelManager {
    /// Creates a new voxel manager with the given registry of voxel types.
    pub fn new(voxel_type_registry: VoxelTypeRegistry) -> Self {
        Self {
            voxel_type_registry,
            voxel_objects: HashMap::new(),
            voxel_object_id_counter: 1,
        }
    }

    /// Returns a reference to the [`VoxelTypeRegistry`].
    pub fn voxel_type_registry(&self) -> &VoxelTypeRegistry {
        &self.voxel_type_registry
    }

    /// Returns a reference to the [`MeshedChunkedVoxelObject`] with the given
    /// ID, or [`None`] if the voxel object is not present.
    pub fn get_voxel_object(
        &self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&MeshedChunkedVoxelObject> {
        self.voxel_objects.get(&voxel_object_id)
    }

    /// Returns a mutable reference to the [`MeshedChunkedVoxelObject`] with the
    /// given ID, or [`None`] if the voxel object is not present.
    pub fn get_voxel_object_mut(
        &mut self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&mut MeshedChunkedVoxelObject> {
        self.voxel_objects.get_mut(&voxel_object_id)
    }

    /// Whether a voxel object with the given ID exists in the manager.
    pub fn has_voxel_object(&self, voxel_object_id: VoxelObjectID) -> bool {
        self.voxel_objects.contains_key(&voxel_object_id)
    }

    /// Returns a reference to the [`HashMap`] storing all voxel objects.
    pub fn voxel_objects(&self) -> &HashMap<VoxelObjectID, MeshedChunkedVoxelObject> {
        &self.voxel_objects
    }

    /// Returns a mutable reference to the [`HashMap`] storing all voxel
    /// objects.
    pub fn voxel_objects_mut(&mut self) -> &mut HashMap<VoxelObjectID, MeshedChunkedVoxelObject> {
        &mut self.voxel_objects
    }

    /// Adds the given [`MeshedChunkedVoxelObject`] to the manager.
    ///
    /// # Returns
    /// A new [`ChunkedVoxelObjectID`] representing the added voxel object.
    pub fn add_voxel_object(&mut self, voxel_object: MeshedChunkedVoxelObject) -> VoxelObjectID {
        let voxel_object_id = self.create_new_voxel_object_id();
        self.voxel_objects.insert(voxel_object_id, voxel_object);
        voxel_object_id
    }

    /// Removes all voxel objects in the manager.
    pub fn remove_all_voxel_objects(&mut self) {
        self.voxel_objects.clear();
    }

    fn create_new_voxel_object_id(&mut self) -> VoxelObjectID {
        let voxel_object_id = VoxelObjectID(self.voxel_object_id_counter);
        self.voxel_object_id_counter = self.voxel_object_id_counter.checked_add(1).unwrap();
        voxel_object_id
    }
}
