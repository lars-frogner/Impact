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

use crate::{gpu::rendering::fre, model::transform::InstanceModelViewTransform, num::Float};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use chunks::ChunkedVoxelObject;
use nalgebra::{vector, Similarity3, UnitVector3, Vector3};
use num_derive::{FromPrimitive as DeriveFromPrimitive, ToPrimitive as DeriveToPrimitive};
use num_traits::FromPrimitive;
use simba::scalar::{SubsetOf, SupersetOf};
use std::{array, collections::HashMap};

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

/// The total number of separate [`VoxelType`]s.
const N_VOXEL_TYPES: usize = 1;

/// A type identifier that determines all the properties of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DeriveToPrimitive, DeriveFromPrimitive)]
pub enum VoxelType {
    Default = 0,
}

/// A mapping from voxel types to the corresponding values of a specific voxel
/// property.
#[derive(Debug)]
pub struct VoxelPropertyMap<P> {
    property_values: [P; N_VOXEL_TYPES],
}

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f64;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel type at the given indices in a voxel grid, or [`None`]
    /// if the voxel is absent or the indices are outside the bounds of the
    /// grid.
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType>;
}

/// One of the six faces of a voxel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum VoxelFace {
    LowerX = 0,
    UpperX = 1,
    LowerY = 2,
    UpperY = 3,
    LowerZ = 4,
    UpperZ = 5,
}

/// A transform from the space of a voxel in a multi-voxel model to the space of
/// the whole model.
#[derive(Clone, Debug, PartialEq)]
struct VoxelTransform<F: Float> {
    translation: Vector3<F>,
    scaling: F,
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

impl VoxelType {
    /// Returns an array with each voxel type in the order of their index.
    pub fn all() -> [Self; N_VOXEL_TYPES] {
        array::from_fn(|idx| Self::from_usize(idx).unwrap())
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

impl VoxelFace {
    const X_FACES: [Self; 2] = [Self::LowerX, Self::UpperX];
    const Y_FACES: [Self; 2] = [Self::LowerY, Self::UpperY];
    const Z_FACES: [Self; 2] = [Self::LowerZ, Self::UpperZ];

    fn opposite_face(&self) -> Self {
        match *self {
            Self::LowerX => Self::UpperX,
            Self::UpperX => Self::LowerX,
            Self::LowerY => Self::UpperY,
            Self::UpperY => Self::LowerY,
            Self::LowerZ => Self::UpperZ,
            Self::UpperZ => Self::LowerZ,
        }
    }
}

impl<F: Float> VoxelTransform<F> {
    /// Creates a new voxel transform with the given translation and scaling.
    fn new(translation: Vector3<F>, scaling: F) -> Self {
        Self {
            translation,
            scaling,
        }
    }

    /// Creates a new identity voxel transform.
    fn identity() -> Self {
        Self {
            translation: Vector3::zeros(),
            scaling: F::ONE,
        }
    }

    /// Returns a reference to the translational part of the voxel transform.
    #[cfg(test)]
    fn translation(&self) -> &Vector3<F> {
        &self.translation
    }

    /// Returns the scaling part of the voxel transform.
    #[cfg(test)]
    fn scaling(&self) -> F {
        self.scaling
    }

    /// Applies the given transform from the space of the multi-voxel model to
    /// camera space, yielding the model view transform of the voxel.
    fn transform_into_model_view_transform(
        &self,
        view_transform: &Similarity3<F>,
        camera_space_axes_in_model_space: &(UnitVector3<F>, UnitVector3<F>, UnitVector3<F>),
    ) -> InstanceModelViewTransform
    where
        F: SubsetOf<fre>,
    {
        let scaling_from_model_to_camera_space = view_transform.scaling();
        let rotation_from_model_to_camera_space = view_transform.isometry.rotation;
        let translation_from_model_to_camera_space = view_transform.isometry.translation.vector;

        let new_scaling = scaling_from_model_to_camera_space * self.scaling;

        let new_translation = translation_from_model_to_camera_space
            + vector![
                camera_space_axes_in_model_space.0.dot(&self.translation),
                camera_space_axes_in_model_space.1.dot(&self.translation),
                camera_space_axes_in_model_space.2.dot(&self.translation)
            ] * scaling_from_model_to_camera_space;

        InstanceModelViewTransform {
            rotation: rotation_from_model_to_camera_space.cast::<fre>(),
            translation: new_translation.cast::<fre>(),
            scaling: fre::from_subset(&new_scaling),
        }
    }

    fn compute_camera_space_axes_in_model_space(
        transform_from_model_to_camera_space: &Similarity3<F>,
    ) -> (UnitVector3<F>, UnitVector3<F>, UnitVector3<F>) {
        let rotation = &transform_from_model_to_camera_space.isometry.rotation;
        (
            rotation.inverse_transform_unit_vector(&Vector3::x_axis()),
            rotation.inverse_transform_unit_vector(&Vector3::y_axis()),
            rotation.inverse_transform_unit_vector(&Vector3::z_axis()),
        )
    }
}

impl<F: Float> Default for VoxelTransform<F> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<F> AbsDiffEq for VoxelTransform<F>
where
    F: Float + AbsDiffEq,
    <F as AbsDiffEq>::Epsilon: Clone,
{
    type Epsilon = <F as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <F as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Vector3::abs_diff_eq(&self.translation, &other.translation, epsilon)
            && F::abs_diff_eq(&self.scaling, &other.scaling, epsilon)
    }
}
