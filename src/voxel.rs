//! Voxels.

pub mod buffer;
pub mod chunks;
pub mod components;
pub mod entity;
pub mod generation;
pub mod mesh;
pub mod render_commands;
pub mod utils;

pub use entity::register_voxel_feature_types;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use chunks::ChunkedVoxelObject;
use num_derive::{FromPrimitive as DeriveFromPrimitive, ToPrimitive as DeriveToPrimitive};
use num_traits::FromPrimitive;
use std::fmt;
use std::{array, collections::HashMap};
use utils::{Dimension, Side};

/// A type identifier that determines all the properties of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DeriveToPrimitive, DeriveFromPrimitive)]
pub enum VoxelType {
    Default = 0,
}

/// The total number of separate [`VoxelType`]s.
const N_VOXEL_TYPES: usize = 1;

/// Identifier for predefined set of voxel properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoxelPropertyID(u8);

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
    property_id: VoxelPropertyID,
    signed_distance: VoxelSignedDistance,
    flags: VoxelFlags,
}

/// A mapping from voxel types to the corresponding values of a specific voxel
/// property.
#[derive(Debug)]
pub struct VoxelPropertyMap<P> {
    property_values: [P; N_VOXEL_TYPES],
}

/// Identifier for a [`ChunkedVoxelObject`] in a [`VoxelManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct VoxelObjectID(u32);

/// Manager of all [`ChunkedVoxelObject`]s in a scene.
#[derive(Debug)]
pub struct VoxelManager {
    voxel_objects: HashMap<VoxelObjectID, ChunkedVoxelObject>,
    voxel_object_id_counter: u32,
}

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f64;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel at the given indices in a voxel grid. If the indices
    /// are outside the bounds of the grid, this should return
    /// [`Voxel::fully_outside`].
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel;
}

impl VoxelType {
    /// Returns an array with each voxel type in the order of their index.
    pub fn all() -> [Self; N_VOXEL_TYPES] {
        array::from_fn(|idx| Self::from_usize(idx).unwrap())
    }
}

impl VoxelPropertyID {
    /// Creates a new property ID for the given `VoxelType`.
    pub const fn from_voxel_type(voxel_type: VoxelType) -> Self {
        Self(voxel_type as u8)
    }

    const fn dummy() -> Self {
        Self(u8::MAX)
    }
}

impl From<VoxelType> for VoxelPropertyID {
    fn from(voxel_type: VoxelType) -> Self {
        Self::from_voxel_type(voxel_type)
    }
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

    /// A `SignedDistance` for a voxel that is fully outside the object.
    pub const fn fully_outside() -> Self {
        Self::from_encoded(i8::MAX)
    }

    /// A `SignedDistance` for a voxel that is fully inside the object.
    pub const fn fully_inside() -> Self {
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
    pub fn is_negative(self) -> bool {
        self.encoded < 0
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
    /// Creates a new voxel with the given property ID, state flags and signed
    /// distance.
    const fn new(
        property_id: VoxelPropertyID,
        flags: VoxelFlags,
        signed_distance: VoxelSignedDistance,
    ) -> Self {
        Self {
            property_id,
            flags,
            signed_distance,
        }
    }

    /// Creates a new voxel with the given property ID and signed distance that
    /// is near the surface of the object (it is adjacent to a voxel with an
    /// opposite signed distance).
    pub const fn near_surface(
        property_id: VoxelPropertyID,
        signed_distance: VoxelSignedDistance,
    ) -> Self {
        Self {
            property_id,
            flags: VoxelFlags::new(),
            signed_distance,
        }
    }

    /// Creates a new voxel with the given property ID that is fully inside the
    /// object (not adjacent to a voxel whose center is outside the object's
    /// surface).
    pub const fn fully_inside(property_id: VoxelPropertyID) -> Self {
        Self::new(
            property_id,
            VoxelFlags::new(),
            VoxelSignedDistance::fully_inside(),
        )
    }

    /// Creates a new empty voxel that is fully outside the object (not adjacent
    /// to a voxel whose center is inside the object's surface).
    pub const fn fully_outside() -> Self {
        Self::new(
            VoxelPropertyID::dummy(),
            VoxelFlags::IS_EMPTY,
            VoxelSignedDistance::fully_outside(),
        )
    }

    /// Whether the voxel is empty.
    pub fn is_empty(&self) -> bool {
        self.flags.contains(VoxelFlags::IS_EMPTY)
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

impl<P> VoxelPropertyMap<P> {
    /// Creates a new voxel property map using the given property values, with
    /// the value for a given voxel type residing at the numerical value of the
    /// corresponding [`VoxelType`] enum variant.
    pub fn new(property_values: [P; N_VOXEL_TYPES]) -> Self {
        Self { property_values }
    }

    /// Returns a reference to the property value for the given voxel type.
    pub fn value(&self, voxel_type: VoxelType) -> &P {
        &self.property_values[voxel_type as usize]
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
    pub fn new() -> Self {
        Self {
            voxel_objects: HashMap::new(),
            voxel_object_id_counter: 1,
        }
    }

    /// Returns a reference to the [`ChunkedVoxelObject`] with the given ID, or
    /// [`None`] if the voxel object is not present.
    pub fn get_voxel_object(&self, voxel_object_id: VoxelObjectID) -> Option<&ChunkedVoxelObject> {
        self.voxel_objects.get(&voxel_object_id)
    }

    /// Returns a mutable reference to the [`ChunkedVoxelObject`] with the given
    /// ID, or [`None`] if the voxel object is not present.
    pub fn get_voxel_object_mut(
        &mut self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&mut ChunkedVoxelObject> {
        self.voxel_objects.get_mut(&voxel_object_id)
    }

    /// Whether a voxel object with the given ID exists in the manager.
    pub fn has_voxel_object(&self, voxel_object_id: VoxelObjectID) -> bool {
        self.voxel_objects.contains_key(&voxel_object_id)
    }

    /// Returns a reference to the [`HashMap`] storing all voxel objects.
    pub fn voxel_objects(&self) -> &HashMap<VoxelObjectID, ChunkedVoxelObject> {
        &self.voxel_objects
    }

    /// Adds the given [`ChunkedVoxelObject`] to the manager.
    ///
    /// # Returns
    /// A new [`ChunkedVoxelObjectID`] representing the added voxel object.
    pub fn add_voxel_object(&mut self, voxel_object: ChunkedVoxelObject) -> VoxelObjectID {
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

impl Default for VoxelManager {
    fn default() -> Self {
        Self::new()
    }
}
