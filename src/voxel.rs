//! Voxels.

mod appearance;
pub mod components;
pub mod entity;
pub mod generation;
mod tree;

pub use tree::{VoxelTree, VoxelTreeLODController};

use crate::{
    assets::Assets,
    geometry::Radians,
    gpu::{rendering::fre, GraphicsDevice},
    material::MaterialLibrary,
    mesh::{FrontFaceSide, MeshID, MeshRepository, TriangleMesh},
    model::{
        transform::InstanceModelViewTransform, DynamicInstanceFeatureBuffer, InstanceFeatureID,
        InstanceFeatureManager, InstanceFeatureStorage,
    },
    num::Float,
};
use appearance::VoxelAppearance;
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, KeyIndexMapper};
use lazy_static::lazy_static;
use nalgebra::{vector, Similarity3, UnitVector3, Vector3};
use nohash_hasher::BuildNoHashHasher;
use num_derive::{FromPrimitive as DeriveFromPrimitive, ToPrimitive as DeriveToPrimitive};
use num_traits::FromPrimitive;
use simba::scalar::{SubsetOf, SupersetOf};
use std::{array, collections::HashMap};

/// Voxel configuration options.
#[derive(Clone, Debug)]
pub struct VoxelConfig<F> {
    pub voxel_extent: F,
    pub initial_min_angular_voxel_extent_for_lod: Radians<F>,
}

/// Identifier for a [`VoxelTree`] in a [`VoxelManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct VoxelTreeID(u32);

/// Manager of all [`VoxelTree`]s in a scene.
#[derive(Debug)]
pub struct VoxelManager<F: Float> {
    config: VoxelConfig<F>,
    voxel_appearances: VoxelPropertyMap<VoxelAppearance>,
    voxel_material_feature_ids: VoxelPropertyMap<InstanceFeatureID>,
    voxel_tree_lod_controller: VoxelTreeLODController<F>,
    voxel_trees: HashMap<VoxelTreeID, VoxelTree<F>>,
    voxel_tree_id_counter: u32,
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
pub trait VoxelGenerator<F: Float> {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> F;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel type at the given indices in a voxel grid, or [`None`]
    /// if the voxel is absent or the indices are outside the bounds of the
    /// grid.
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType>;

