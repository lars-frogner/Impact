//! Voxels.

#[macro_use]
mod macros;

pub mod chunks;
pub mod collidable;
pub mod generation;
pub mod interaction;
pub mod mesh;
pub mod render_commands;
pub mod resource;
pub mod setup;
pub mod shader_templates;
pub mod utils;
pub mod voxel_types;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use chunks::inertia::VoxelObjectInertialPropertyManager;
use impact_containers::HashMap;
use impact_model::{InstanceFeatureManager, impl_InstanceFeature};
use impact_physics::rigid_body::DynamicRigidBodyID;
use mesh::MeshedChunkedVoxelObject;
use roc_integration::roc;
use std::{
    fmt,
    hash::Hash,
    path::{Path, PathBuf},
};
use utils::{Dimension, Side};
use voxel_types::{VoxelType, VoxelTypeRegistry};

define_component_type! {
    /// Identifier for a
    /// [`ChunkedVoxelObject`](crate::chunks::ChunkedVoxelObject) in a
    /// [`VoxelManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct VoxelObjectID(u32);
}

/// A voxel, which may either be be empty or filled with a material with
/// specific properties.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Voxel {
    voxel_type: VoxelType,
    signed_distance: VoxelSignedDistance,
    flags: VoxelFlags,
}

/// A compact encoding of a signed distance for a voxel.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Zeroable, Pod)]
pub struct VoxelSignedDistance {
    encoded: i8,
}