    /// Returns the height above the bottom of the voxel tree below which
    /// decisions on whether to pass voxel instances to the GPU should be made
    /// for entire octants at once.
    fn instance_group_height(&self) -> u32 {
        0
    }
}

/// Storage for voxel instance data to be passed to the GPU.
#[derive(Clone, Debug)]
pub struct VoxelInstanceStorage<F: Float> {
    voxel_types: Vec<VoxelType>,
    transforms: Vec<VoxelTransform<F>>,
    /// Maps [`VoxelInstanceID`]s to indices in `voxel_types` and `transforms`.
    index_map: KeyIndexMapper<usize, BuildNoHashHasher<usize>>,
}

/// A collection of [`VoxelInstanceStorage`]s for holding separate groups of
/// voxel instances.
#[derive(Clone, Debug)]
struct GroupedVoxelInstanceStorage<F: Float> {
    /// Each group may contain one [`VoxelInstanceStorage`] for each level of
    /// detail that the voxel instances in the group can be rendered at.
    groups: Vec<Vec<VoxelInstanceStorage<F>>>,
    /// Maps [`InstanceGroupID`]s to indices in `groups`.
    index_map: KeyIndexMapper<usize, BuildNoHashHasher<usize>>,
    instance_id_count: usize,
    group_id_count: usize,
}

pub type CoordinateAxes<F> = (UnitVector3<F>, UnitVector3<F>, UnitVector3<F>);

/// Identifier for a voxel instance in a [`VoxelInstanceStorage`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct VoxelInstanceID(usize);

/// Identifier for a group of [`VoxelInstanceStorage`]s in a
/// [`GroupedVoxelInstanceStorage`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstanceGroupID(usize);

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

lazy_static! {
    /// The ID of the [`TriangleMesh`] in the [`MeshRepository`] representing a
    /// standard voxel.
    pub static ref VOXEL_MESH_ID: MeshID = MeshID(hash64!("VoxelMesh"));
}

impl<F: Float> Default for VoxelConfig<F> {
    fn default() -> Self {
        Self {
            voxel_extent: F::from_f64(0.25).unwrap(),
            initial_min_angular_voxel_extent_for_lod: Radians(F::ZERO),
        }
    }
}

#[cfg(test)]
impl VoxelTreeID {
    /// Creates a dummy [`VoxelTreeID`] that will never match an actual ID
    /// returned from the [`VoxelManager`]. Used for testing purposes.
    pub fn dummy() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for VoxelTreeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<F: Float> VoxelManager<F> {
    /// Returns a reference to the voxel configuration options.
    pub fn config(&self) -> &VoxelConfig<F> {
        &self.config
    }

    /// Returns a reference to the map from voxel types to material property
    /// feature IDs.
    pub fn voxel_material_feature_ids(&self) -> &VoxelPropertyMap<InstanceFeatureID> {
        &self.voxel_material_feature_ids
    }

    /// Returns a reference to the [`VoxelAppearance`] for the given voxel type.
    pub fn voxel_appearance(&self, voxel_type: VoxelType) -> &VoxelAppearance {
        self.voxel_appearances.value(voxel_type)
    }

    /// Returns a reference to the voxel tree LOD controller.
    pub fn voxel_tree_lod_controller(&self) -> &VoxelTreeLODController<F> {
        &self.voxel_tree_lod_controller
    }

    /// Returns a reference to the [`VoxelTree`] with the given ID, or [`None`]
    /// if the voxel tree is not present.
    pub fn get_voxel_tree(&self, voxel_tree_id: VoxelTreeID) -> Option<&VoxelTree<F>> {
        self.voxel_trees.get(&voxel_tree_id)
    }

    /// Scales the minimum angular voxel extent in the voxel tree LOD controller
    /// by the given factor. The extent should be scaled to remain proportional
    /// to the field of view and inversely proportional to the number of pixels
    /// across the window.
    pub fn scale_min_angular_voxel_extent_for_lod(&mut self, scale: F) {
        self.voxel_tree_lod_controller
            .scale_min_angular_voxel_extent(scale);
    }

    /// Returns a mutable reference to the [`VoxelTree`] with the given ID, or
    /// [`None`] if the voxel tree is not present.
    pub fn get_voxel_tree_mut(&mut self, voxel_tree_id: VoxelTreeID) -> Option<&mut VoxelTree<F>> {
        self.voxel_trees.get_mut(&voxel_tree_id)
    }

    /// Whether a voxel tree with the given ID exists in the manager.
    pub fn has_voxel_tree(&self, voxel_tree_id: VoxelTreeID) -> bool {
        self.voxel_trees.contains_key(&voxel_tree_id)
    }

    /// Returns a reference to the [`HashMap`] storing all voxel trees.
    pub fn voxel_trees(&self) -> &HashMap<VoxelTreeID, VoxelTree<F>> {
        &self.voxel_trees
    }

    /// Adds the given [`VoxelTree`] to the manager.
    ///
    /// # Returns
    /// A new [`VoxelTreeID`] representing the added voxel tree.
    pub fn add_voxel_tree(&mut self, voxel_tree: VoxelTree<F>) -> VoxelTreeID {
        let voxel_tree_id = self.create_new_voxel_tree_id();
        self.voxel_trees.insert(voxel_tree_id, voxel_tree);
        voxel_tree_id
    }

    /// Removes all voxel trees in the manager.
    pub fn remove_all_voxel_trees(&mut self) {
        self.voxel_trees.clear();
    }

    fn create_new_voxel_tree_id(&mut self) -> VoxelTreeID {
        let voxel_tree_id = VoxelTreeID(self.voxel_tree_id_counter);
        self.voxel_tree_id_counter = self.voxel_tree_id_counter.checked_add(1).unwrap();
        voxel_tree_id
    }
}

impl VoxelManager<fre> {
    pub fn create(
        config: VoxelConfig<fre>,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        mesh_repository: &mut MeshRepository<fre>,
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) -> Self {
        mesh_repository.add_mesh_unless_present(
            *VOXEL_MESH_ID,
            TriangleMesh::create_box(
                config.voxel_extent,
                config.voxel_extent,
                config.voxel_extent,
                FrontFaceSide::Outside,
            ),
        );

        let voxel_appearances = VoxelType::all().map(|voxel_type| {
            VoxelAppearance::setup(
                voxel_type,
                graphics_device,
                assets,
                material_library,
                instance_feature_manager,
            )
        });

        let voxel_material_feature_ids = voxel_appearances
            .iter()
            .map(|appearance| {
                appearance
                    .material_handle
                    .material_property_feature_id()
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let voxel_tree_lod_controller =
            VoxelTreeLODController::new(config.initial_min_angular_voxel_extent_for_lod);

        Self {
            config,
            voxel_appearances: VoxelPropertyMap::new(voxel_appearances),
            voxel_material_feature_ids: VoxelPropertyMap::new(voxel_material_feature_ids),
            voxel_tree_lod_controller,
            voxel_trees: HashMap::new(),
            voxel_tree_id_counter: 1,
        }
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

impl<F: Float> VoxelInstanceStorage<F> {
    fn new() -> Self {
        Self {
            voxel_types: Vec::new(),
            transforms: Vec::new(),
            index_map: KeyIndexMapper::default(),
        }
    }

    fn n_instances(&self) -> usize {
        self.index_map.len()
    }

    fn transforms(&self) -> &[VoxelTransform<F>] {
        &self.transforms
    }

    #[cfg(test)]
    fn transform(&self, instance_id: VoxelInstanceID) -> &VoxelTransform<F> {
        let idx = self.index_map.idx(instance_id.0);
        &self.transforms[idx]
    }

    fn add_instance(
        &mut self,
        instance_id: VoxelInstanceID,
        voxel_type: VoxelType,
        transform: VoxelTransform<F>,
    ) {
        self.index_map.push_key(instance_id.0);
        self.voxel_types.push(voxel_type);
        self.transforms.push(transform);
    }

    /// Converts each voxel transform in the storage into a model-view transform
    /// and adds it to the given transform buffer.
    pub fn buffer_all_transforms(
        &self,
        transform_buffer: &mut DynamicInstanceFeatureBuffer,
        view_transform: &Similarity3<F>,
        camera_space_axes_in_model_space: &CoordinateAxes<F>,
    ) where
        F: SubsetOf<fre>,
    {
        transform_buffer.add_features_from_iterator(self.transforms().iter().map(|transform| {
            transform.transform_into_model_view_transform(
                view_transform,
                camera_space_axes_in_model_space,
            )
        }));
    }

    /// Adds all voxel instance features in the storage to the given feature
    /// buffer.
    pub fn buffer_all_features(
        &self,
        feature_id_map: &VoxelPropertyMap<InstanceFeatureID>,
        feature_storage: &InstanceFeatureStorage,
        feature_buffer: &mut DynamicInstanceFeatureBuffer,
    ) {
        let feature_id = feature_id_map.value(VoxelType::Default);

        feature_buffer.add_feature_from_storage_repeatedly(
            feature_storage,
            *feature_id,
            self.n_instances(),
        );
    }
}

impl<F: Float> GroupedVoxelInstanceStorage<F> {
    fn new() -> Self {
        Self {
            groups: Vec::new(),
            index_map: KeyIndexMapper::default(),
            instance_id_count: 0,
            group_id_count: 0,
        }
    }

    #[cfg(test)]
    fn n_groups(&self) -> usize {
        self.index_map.len()
    }

    fn group(&self, group_id: InstanceGroupID) -> &VoxelInstanceStorage<F> {
        self.group_at_level_of_detail(group_id, 0)
    }

    fn group_at_level_of_detail(
        &self,
        group_id: InstanceGroupID,
        lod: u32,
    ) -> &VoxelInstanceStorage<F> {
        let idx = self.index_map.idx(group_id.0);
        &self.groups[idx][lod as usize]
    }

    fn group_at_level_of_detail_mut(
        &mut self,
        group_id: InstanceGroupID,
        lod: u32,
    ) -> &mut VoxelInstanceStorage<F> {
        let idx = self.index_map.idx(group_id.0);
        &mut self.groups[idx][lod as usize]
    }

    fn create_group(&mut self) -> InstanceGroupID {
        self.create_group_with_multiple_levels_of_detail(0)
    }

    fn create_group_with_multiple_levels_of_detail(&mut self, max_lod: u32) -> InstanceGroupID {
        let group_id = self.create_new_instance_group_id();
        self.index_map.push_key(group_id.0);
        self.groups
            .push(vec![VoxelInstanceStorage::new(); (max_lod + 1) as usize]);
        group_id
    }

    fn remove_group(&mut self, group_id: InstanceGroupID) {
        let idx = self.index_map.swap_remove_key(group_id.0);
        self.groups.swap_remove(idx);
    }

    fn create_new_voxel_instance_id(&mut self) -> VoxelInstanceID {
        let instance_id = VoxelInstanceID(self.instance_id_count);
        self.instance_id_count += 1;
        instance_id
    }

    fn create_new_instance_group_id(&mut self) -> InstanceGroupID {
        let group_id = InstanceGroupID(self.group_id_count);
        self.group_id_count += 1;
        group_id
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
        camera_space_axes_in_model_space: &CoordinateAxes<F>,
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