bitflags! {
    /// Bitflags encoding a set of potential binary states for a voxel.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoxelPlacement {
    Interior,
    Surface(VoxelSurfacePlacement),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoxelSurfacePlacement {
    Face,
    Edge,
    Corner,
}

/// Manager of voxels in a scene.
#[derive(Debug)]
pub struct VoxelManager {
    pub type_registry: VoxelTypeRegistry,
    pub object_manager: VoxelObjectManager,
}

/// Manager of all [`ChunkedVoxelObject`](crate::chunks::ChunkedVoxelObject)s in
/// a scene.
#[derive(Debug)]
pub struct VoxelObjectManager {
    voxel_objects: HashMap<VoxelObjectID, MeshedChunkedVoxelObject>,
    physics_contexts: HashMap<VoxelObjectID, VoxelObjectPhysicsContext>,
    id_counter: u32,
}

/// Physics context for voxel objects that participate in dynamic rigid body
/// simulation.
#[derive(Debug)]
pub struct VoxelObjectPhysicsContext {
    pub inertial_property_manager: VoxelObjectInertialPropertyManager,
    pub rigid_body_id: DynamicRigidBodyID,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct VoxelConfig {
    /// Path to the RON file containing the specification of each voxel type.
    #[cfg_attr(feature = "serde", serde(default))]
    pub voxel_types_path: Option<PathBuf>,
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

    const fn placement(self) -> VoxelPlacement {
        const PLACEMENTS: [VoxelPlacement; 7] = [
            VoxelPlacement::Surface(VoxelSurfacePlacement::Corner),
            VoxelPlacement::Surface(VoxelSurfacePlacement::Corner),
            VoxelPlacement::Surface(VoxelSurfacePlacement::Corner),
            VoxelPlacement::Surface(VoxelSurfacePlacement::Corner),
            VoxelPlacement::Surface(VoxelSurfacePlacement::Edge),
            VoxelPlacement::Surface(VoxelSurfacePlacement::Face),
            VoxelPlacement::Interior,
        ];
        let n_blocked_faces = (self.bits() & 0b11111100).count_ones();
        PLACEMENTS[n_blocked_faces as usize]
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

    /// Creates a new non-empty voxel of the given type with the given signed
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

    /// Whether this voxel has the same type and flags as the given voxel.
    pub fn matches_type_and_flags(&self, other: Self) -> bool {
        self.voxel_type == other.voxel_type && self.flags == other.flags
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

    /// Returns the placement of the voxel if it is non-empty.
    pub fn placement(&self) -> Option<VoxelPlacement> {
        if self.is_empty() {
            None
        } else {
            Some(self.flags.placement())
        }
    }

    /// Returns the signed distance from the center of the voxel to the
    /// nearest surface of the object.
    pub fn signed_distance(&self) -> VoxelSignedDistance {
        self.signed_distance
    }

    /// Increases the signed distance by the given amount, and marks the voxel
    /// as empty and calls the given closure if the signed distance becomes
    /// positive.
    pub fn increase_signed_distance(
        &mut self,
        signed_distance_delta: f32,
        on_empty: &mut impl FnMut(&Self),
    ) {
        let new_signed_distance = self.signed_distance.to_f32() + signed_distance_delta;
        self.signed_distance = VoxelSignedDistance::from_f32(new_signed_distance);
        if !self.signed_distance.is_negative() {
            self.add_flags(VoxelFlags::IS_EMPTY);
            on_empty(self);
        }
    }

    /// Updates the voxel's state flags to the given set of flags.
    fn update_flags(&mut self, flags: VoxelFlags) {
        self.flags = flags;
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

impl_InstanceFeature!(VoxelObjectID);

impl VoxelManager {
    /// Creates a new voxel manager with the given registry of voxel types.
    pub fn new(voxel_type_registry: VoxelTypeRegistry) -> Self {
        Self {
            type_registry: voxel_type_registry,
            object_manager: VoxelObjectManager::new(),
        }
    }

    /// Creates a new voxel manager based on the given configuration
    /// parameters.
    #[cfg(feature = "ron")]
    pub fn from_config(voxel_config: VoxelConfig) -> anyhow::Result<Self> {
        let voxel_type_registry = match voxel_config.voxel_types_path {
            Some(file_path) => VoxelTypeRegistry::from_voxel_type_ron_file(file_path)?,
            None => VoxelTypeRegistry::new(voxel_types::VoxelTypeSpecifications::default())?,
        };
        Ok(Self::new(voxel_type_registry))
    }
}

impl VoxelObjectManager {
    /// Creates a new voxel object manager with no objects.
    pub fn new() -> Self {
        Self {
            voxel_objects: HashMap::default(),
            physics_contexts: HashMap::default(),
            id_counter: 1,
        }
    }

    /// Returns the number of voxel objects in the manager.
    pub fn voxel_object_count(&self) -> usize {
        self.voxel_objects.len()
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

    /// Returns a reference to the [`VoxelObjectPhysicsContext`] for the voxel
    /// object with the given ID, or [`None`] if the object or physics context
    /// is not present.
    pub fn get_physics_context(
        &self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&VoxelObjectPhysicsContext> {
        self.physics_contexts.get(&voxel_object_id)
    }

    /// Returns mutable references to the [`MeshedChunkedVoxelObject`] with the
    /// given ID and its [`VoxelObjectPhysicsContext`] if both exist.
    pub fn get_voxel_object_with_physics_context_mut(
        &mut self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<(
        &mut MeshedChunkedVoxelObject,
        &mut VoxelObjectPhysicsContext,
    )> {
        let voxel_object = self.voxel_objects.get_mut(&voxel_object_id)?;
        let physics_context = self.physics_contexts.get_mut(&voxel_object_id)?;
        Some((voxel_object, physics_context))
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
    /// A new [`VoxelObjectID`] representing the added voxel object.
    pub fn add_voxel_object(&mut self, voxel_object: MeshedChunkedVoxelObject) -> VoxelObjectID {
        let voxel_object_id = self.create_new_voxel_object_id();
        self.voxel_objects.insert(voxel_object_id, voxel_object);
        voxel_object_id
    }

    /// Adds the given [`VoxelObjectPhysicsContext`] for the voxel object with
    /// the given ID.
    pub fn add_physics_context_for_voxel_object(
        &mut self,
        voxel_object_id: VoxelObjectID,
        physics_context: VoxelObjectPhysicsContext,
    ) {
        self.physics_contexts
            .insert(voxel_object_id, physics_context);
    }

    /// Removes the [`MeshedChunkedVoxelObject`] with the given ID if it exists.
    /// Also removes any associated [`VoxelObjectInertialPropertyManager`].
    pub fn remove_voxel_object(&mut self, voxel_object_id: VoxelObjectID) {
        self.voxel_objects.remove(&voxel_object_id);
        self.physics_contexts.remove(&voxel_object_id);
    }

    /// Removes all voxel objects in the manager.
    pub fn remove_all_voxel_objects(&mut self) {
        self.voxel_objects.clear();
    }

    /// Recomputes the meshes for any voxel objects with invalidated chunks.
    pub fn sync_voxel_object_meshes(&mut self, desynchronized: &mut bool) {
        for voxel_object in self.voxel_objects.values_mut() {
            voxel_object.sync_mesh_with_object(desynchronized);
        }
    }

    fn create_new_voxel_object_id(&mut self) -> VoxelObjectID {
        let voxel_object_id = VoxelObjectID(self.id_counter);
        self.id_counter = self.id_counter.checked_add(1).unwrap();
        voxel_object_id
    }
}

impl Default for VoxelObjectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(voxel_types_path) = self.voxel_types_path.as_mut() {
            *voxel_types_path = root_path.join(&voxel_types_path);
        }
    }
}

pub fn register_voxel_feature_types<MID: Clone + Eq + Hash>(
    instance_feature_manager: &mut InstanceFeatureManager<MID>,
) {
    instance_feature_manager.register_feature_type::<VoxelObjectID>();
}
