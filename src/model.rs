//! Models defined by a mesh and material.

pub mod buffer;

use crate::{
    gpu::{
        rendering::fre,
        shader::{InstanceFeatureShaderInput, ModelViewTransformShaderInput},
    },
    impl_InstanceFeature,
    material::{MaterialHandle, MaterialLibrary},
    mesh::MeshID,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{self, AlignedByteVec, Alignment, Hash64, KeyIndexMapper};
use nalgebra::{Similarity3, UnitQuaternion, Vector3};
use nohash_hasher::BuildNoHashHasher;
use std::{
    cmp,
    collections::HashMap,
    fmt::{self},
    hash::{Hash, Hasher},
    mem,
    ops::Range,
};

/// Represents a piece of data associated with a model instance.
pub trait InstanceFeature: Pod {
    /// A unique ID representing the feature type.
    const FEATURE_TYPE_ID: InstanceFeatureTypeID;

    /// The size of the feature type in bytes.
    const FEATURE_SIZE: usize = mem::size_of::<Self>();

    /// The memory alignment of the feature type.
    const FEATURE_ALIGNMENT: Alignment = Alignment::of::<Self>();

    /// The layout of the vertex GPU buffer that can be used to pass the
    /// feature to the GPU.
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static>;

    /// The input required for a shader to access this feature.
    const SHADER_INPUT: InstanceFeatureShaderInput;

    /// Returns a slice with the raw bytes representing the feature.
    fn feature_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Returns a slice with the raw bytes representing the given slice of
    /// features.
    fn feature_slice_bytes(slice: &[Self]) -> &[u8] {
        bytemuck::cast_slice(slice)
    }
}

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material. If the material has an
/// associated prepass material, that will also be part of the model definition.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_handle: MaterialHandle,
    prepass_material_handle: Option<MaterialHandle>,
    hash: Hash64,
}

/// Container for features associated with instances of specific models.
///
/// Holds a set of [`InstanceFeatureStorage`]s, one storage for each feature
/// type. These storages are presistent and can be accessed to add, remove or
/// modify feature values for individual instances.
///
/// Additionally, a set of [`DynamicInstanceFeatureBuffer`]s is kept for each
/// model that has instances, one buffer for each feature type associated with
/// the model, with the first one always being a buffer for model-to-camera
/// transforms. These buffers are filled with transforms as well as feature
/// values from the `InstanceFeatureStorage`s for all instances that are to be
/// rendered. Their contents can then be copied directly to the corresponding
/// GPU buffers, before they are cleared in preparation for the next frame.
#[derive(Debug, Default)]
pub struct InstanceFeatureManager {
    feature_storages: HashMap<InstanceFeatureTypeID, InstanceFeatureStorage>,
    instance_feature_buffers: HashMap<ModelID, (usize, Vec<DynamicInstanceFeatureBuffer>)>,
}

/// Identifier for a type of instance feature.
pub type InstanceFeatureTypeID = Hash64;

/// Identifier for an instance feature value.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct InstanceFeatureID {
    feature_type_id: InstanceFeatureTypeID,
    idx: usize,
}

/// Container for instance feature values of the same type.
///
/// The storage is designed for efficient insertion of, access to and removal of
/// individual feature values.
///
/// Stores the raw bytes of the features to avoid exposing the feature type
/// signature. The type information is extracted on construction and used to
/// validate access requests.
#[derive(Debug)]
pub struct InstanceFeatureStorage {
    type_descriptor: InstanceFeatureTypeDescriptor,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    shader_input: InstanceFeatureShaderInput,
    bytes: AlignedByteVec,
    index_map: KeyIndexMapper<usize, BuildNoHashHasher<usize>>,
    feature_id_count: usize,
}

/// A buffer for instance feature values of the same type.
///
/// The buffer is grown on demand, but never shrunk. Instead, a counter keeps
/// track of the position of the last valid byte in the buffer, and the counter
/// is reset to zero when the buffer is cleared. This allows the it to be filled
/// and emptied repeatedly without unneccesary allocations.
///
/// Stores the raw bytes of the features to avoid exposing the feature type
/// signature. The type information is extracted on construction and used to
/// validate access requests.
#[derive(Debug)]
pub struct DynamicInstanceFeatureBuffer {
    type_descriptor: InstanceFeatureTypeDescriptor,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    shader_input: InstanceFeatureShaderInput,
    bytes: AlignedByteVec,
    n_valid_bytes: usize,
    range_manager: InstanceFeatureBufferRangeManager,
}

/// Identifier for a specific range of valid features in a
/// [`DynamicInstanceFeatureBuffer`].
pub type InstanceFeatureBufferRangeID = u32;

/// Helper for managing ranges in a [`DynamicInstanceFeatureBuffer`].
#[derive(Debug)]
pub struct InstanceFeatureBufferRangeManager {
    range_start_indices: Vec<u32>,
    range_id_index_mapper: KeyIndexMapper<InstanceFeatureBufferRangeID>,
}

/// Describes the ranges defined in a [`DynamicInstanceFeatureBuffer`].
#[derive(Clone, Debug)]
pub struct InstanceFeatureBufferRangeMap {
    range_start_indices: Vec<u32>,
    range_id_index_map: HashMap<InstanceFeatureBufferRangeID, usize>,
}

/// A model-to-camera transform for a specific instance of a model.
///
/// This struct is intended to be passed to the GPU in a vertex buffer. The
/// order of the fields is assumed in the shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InstanceModelViewTransform {
    pub rotation: UnitQuaternion<fre>,
    pub translation: Vector3<fre>,
    pub scaling: fre,
}

/// A model-to-light transform for a specific instance of a model.
pub type InstanceModelLightTransform = InstanceModelViewTransform;

#[derive(Copy, Clone, Debug)]
struct InstanceFeatureTypeDescriptor {
    id: InstanceFeatureTypeID,
    size: usize,
    alignment: Alignment,
}

const INSTANCE_VERTEX_BINDING_START: u32 = 0;

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the mesh and
    /// material with an optional prepass material.
    pub fn for_mesh_and_material(
        mesh_id: MeshID,
        material_handle: MaterialHandle,
        prepass_material_handle: Option<MaterialHandle>,
    ) -> Self {
        let mut hash = impact_utils::compute_hash_64_of_two_hash_64(
            mesh_id.0.hash(),
            material_handle.compute_hash(),
        );

        if let Some(prepass_material_handle) = prepass_material_handle {
            hash = impact_utils::compute_hash_64_of_two_hash_64(
                hash,
                prepass_material_handle.compute_hash(),
            );
        }

        Self {
            mesh_id,
            material_handle,
            prepass_material_handle,
            hash,
        }
    }

    /// The ID of the model's mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The handle for the model's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }

    /// The handle for the prepass material associated with the model's
    /// material.
    pub fn prepass_material_handle(&self) -> Option<&MaterialHandle> {
        self.prepass_material_handle.as_ref()
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}{}}}",
            self.mesh_id,
            &self.material_handle,
            if let Some(prepass_material_handle) = self.prepass_material_handle {
                format!(", prepass_material: {}", prepass_material_handle)
            } else {
                String::new()
            }
        )
    }
}

impl PartialEq for ModelID {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl Eq for ModelID {}

impl Ord for ModelID {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl PartialOrd for ModelID {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for ModelID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl InstanceFeatureManager {
    /// Creates a new empty instance feature manager.
    pub fn new() -> Self {
        Self {
            feature_storages: HashMap::new(),
            instance_feature_buffers: HashMap::new(),
        }
    }

    /// Whether the manager has instance feature buffers for the model with the
    /// given ID.
    pub fn has_model_id(&self, model_id: ModelID) -> bool {
        self.instance_feature_buffers.contains_key(&model_id)
    }

    /// Returns a reference to the set of instance feature buffers for the model
    /// with the given ID, or [`None`] if the model is not present.
    pub fn get_buffers(&self, model_id: ModelID) -> Option<&Vec<DynamicInstanceFeatureBuffer>> {
        self.instance_feature_buffers
            .get(&model_id)
            .map(|(_, buffers)| buffers)
    }

    /// Returns a mutable iterator over each buffer of model instance
    /// transforms.
    pub fn transform_buffers_mut(
        &mut self,
    ) -> impl Iterator<Item = &'_ mut DynamicInstanceFeatureBuffer> {
        self.instance_feature_buffers.values_mut().map(|buffers| {
            buffers
                .1
                .get_mut(0)
                .expect("Missing transform buffer for model")
        })
    }

    /// Returns a mutable reference to the buffer of model instance transforms
    /// for the model with the given ID.
    pub fn transform_buffer_mut(&mut self, model_id: ModelID) -> &mut DynamicInstanceFeatureBuffer {
        self.instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instances of missing model")
            .1
            .get_mut(0)
            .expect("Missing transform buffer for model")
    }

    /// Returns a mutable reference to the buffer of model instance transforms
    /// for the model with the given ID and another mutable reference to the
    /// first instance feature buffer following the transform buffer, along with
    /// a reference to the feature storage associated with the latter.
    ///
    /// # Panics
    /// If any of the requested buffers are not present.
    pub fn transform_and_next_feature_buffer_mut_with_storage(
        &mut self,
        model_id: ModelID,
    ) -> (
        &mut DynamicInstanceFeatureBuffer,
        (&InstanceFeatureStorage, &mut DynamicInstanceFeatureBuffer),
    ) {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instances of missing model")
            .1;

        let (transform_buffer, remaining_buffers) = feature_buffers
            .split_first_mut()
            .expect("Missing instance feature buffer for transforms");

        let (next_feature_buffer, _) = remaining_buffers
            .split_first_mut()
            .expect("Missing instance feature buffer following transform buffer");

        let feature_type_id = next_feature_buffer.feature_type_id();

        let storage = self.feature_storages.get(&feature_type_id).expect(
            "Missing storage associated with instance feature buffer following transform buffer",
        );

        (transform_buffer, (storage, next_feature_buffer))
    }

    /// Returns a reference to the storage of instance features of type `Fe`, or
    /// [`None`] if no storage exists for that type.
    pub fn get_storage<Fe: InstanceFeature>(&self) -> Option<&InstanceFeatureStorage> {
        self.get_storage_for_feature_type_id(Fe::FEATURE_TYPE_ID)
    }

    /// Returns a mutable reference to the storage of instance features
    /// of type `Fe`, or [`None`] if no storage exists for that type.
    pub fn get_storage_mut<Fe: InstanceFeature>(&mut self) -> Option<&mut InstanceFeatureStorage> {
        self.get_storage_mut_for_feature_type_id(Fe::FEATURE_TYPE_ID)
    }

    /// Returns a reference to the storage of instance features of the type with
    /// the given ID, or [`None`] if no storage exists for that type.
    pub fn get_storage_for_feature_type_id(
        &self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&InstanceFeatureStorage> {
        self.feature_storages.get(&feature_type_id)
    }

    /// Returns a mutable reference to the storage of instance features of the
    /// type with the given ID, or [`None`] if no storage exists for that type.
    pub fn get_storage_mut_for_feature_type_id(
        &mut self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&mut InstanceFeatureStorage> {
        self.feature_storages.get_mut(&feature_type_id)
    }

    /// Returns an iterator over the model IDs and their associated sets of
    /// instance feature buffers.
    pub fn model_ids_and_buffers(
        &self,
    ) -> impl Iterator<Item = (ModelID, &'_ Vec<DynamicInstanceFeatureBuffer>)> {
        self.instance_feature_buffers
            .iter()
            .map(|(model_id, (_, buffers))| (*model_id, buffers))
    }

    /// Returns an iterator over the model IDs and their associated sets of
    /// instance feature buffers, with the buffers being mutable.
    pub fn model_ids_and_mutable_buffers(
        &mut self,
    ) -> impl Iterator<Item = (ModelID, &'_ mut Vec<DynamicInstanceFeatureBuffer>)> {
        self.instance_feature_buffers
            .iter_mut()
            .map(|(model_id, (_, buffers))| (*model_id, buffers))
    }

    /// Sets up a storage for features of type `Fe`, which is required for
    /// supporting instances with features of that type.
    ///
    /// If a storage for the feature type is already set up, nothing happens.
    pub fn register_feature_type<Fe: InstanceFeature>(&mut self) {
        self.feature_storages
            .entry(Fe::FEATURE_TYPE_ID)
            .or_insert_with(|| InstanceFeatureStorage::new::<Fe>());
    }

    /// Registers the existance of a new instance of the model with the given
    /// ID. This involves initializing instance feature buffers for all the
    /// feature types associated with the model if they do not already exist.
    ///
    /// # Panics
    /// - If the model's material is not present in the material library.
    /// - If any of the model's feature types have not been registered with
    ///   [`register_feature_type`].
    pub fn register_instance(&mut self, material_library: &MaterialLibrary, model_id: ModelID)
    where
        InstanceModelViewTransform: InstanceFeature,
    {
        let material_feature_type_ids = material_library
            .get_material_specification(model_id.material_handle().material_id())
            .expect("Missing material specification for model material")
            .instance_feature_type_ids();

        if let Some(prepass_material_handle) = model_id.prepass_material_handle() {
            let prepass_material_feature_type_ids = material_library
                .get_material_specification(prepass_material_handle.material_id())
                .expect("Missing material specification for model prepass material")
                .instance_feature_type_ids();

            if !prepass_material_feature_type_ids.is_empty() {
                assert_eq!(
                    prepass_material_feature_type_ids, material_feature_type_ids,
                    "Prepass material must use the same feature types as main material"
                );
            }
        }

        self.register_instance_with_feature_type_ids(model_id, material_feature_type_ids);
    }

    /// Informs the manager that an instance of the model with the given ID has
    /// been deleted, so that the associated instance feature buffers can be
    /// deleted if this was the last instance of that model.
    ///
    /// # Panics
    /// If no instance of the specified model exists.
    pub fn unregister_instance(&mut self, model_id: ModelID) {
        let count = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to unregister instance of model that has no instances")
            .0;

        assert!(*count > 0);

        *count -= 1;

        if *count == 0 {
            self.instance_feature_buffers.remove(&model_id);
        }
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the given transform and the feature values corrsponding to the
    /// given feature IDs onto the buffers.
    ///
    /// # Panics
    /// - If no buffers exist for the model with the given ID.
    /// - If the number of feature IDs is not exactly one less than the number
    ///   of buffers (the first buffer is used for the transform).
    /// - If any of the feature IDs are for feature types other than the type
    ///   stored in the corresponding buffer (the order of the feature IDs has
    ///   to be the same as in the
    ///   [`MaterialSpecification`](crate::material::MaterialSpecification) of
    ///   the model, which was used to initialize the buffers.
    pub fn buffer_instance(
        &mut self,
        model_id: ModelID,
        transform: &InstanceModelViewTransform,
        feature_ids: &[InstanceFeatureID],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        assert_eq!(feature_ids.len() + 1, feature_buffers.len());

        let mut feature_buffers = feature_buffers.iter_mut();

        feature_buffers
            .next()
            .expect("Missing transform buffer for instance")
            .add_feature(transform);

        for (&feature_id, feature_buffer) in feature_ids.iter().zip(feature_buffers) {
            let feature_type_id = feature_buffer.feature_type_id();

            let storage = self
                .feature_storages
                .get(&feature_type_id)
                .expect("Missing storage for model instance feature");

            feature_buffer.add_feature_from_storage(storage, feature_id);
        }
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the given set of transforms (one for each instance) and the
    /// feature values corrsponding to the given sets of feature IDs (one
    /// [`Vec`] for each feature type, each [`Vec`] containing either a single
    /// feature ID, in which case this is assumed to apply to all instances, or
    /// containing a separate feature ID for each instance) onto the buffers.
    ///
    /// # Panics
    /// - If no buffers exist for the model with the given ID.
    /// - If the number of feature IDs is not exactly one less than the number
    ///   of buffers (the first buffer is used for the transform).
    /// - If any of the feature IDs are for feature types other than the type
    ///   stored in the corresponding buffer (the order of the feature IDs has
    ///   to be the same as in the
    ///   [`MaterialSpecification`](crate::material::MaterialSpecification) of
    ///   the
    ///   model, which was used to initialize the buffers.
    /// - If any of the [`Vec`] with feature IDs has a different length than one
    ///   or the number of transforms.
    pub fn buffer_multiple_instances(
        &mut self,
        model_id: ModelID,
        transforms: impl ExactSizeIterator<Item = InstanceModelViewTransform>,
        feature_ids: &[Vec<InstanceFeatureID>],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instances of missing model")
            .1;

        assert_eq!(feature_ids.len() + 1, feature_buffers.len());

        let mut feature_buffers = feature_buffers.iter_mut();

        let transform_buffer = feature_buffers
            .next()
            .expect("Missing transform buffer for instance");

        let n_instances = transforms.len();
        transform_buffer.add_features_from_iterator(transforms);

        for (feature_ids_for_feature_type, feature_buffer) in
            feature_ids.iter().zip(feature_buffers)
        {
            let feature_type_id = feature_buffer.feature_type_id();

            let storage = self
                .feature_storages
                .get(&feature_type_id)
                .expect("Missing storage for model instance feature");

            let n_features_for_feature_type = feature_ids_for_feature_type.len();
            if n_features_for_feature_type == 1 && n_instances > 1 {
                feature_buffer.add_feature_from_storage_repeatedly(
                    storage,
                    feature_ids_for_feature_type[0],
                    n_instances,
                );
            } else {
                assert_eq!(
                    n_features_for_feature_type, n_instances,
                    "Encountered different instance counts for different feature types when buffering multiple instances"
                );
                for &feature_id in feature_ids_for_feature_type {
                    feature_buffer.add_feature_from_storage(storage, feature_id);
                }
            }
        }
    }

    /// Finds the instance transform buffer for the model with the given ID and
    /// pushes the given transfrom onto it.
    ///
    /// # Panics
    /// If no buffers exist for the model with the given ID.
    pub fn buffer_instance_transform(
        &mut self,
        model_id: ModelID,
        transform: &InstanceModelViewTransform,
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        feature_buffers
            .get_mut(0)
            .expect("Missing transform buffer for instance")
            .add_feature(transform);
    }

    /// Finds the instance transform buffer for the model with the given ID and
    /// pushes the given transforms onto it.
    ///
    /// # Panics
    /// If no buffers exist for the model with the given ID.
    pub fn buffer_multiple_instance_transforms(
        &mut self,
        model_id: ModelID,
        transforms: impl ExactSizeIterator<Item = InstanceModelViewTransform>,
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        feature_buffers
            .get_mut(0)
            .expect("Missing transform buffer for instance")
            .add_features_from_iterator(transforms);
    }

    /// Clears all instance feature buffers and removes all features from the
    /// storages.
    pub fn clear_storages_and_buffers(&mut self) {
        self.instance_feature_buffers.clear();
        for storage in self.feature_storages.values_mut() {
            storage.remove_all_features();
        }
    }

    fn register_instance_with_feature_type_ids(
        &mut self,
        model_id: ModelID,
        feature_type_ids: &[InstanceFeatureTypeID],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        self.instance_feature_buffers
            .entry(model_id)
            .and_modify(|(count, buffers)| {
                assert_eq!(buffers.len(), feature_type_ids.len() + 1);
                *count += 1;
            })
            .or_insert_with(|| {
                let mut buffers = Vec::with_capacity(feature_type_ids.len() + 1);
                buffers.push(DynamicInstanceFeatureBuffer::new::<
                    InstanceModelViewTransform,
                >());
                buffers.extend(feature_type_ids.iter().map(|feature_type_id| {
                    let storage = self.feature_storages.get(feature_type_id).expect(
                        "Missing storage for instance feature type \
                             (all feature types must be registered with `register_feature_type`)",
                    );
                    DynamicInstanceFeatureBuffer::new_for_storage(storage)
                }));
                (1, buffers)
            });
    }
}

impl InstanceFeatureID {
    /// Creates an ID that does not represent a valid feature.
    pub fn not_applicable() -> Self {
        Self {
            feature_type_id: Hash64::zeroed(),
            idx: usize::MAX,
        }
    }

    /// Returns the ID of the type of feature this ID identifies.
    pub fn feature_type_id(&self) -> InstanceFeatureTypeID {
        self.feature_type_id
    }

    /// Returns `true` if this ID does not represent a valid feature.
    pub fn is_not_applicable(&self) -> bool {
        self.feature_type_id == Hash64::zeroed() && self.idx == usize::MAX
    }
}

impl InstanceFeatureStorage {
    /// Creates a new empty storage for features of type `Fe`.
    pub fn new<Fe: InstanceFeature>() -> Self {
        Self {
            type_descriptor: InstanceFeatureTypeDescriptor::for_type::<Fe>(),
            vertex_buffer_layout: Fe::BUFFER_LAYOUT,
            shader_input: Fe::SHADER_INPUT,
            bytes: AlignedByteVec::new(Fe::FEATURE_ALIGNMENT),
            index_map: KeyIndexMapper::default(),
            feature_id_count: 0,
        }
    }

    /// Creates a new empty storage with preallocated capacity for the given
    /// number of features of type `Fe`.
    pub fn with_capacity<Fe: InstanceFeature>(feature_count: usize) -> Self {
        Self {
            type_descriptor: InstanceFeatureTypeDescriptor::for_type::<Fe>(),
            vertex_buffer_layout: Fe::BUFFER_LAYOUT,
            shader_input: Fe::SHADER_INPUT,
            bytes: AlignedByteVec::with_capacity(
                Fe::FEATURE_ALIGNMENT,
                feature_count * Fe::FEATURE_SIZE,
            ),
            index_map: KeyIndexMapper::default(),
            feature_id_count: 0,
        }
    }

    /// Returns the ID of the type of feature this storage can store.
    pub fn feature_type_id(&self) -> InstanceFeatureTypeID {
        self.type_descriptor.type_id()
    }

    /// Returns the size in bytes of the type of feature this storage can store.
    pub fn feature_size(&self) -> usize {
        self.type_descriptor.size()
    }

    /// Returns the layout of the vertex GPU buffer that can be used for the
    /// stored features.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the input required for accessing the features in a shader.
    pub fn shader_input(&self) -> &InstanceFeatureShaderInput {
        &self.shader_input
    }

    /// Returns the number of stored features.
    pub fn feature_count(&self) -> usize {
        self.index_map.len()
    }

    /// Whether a feature with the given identifier exists in the storage.
    ///
    /// # Panics
    /// If the given feature ID was issued from a storage for a different
    /// feature type.
    pub fn has_feature(&self, feature_id: InstanceFeatureID) -> bool {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        self.index_map.contains_key(feature_id.idx)
    }

    /// Returns a reference to the value of the feature stored under the given
    /// identifier.
    ///
    /// # Panics
    /// - If the given feature ID was issued from a storage for a different
    ///   feature type.
    /// - If `Fe` is not the feature type the storage was initialized with.
    /// - If `Fe` is a zero-sized type.
    pub fn feature<Fe: InstanceFeature>(&self, feature_id: InstanceFeatureID) -> &Fe {
        self.type_descriptor.validate_feature::<Fe>();
        assert_ne!(
            Fe::FEATURE_SIZE,
            0,
            "Tried to obtain zero-sized feature from storage"
        );
        &bytemuck::cast_slice(self.feature_bytes(feature_id))[0]
    }

    /// Returns a mutable reference to the value of the feature stored under the
    /// given identifier.
    ///
    /// # Panics
    /// - If the given feature ID was issued from a storage for a different
    ///   feature type.
    /// - If `Fe` is not the feature type the storage was initialized with.
    /// - If `Fe` is a zero-sized type.
    pub fn feature_mut<Fe: InstanceFeature>(&mut self, feature_id: InstanceFeatureID) -> &mut Fe {
        self.type_descriptor.validate_feature::<Fe>();
        assert_ne!(
            Fe::FEATURE_SIZE,
            0,
            "Tried to obtain zero-sized feature mutably from storage"
        );
        &mut bytemuck::cast_slice_mut(self.feature_bytes_mut(feature_id))[0]
    }

    /// Appends the given feature value to the end of the storage.
    ///
    /// # Returns
    /// An identifier that can be used to access the feature.
    ///
    /// # Panics
    /// If `Fe` is not the feature type the storage was initialized with.
    pub fn add_feature<Fe: InstanceFeature>(&mut self, feature: &Fe) -> InstanceFeatureID {
        self.type_descriptor.validate_feature::<Fe>();
        self.bytes.extend_from_slice(bytemuck::bytes_of(feature));
        let feature_id = self.create_new_feature_id();
        self.index_map.push_key(feature_id.idx);
        feature_id
    }

    /// Removes the feature with the given identifier.
    ///
    /// # Panics
    /// - If the given feature ID was issued from a storage for a different
    ///   feature type.
    /// - If no feature with the given identifier exists in the storage.
    pub fn remove_feature(&mut self, feature_id: InstanceFeatureID) {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);

        let feature_size = self.feature_size();
        let feature_idx = self.index_map.swap_remove_key(feature_id.idx);

        if feature_size > 0 {
            let feature_to_remove_start = feature_idx.checked_mul(feature_size).unwrap();
            let buffer_size = self.bytes.len();

            // Copy over with last feature unless the feature to remove is the
            // last one
            let last_feature_start = buffer_size - feature_size;
            if feature_to_remove_start < last_feature_start {
                unsafe {
                    // Pointer to beginning of last feature
                    let src_ptr = self.bytes.as_ptr().add(last_feature_start);

                    // Mutable pointer to beginning of feature to remove
                    let dst_ptr = self.bytes.as_mut_ptr().add(feature_to_remove_start);

                    // Copy last feature over feature to remove
                    std::ptr::copy_nonoverlapping::<u8>(src_ptr, dst_ptr, feature_size);
                }
            }

            // Remove last feature (this must be done on the raw byte `Vec`)
            self.bytes.truncate(last_feature_start);
        }
    }

    /// Removes all the features from the storage.
    pub fn remove_all_features(&mut self) {
        self.bytes.truncate(0);
        self.index_map.clear();
    }

    fn type_descriptor(&self) -> InstanceFeatureTypeDescriptor {
        self.type_descriptor
    }

    fn feature_bytes(&self, feature_id: InstanceFeatureID) -> &[u8] {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        let feature_idx = self.index_map.idx(feature_id.idx);
        let byte_range = self.feature_byte_range(feature_idx);
        &self.bytes[byte_range]
    }

    fn feature_bytes_mut(&mut self, feature_id: InstanceFeatureID) -> &mut [u8] {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        let feature_idx = self.index_map.idx(feature_id.idx);
        let byte_range = self.feature_byte_range(feature_idx);
        &mut self.bytes[byte_range]
    }

    fn feature_byte_range(&self, feature_idx: usize) -> Range<usize> {
        let byte_idx = feature_idx.checked_mul(self.feature_size()).unwrap();
        byte_idx..(byte_idx + self.feature_size())
    }

    fn create_new_feature_id(&mut self) -> InstanceFeatureID {
        let feature_id = InstanceFeatureID {
            feature_type_id: self.feature_type_id(),
            idx: self.feature_id_count,
        };
        self.feature_id_count += 1;
        feature_id
    }
}

impl DynamicInstanceFeatureBuffer {
    /// The number of features that space should be allocated for when a new
    /// buffer is constructed.
    ///
    /// By having some initial space we avoid the issue of potentially
    /// constructing empty GPU buffers when synchronizing this buffer with
    /// the GPU.
    const INITIAL_ALLOCATED_FEATURE_COUNT: usize = 1;

    /// Creates a new empty buffer for features of type `Fe`.
    pub fn new<Fe: InstanceFeature>() -> Self {
        Self {
            type_descriptor: InstanceFeatureTypeDescriptor::for_type::<Fe>(),
            vertex_buffer_layout: Fe::BUFFER_LAYOUT,
            shader_input: Fe::SHADER_INPUT,
            bytes: AlignedByteVec::copied_from_slice(
                Fe::FEATURE_ALIGNMENT,
                &vec![0; Fe::FEATURE_SIZE * Self::INITIAL_ALLOCATED_FEATURE_COUNT],
            ),
            n_valid_bytes: 0,
            range_manager: InstanceFeatureBufferRangeManager::new_with_initial_range(),
        }
    }

    /// Creates a new empty buffer for the same type of features as stored in
    /// the given storage.
    pub fn new_for_storage(storage: &InstanceFeatureStorage) -> Self {
        let type_descriptor = storage.type_descriptor();
        Self {
            type_descriptor,
            vertex_buffer_layout: storage.vertex_buffer_layout().clone(),
            shader_input: storage.shader_input().clone(),
            bytes: AlignedByteVec::copied_from_slice(
                type_descriptor.alignment(),
                &vec![0; type_descriptor.size() * Self::INITIAL_ALLOCATED_FEATURE_COUNT],
            ),
            n_valid_bytes: 0,
            range_manager: InstanceFeatureBufferRangeManager::new_with_initial_range(),
        }
    }

    /// Returns the ID of the type of feature this buffer can store.
    pub fn feature_type_id(&self) -> InstanceFeatureTypeID {
        self.type_descriptor.type_id()
    }

    /// Returns the size in bytes of the type of feature this buffer can store.
    pub fn feature_size(&self) -> usize {
        self.type_descriptor.size()
    }

    /// Returns the layout of the vertex GPU buffer that can be used for the
    /// stored features.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the input required for accessing the features in a shader.
    pub fn shader_input(&self) -> &InstanceFeatureShaderInput {
        &self.shader_input
    }

    /// Returns the current number of valid features in the buffer.
    ///
    /// # Panics
    /// If the feature type the buffer stores is a zero-sized type.
    pub fn n_valid_features(&self) -> usize {
        assert_ne!(self.feature_size(), 0);
        self.n_valid_bytes() / self.feature_size()
    }

    /// Returns the range of valid feature indices with the given ID. Ranges are
    /// defined by calling [`begin_range`]. The range spans from and including
    /// the first feature added after the `begin_range` call to and including
    /// the last feature added before the next `begin_range` call, or to the
    /// last valid feature if the `begin_range` call was the last one. Calling
    /// [`clear`] removes all range information.
    ///
    /// # Panics
    /// If no range with the given ID exists.
    pub fn valid_feature_range(&self, range_id: InstanceFeatureBufferRangeID) -> Range<u32> {
        self.range_manager
            .get_range(range_id, || self.n_valid_features())
    }

    /// Returns the range of valid feature indices encompassing all features
    /// added before defining any explicit ranges with [`begin_range`].
    pub fn initial_valid_feature_range(&self) -> Range<u32> {
        self.valid_feature_range(InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID)
    }

    /// Creates an [`InstanceFeatureBufferRangeMap`] containing the information
    /// describing the ranges that have been defined with [`begin_range`].
    pub fn create_range_map(&self) -> InstanceFeatureBufferRangeMap {
        InstanceFeatureBufferRangeMap::from_manager(&self.range_manager)
    }

    /// Returns the number of bytes from the beginning of the buffer that are
    /// currently valid.
    pub fn n_valid_bytes(&self) -> usize {
        self.n_valid_bytes
    }

    /// Returns a slice with the currently valid features in the buffer.
    ///
    /// # Panics
    /// - If `Fe` is not the feature type the buffer was initialized with.
    /// - If `Fe` is a zero-sized type.
    pub fn valid_features<Fe: InstanceFeature>(&self) -> &[Fe] {
        self.type_descriptor.validate_feature::<Fe>();
        assert_ne!(self.feature_size(), 0);
        let valid_bytes = self.valid_bytes();

        // Make sure not to call `cast_slice` on an empty slice, as an empty
        // slice is not guaranteed to have the correct alignment
        if valid_bytes.is_empty() {
            &[]
        } else {
            bytemuck::cast_slice(valid_bytes)
        }
    }

    /// Returns a slice with the currently valid bytes in the buffer.
    pub fn valid_bytes(&self) -> &[u8] {
        &self.bytes[..self.n_valid_bytes()]
    }

    /// Returns a slice with all the bytes in the buffer, including currently
    /// invalid ones.
    ///
    /// # Warning
    /// Only the bytes below [`n_valid_bytes`](Self::n_valid_bytes) are
    /// considered to have valid values.
    pub fn raw_buffer(&self) -> &[u8] {
        &self.bytes
    }

    /// Pushes a copy of the given feature value onto the buffer.
    ///
    /// # Panics
    /// If `Fe` is not the feature type the buffer was initialized with.
    pub fn add_feature<Fe: InstanceFeature>(&mut self, feature: &Fe) {
        self.type_descriptor.validate_feature::<Fe>();
        self.add_feature_bytes(feature.feature_bytes());
    }

    /// Pushes a copy of the given slice of feature values onto the buffer.
    ///
    /// # Panics
    /// If `Fe` is not the feature type the buffer was initialized with.
    pub fn add_feature_slice<Fe: InstanceFeature>(&mut self, features: &[Fe]) {
        self.type_descriptor.validate_feature::<Fe>();
        self.add_feature_slice_bytes(features.len(), Fe::feature_slice_bytes(features));
    }

    /// Pushes each feature from the given iterator into the buffer.
    ///
    /// # Panics
    /// If `Fe` is not the feature type the buffer was initialized with.
    pub fn add_features_from_iterator<Fe: InstanceFeature>(
        &mut self,
        features: impl ExactSizeIterator<Item = Fe>,
    ) {
        self.type_descriptor.validate_feature::<Fe>();

        let feature_size = self.feature_size();
        let n_features = features.len();

        if feature_size > 0 && n_features > 0 {
            let start_byte_idx = self.n_valid_bytes;
            self.n_valid_bytes += feature_size.checked_mul(n_features).unwrap();
            let end_byte_idx = self.n_valid_bytes;

            // If the buffer is full, grow it first
            if end_byte_idx >= self.bytes.len() {
                self.grow_buffer(end_byte_idx);
            }

            self.bytes[start_byte_idx..end_byte_idx]
                .chunks_exact_mut(feature_size)
                .zip(features)
                .for_each(|(dest, feature)| {
                    dest.copy_from_slice(feature.feature_bytes());
                });
        }
    }

    /// Pushes a copy of the feature value stored in the given storage under the
    /// given identifier onto the buffer.
    ///
    /// # Panics
    /// - If the feature types of the storage and buffer are not the same.
    /// - If no feature with the given identifier exists in the storage.
    pub fn add_feature_from_storage(
        &mut self,
        storage: &InstanceFeatureStorage,
        feature_id: InstanceFeatureID,
    ) {
        self.type_descriptor
            .validate_feature_type_id(storage.feature_type_id());
        self.add_feature_bytes(storage.feature_bytes(feature_id));
    }

    /// Pushes the given number of copies of the feature value stored in the
    /// given storage under the given identifier onto the buffer.
    ///
    /// # Panics
    /// - If the feature types of the storage and buffer are not the same.
    /// - If no feature with the given identifier exists in the storage.
    pub fn add_feature_from_storage_repeatedly(
        &mut self,
        storage: &InstanceFeatureStorage,
        feature_id: InstanceFeatureID,
        n_copies: usize,
    ) {
        self.type_descriptor
            .validate_feature_type_id(storage.feature_type_id());
        self.add_feature_bytes_repeatedly(storage.feature_bytes(feature_id), n_copies);
    }

    /// Begins a new range in the buffer starting at the location just after the
    /// current last feature (or at the beginning if the buffer is empty). The
    /// range is assigned the given ID. All features added between this and the
    /// next [`begin_range`] call will be considered part of this new range.
    ///
    /// # Panics
    /// If a range with the given ID already exists.
    pub fn begin_range(&mut self, range_id: InstanceFeatureBufferRangeID) {
        self.range_manager
            .begin_range(self.n_valid_features(), range_id);
    }

    /// Empties the buffer and forgets any range information.
    ///
    /// Does not actually drop buffer contents, just resets the count of valid
    /// bytes to zero.
    pub fn clear(&mut self) {
        self.n_valid_bytes = 0;
        self.range_manager.clear();
    }

    fn add_feature_bytes(&mut self, feature_bytes: &[u8]) {
        let feature_size = self.feature_size();
        assert_eq!(feature_bytes.len(), feature_size);

        if feature_size > 0 {
            let start_byte_idx = self.n_valid_bytes;
            self.n_valid_bytes += feature_size;
            let end_byte_idx = self.n_valid_bytes;

            // If the buffer is full, grow it first
            if end_byte_idx >= self.bytes.len() {
                self.grow_buffer(end_byte_idx);
            }

            self.bytes[start_byte_idx..end_byte_idx].copy_from_slice(feature_bytes);
        }
    }

    fn add_feature_bytes_repeatedly(&mut self, feature_bytes: &[u8], n_copies: usize) {
        let feature_size = self.feature_size();
        assert_eq!(feature_bytes.len(), feature_size);

        if feature_size > 0 && n_copies > 0 {
            let start_byte_idx = self.n_valid_bytes;
            self.n_valid_bytes += feature_size.checked_mul(n_copies).unwrap();
            let end_byte_idx = self.n_valid_bytes;

            // If the buffer is full, grow it first
            if end_byte_idx >= self.bytes.len() {
                self.grow_buffer(end_byte_idx);
            }

            self.bytes[start_byte_idx..end_byte_idx]
                .chunks_exact_mut(feature_size)
                .for_each(|dest| {
                    dest.copy_from_slice(feature_bytes);
                });
        }
    }

    fn add_feature_slice_bytes(&mut self, feature_count: usize, feature_bytes: &[u8]) {
        let feature_slice_size = self.feature_size() * feature_count;
        assert_eq!(feature_bytes.len(), feature_slice_size);

        if feature_slice_size > 0 {
            let start_byte_idx = self.n_valid_bytes;
            self.n_valid_bytes += feature_slice_size;
            let end_byte_idx = self.n_valid_bytes;

            // If the buffer is full, grow it first
            if end_byte_idx >= self.bytes.len() {
                self.grow_buffer(end_byte_idx);
            }

            self.bytes[start_byte_idx..end_byte_idx].copy_from_slice(feature_bytes);
        }
    }

    fn grow_buffer(&mut self, min_size: usize) {
        let old_buffer_size = self.bytes.len();

        // Add one before doubling to avoid getting stuck at zero
        let mut new_buffer_size = (old_buffer_size + 1).checked_mul(2).unwrap();

        while new_buffer_size < min_size {
            new_buffer_size = new_buffer_size.checked_mul(2).unwrap();
        }

        self.bytes.resize(new_buffer_size, 0);
    }
}

impl InstanceFeatureBufferRangeManager {
    /// ID of the initial range created when calling [`new_with_initial_range`].
    pub const INITIAL_RANGE_ID: InstanceFeatureBufferRangeID = InstanceFeatureBufferRangeID::MAX;

    /// Creates a new [`BufferRangeManager`] with a single range starting at
    /// index 0. The ID of the initial range is available in the
    /// [`Self::INITIAL_RANGE_ID`] constant.
    pub fn new_with_initial_range() -> Self {
        Self {
            range_start_indices: vec![0],
            range_id_index_mapper: KeyIndexMapper::new_with_key(Self::INITIAL_RANGE_ID),
        }
    }

    /// Returns the range with the given [`InstanceFeatureBufferRangeID`]. If
    /// the range is the last one, the given buffer length will be used as the
    /// upper limit of the range.
    ///
    /// # Panics
    /// - If no range with the given ID exists.
    /// - If the range is the last one and the given buffer length is smaller
    ///   than the range start index.
    pub fn get_range(
        &self,
        range_id: InstanceFeatureBufferRangeID,
        get_buffer_length: impl Fn() -> usize,
    ) -> Range<u32> {
        let range_idx = self
            .range_id_index_mapper
            .get(range_id)
            .expect("Requested range with invalid ID in buffer range manager");

        let next_range_idx = range_idx + 1;

        let range_start_idx = self.range_start_indices[range_idx];

        let range_end_idx = if next_range_idx < self.range_id_index_mapper.len() {
            self.range_start_indices[next_range_idx]
        } else {
            let buffer_length = u32::try_from(get_buffer_length()).unwrap();
            assert!(
                buffer_length >= range_start_idx,
                "Provided buffer length is smaller than start index of last range in buffer range manager"
            );
            buffer_length
        };

        range_start_idx..range_end_idx
    }

    /// Begins a new range in the buffer starting at the given index, and
    /// assigns the range the given ID.
    ///
    /// # Panics
    /// - If the given start index is smaller than the start index of the
    ///   previous range.
    /// - If a range with the given ID already exists.
    pub fn begin_range(&mut self, range_start_idx: usize, range_id: InstanceFeatureBufferRangeID) {
        let range_idx = self.range_id_index_mapper.len();

        let range_start_idx = u32::try_from(range_start_idx).unwrap();

        assert!(
            range_start_idx >= self.range_start_indices[range_idx - 1],
            "Tried to create range starting before the previous range in buffer range manager"
        );

        self.range_id_index_mapper.push_key(range_id);

        if range_idx == self.range_start_indices.len() {
            self.range_start_indices.push(range_start_idx);
        } else {
            self.range_start_indices[range_idx] = range_start_idx;
        }
    }

    /// Forgets all ranges.
    pub fn clear(&mut self) {
        self.range_id_index_mapper.clear();
        self.range_id_index_mapper.push_key(Self::INITIAL_RANGE_ID);
    }
}

impl InstanceFeatureBufferRangeMap {
    /// ID of the initial range.
    pub const INITIAL_RANGE_ID: InstanceFeatureBufferRangeID =
        InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID;

    fn from_manager(manager: &InstanceFeatureBufferRangeManager) -> Self {
        let range_start_indices =
            manager.range_start_indices[..manager.range_id_index_mapper.len()].to_vec();

        let range_id_index_map = manager.range_id_index_mapper.as_map().clone();

        Self {
            range_start_indices,
            range_id_index_map,
        }
    }

    /// Returns the range with the given [`InstanceFeatureBufferRangeID`]. If
    /// the range is the last one, the given buffer length will be used as the
    /// upper limit of the range.
    ///
    /// # Panics
    /// - If no range with the given ID exists.
    /// - If the range is the last one and the given buffer length is smaller
    ///   than the range start index.
    pub fn get_range(
        &self,
        range_id: InstanceFeatureBufferRangeID,
        buffer_length: u32,
    ) -> Range<u32> {
        let range_idx = *self
            .range_id_index_map
            .get(&range_id)
            .expect("Requested range with invalid ID in buffer range map");

        let next_range_idx = range_idx + 1;

        let range_start_idx = self.range_start_indices[range_idx];

        let range_end_idx = if next_range_idx < self.range_start_indices.len() {
            self.range_start_indices[next_range_idx]
        } else {
            assert!(
                buffer_length >= range_start_idx,
                "Provided buffer length is smaller than start index of last range in buffer range map"
            );
            buffer_length
        };

        range_start_idx..range_end_idx
    }
}

impl InstanceFeatureTypeDescriptor {
    fn for_type<Fe: InstanceFeature>() -> Self {
        Self::new(Fe::FEATURE_TYPE_ID, Fe::FEATURE_SIZE, Fe::FEATURE_ALIGNMENT)
    }

    fn new(id: InstanceFeatureTypeID, size: usize, alignment: Alignment) -> Self {
        Self {
            id,
            size,
            alignment,
        }
    }

    fn type_id(&self) -> InstanceFeatureTypeID {
        self.id
    }

    fn size(&self) -> usize {
        self.size
    }

    fn alignment(&self) -> Alignment {
        self.alignment
    }

    fn validate_feature<Fe: InstanceFeature>(&self) {
        self.validate_feature_type_id(Fe::FEATURE_TYPE_ID);
    }

    fn validate_feature_type_id(&self, feature_type_id: InstanceFeatureTypeID) {
        assert!(
            feature_type_id == self.id,
            "Mismatched instance feature types"
        );
    }
}

impl InstanceModelViewTransform {
    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Self {
            rotation: UnitQuaternion::identity(),
            translation: Vector3::zeros(),
            scaling: 1.0,
        }
    }

    /// Creates a new model-to-camera transform corresponding to the given
    /// similarity transform.
    pub fn with_model_view_transform(transform: Similarity3<fre>) -> Self {
        let scaling = transform.scaling();

        Self {
            rotation: transform.isometry.rotation,
            translation: transform.isometry.translation.vector,
            scaling,
        }
    }
}

impl InstanceModelLightTransform {
    /// Creates a new model-to-light transform corresponding to the given
    /// similarity transform.
    pub fn with_model_light_transform(transform: Similarity3<fre>) -> Self {
        Self::with_model_view_transform(transform)
    }
}

impl Default for InstanceModelViewTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl_InstanceFeature!(
    InstanceModelViewTransform,
    wgpu::vertex_attr_array![
        INSTANCE_VERTEX_BINDING_START => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 1 => Float32x4,
    ],
    InstanceFeatureShaderInput::ModelViewTransform(ModelViewTransformShaderInput {
        rotation_location: INSTANCE_VERTEX_BINDING_START,
        translation_and_scaling_location: INSTANCE_VERTEX_BINDING_START + 1,
    })
);

/// Convenience macro for implementing the [`InstanceFeature`] trait. The
/// feature type ID is created by hashing the name of the implementing type.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_InstanceFeature {
    ($ty:ty, $vertex_attr_array:expr, $shader_input:expr) => {
        impl $crate::model::InstanceFeature for $ty {
            const FEATURE_TYPE_ID: $crate::model::InstanceFeatureTypeID =
                impact_utils::ConstStringHash64::new(stringify!($ty)).into_hash();

            const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
                $crate::mesh::buffer::create_vertex_buffer_layout_for_instance::<Self>(
                    &$vertex_attr_array,
                );

            const SHADER_INPUT: $crate::gpu::shader::InstanceFeatureShaderInput = $shader_input;
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

    mod manager {
        use super::*;
        use crate::{
            gpu::shader::InstanceFeatureShaderInput,
            impl_InstanceFeature,
            material::{MaterialHandle, MaterialID},
            mesh::MeshID,
        };
        use bytemuck::{Pod, Zeroable};
        use impact_utils::hash64;
        use nalgebra::{Similarity3, Translation3, UnitQuaternion};

        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
        struct Feature(u8);

        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
        struct DifferentFeature(f64);

        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
        struct ZeroSizedFeature;

        impl_InstanceFeature!(Feature, [], InstanceFeatureShaderInput::None);
        impl_InstanceFeature!(DifferentFeature, [], InstanceFeatureShaderInput::None);
        impl_InstanceFeature!(ZeroSizedFeature, [], InstanceFeatureShaderInput::None);

        fn create_dummy_model_id<S: AsRef<str>>(tag: S) -> ModelID {
            ModelID::for_mesh_and_material(
                MeshID(hash64!(format!("Test mesh {}", tag.as_ref()))),
                MaterialHandle::new(
                    MaterialID(hash64!(format!("Test material {}", tag.as_ref()))),
                    None,
                    None,
                ),
                None,
            )
        }

        fn create_dummy_transform() -> InstanceModelViewTransform {
            InstanceModelViewTransform::with_model_view_transform(Similarity3::from_parts(
                Translation3::new(2.1, -5.9, 0.01),
                UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
                7.0,
            ))
        }

        fn create_dummy_transform_2() -> InstanceModelViewTransform {
            InstanceModelViewTransform::with_model_view_transform(Similarity3::from_parts(
                Translation3::new(6.1, -2.7, -0.21),
                UnitQuaternion::from_euler_angles(1.1, 3.2, 2.3),
                3.0,
            ))
        }

        #[test]
        fn creating_instance_feature_manager_works() {
            let manager = InstanceFeatureManager::new();
            assert!(manager.model_ids_and_buffers().next().is_none());
        }

        #[test]
        fn registering_feature_types_for_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();

            assert!(manager.get_storage::<ZeroSizedFeature>().is_none());
            assert!(manager.get_storage_mut::<ZeroSizedFeature>().is_none());

            manager.register_feature_type::<ZeroSizedFeature>();

            let storage_1 = manager.get_storage::<ZeroSizedFeature>().unwrap();
            assert_eq!(
                storage_1.feature_type_id(),
                ZeroSizedFeature::FEATURE_TYPE_ID
            );
            assert_eq!(storage_1.feature_count(), 0);

            assert!(manager.get_storage::<Feature>().is_none());
            assert!(manager.get_storage_mut::<Feature>().is_none());

            manager.register_feature_type::<Feature>();

            let storage_2 = manager.get_storage::<Feature>().unwrap();
            assert_eq!(storage_2.feature_type_id(), Feature::FEATURE_TYPE_ID);
            assert_eq!(storage_2.feature_count(), 0);
        }

        #[test]
        fn registering_one_instance_of_one_model_with_no_features_in_instance_feature_manager_works(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let mut models_and_buffers = manager.model_ids_and_buffers();
            let (registered_model_id, buffers) = models_and_buffers.next().unwrap();
            assert_eq!(registered_model_id, model_id);
            assert_eq!(buffers.len(), 1);
            assert_eq!(
                buffers[0].feature_type_id(),
                InstanceModelViewTransform::FEATURE_TYPE_ID
            );
            assert_eq!(buffers[0].n_valid_bytes(), 0);

            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(model_id));
            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 1);
            assert_eq!(
                buffers[0].feature_type_id(),
                InstanceModelViewTransform::FEATURE_TYPE_ID
            );
            assert_eq!(buffers[0].n_valid_bytes(), 0);
        }

        #[test]
        #[should_panic]
        fn registering_instance_with_unregistered_features_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
        }

        #[test]
        fn registering_one_instance_of_one_model_with_features_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<ZeroSizedFeature>();

            let model_id = create_dummy_model_id("");

            manager.register_instance_with_feature_type_ids(
                model_id,
                &[ZeroSizedFeature::FEATURE_TYPE_ID, Feature::FEATURE_TYPE_ID],
            );

            assert!(manager.has_model_id(model_id));
            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(
                buffers[0].feature_type_id(),
                InstanceModelViewTransform::FEATURE_TYPE_ID
            );
            assert_eq!(buffers[0].n_valid_bytes(), 0);
            assert_eq!(
                buffers[1].feature_type_id(),
                ZeroSizedFeature::FEATURE_TYPE_ID
            );
            assert_eq!(buffers[1].n_valid_bytes(), 0);
            assert_eq!(buffers[2].feature_type_id(), Feature::FEATURE_TYPE_ID);
            assert_eq!(buffers[2].n_valid_bytes(), 0);
        }

        #[test]
        fn registering_one_instance_of_two_models_with_no_features_in_instance_feature_manager_works(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id_1 = create_dummy_model_id("1");
            let model_id_2 = create_dummy_model_id("2");
            manager.register_instance_with_feature_type_ids(model_id_1, &[]);
            manager.register_instance_with_feature_type_ids(model_id_2, &[]);

            let mut models_and_buffers = manager.model_ids_and_buffers();
            assert_ne!(
                models_and_buffers.next().unwrap().0,
                models_and_buffers.next().unwrap().0
            );
            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(model_id_1));
            assert!(manager.has_model_id(model_id_2));
            assert_eq!(manager.get_buffers(model_id_1).unwrap().len(), 1);
            assert_eq!(manager.get_buffers(model_id_2).unwrap().len(), 1);
        }

        #[test]
        fn registering_two_instances_of_one_model_with_no_features_in_instance_feature_manager_works(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let mut models_and_buffers = manager.model_ids_and_buffers();
            assert_eq!(models_and_buffers.next().unwrap().0, model_id);
            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(model_id));
            assert_eq!(manager.get_buffers(model_id).unwrap().len(), 1);
        }

        #[test]
        fn registering_and_then_unregistering_one_instance_of_model_in_instance_feature_manager_works(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");

            manager.register_instance_with_feature_type_ids(model_id, &[]);
            manager.unregister_instance(model_id);

            assert!(manager.model_ids_and_buffers().next().is_none());
            assert!(!manager.has_model_id(model_id));
            assert!(manager.get_buffers(model_id).is_none());

            manager.register_instance_with_feature_type_ids(model_id, &[]);
            manager.unregister_instance(model_id);

            assert!(manager.model_ids_and_buffers().next().is_none());
            assert!(!manager.has_model_id(model_id));
            assert!(manager.get_buffers(model_id).is_none());
        }

        #[test]
        fn registering_and_then_unregistering_two_instances_of_model_in_instance_feature_manager_works(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);
            manager.register_instance_with_feature_type_ids(model_id, &[]);
            manager.unregister_instance(model_id);
            manager.unregister_instance(model_id);

            assert!(manager.model_ids_and_buffers().next().is_none());
            assert!(!manager.has_model_id(model_id));
            assert!(manager.get_buffers(model_id).is_none());
        }

        #[test]
        #[should_panic]
        fn unregistering_instance_in_empty_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.unregister_instance(model_id);
        }

        #[test]
        #[should_panic]
        fn buffering_unregistered_instance_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[]);
        }

        #[test]
        #[should_panic]
        fn buffering_multiple_of_unregistered_instance_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.buffer_multiple_instances(
                model_id,
                [InstanceModelViewTransform::identity(); 2].into_iter(),
                &[],
            );
        }

        #[test]
        fn buffering_instances_with_no_features_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let transform_1 = create_dummy_transform();
            let transform_2 = InstanceModelViewTransform::identity();

            manager.buffer_instance(model_id, &transform_1, &[]);

            let buffer = &manager.get_buffers(model_id).unwrap()[0];
            assert_eq!(buffer.n_valid_features(), 1);
            assert_eq!(
                buffer.valid_features::<InstanceModelViewTransform>(),
                &[transform_1]
            );

            manager.buffer_instance(model_id, &transform_2, &[]);

            let buffer = &manager.get_buffers(model_id).unwrap()[0];
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(
                buffer.valid_features::<InstanceModelViewTransform>(),
                &[transform_1, transform_2]
            );
        }

        #[test]
        fn buffering_multiple_instances_with_no_features_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let transform_1 = create_dummy_transform();
            let transform_2 = create_dummy_transform_2();
            let transform_3 = InstanceModelViewTransform::identity();

            manager.buffer_multiple_instances(
                model_id,
                [transform_1, transform_2].into_iter(),
                &[],
            );

            let buffer = &manager.get_buffers(model_id).unwrap()[0];
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(
                buffer.valid_features::<InstanceModelViewTransform>(),
                &[transform_1, transform_2]
            );

            manager.buffer_multiple_instances(model_id, [transform_3].into_iter(), &[]);

            let buffer = &manager.get_buffers(model_id).unwrap()[0];
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(
                buffer.valid_features::<InstanceModelViewTransform>(),
                &[transform_1, transform_2, transform_3]
            );
        }

        #[test]
        fn buffering_instance_with_features_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();

            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<DifferentFeature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(
                model_id,
                &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
            );

            let transform_instance_1 = create_dummy_transform();
            let transform_instance_2 = InstanceModelViewTransform::identity();
            let feature_1_instance_1 = Feature(22);
            let feature_1_instance_2 = Feature(43);
            let feature_2_instance_1 = DifferentFeature(-73.1);
            let feature_2_instance_2 = DifferentFeature(32.7);

            let id_1_instance_1 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1_instance_1);
            let id_2_instance_1 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2_instance_1);
            let id_1_instance_2 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1_instance_2);
            let id_2_instance_2 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2_instance_2);

            manager.buffer_instance(
                model_id,
                &transform_instance_1,
                &[id_1_instance_1, id_2_instance_1],
            );

            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(buffers[0].n_valid_features(), 1);
            assert_eq!(
                buffers[0].valid_features::<InstanceModelViewTransform>(),
                &[transform_instance_1]
            );
            assert_eq!(buffers[1].n_valid_features(), 1);
            assert_eq!(
                buffers[1].valid_features::<Feature>(),
                &[feature_1_instance_1]
            );
            assert_eq!(buffers[2].n_valid_features(), 1);
            assert_eq!(
                buffers[2].valid_features::<DifferentFeature>(),
                &[feature_2_instance_1]
            );

            manager.buffer_instance(
                model_id,
                &transform_instance_2,
                &[id_1_instance_2, id_2_instance_2],
            );

            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(buffers[0].n_valid_features(), 2);
            assert_eq!(
                buffers[0].valid_features::<InstanceModelViewTransform>(),
                &[transform_instance_1, transform_instance_2]
            );
            assert_eq!(buffers[1].n_valid_features(), 2);
            assert_eq!(
                buffers[1].valid_features::<Feature>(),
                &[feature_1_instance_1, feature_1_instance_2]
            );
            assert_eq!(buffers[2].n_valid_features(), 2);
            assert_eq!(
                buffers[2].valid_features::<DifferentFeature>(),
                &[feature_2_instance_1, feature_2_instance_2]
            );
        }

        #[test]
        fn buffering_multiple_instances_with_features_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();

            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<DifferentFeature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(
                model_id,
                &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
            );

            let transform_instance_1 = create_dummy_transform();
            let transform_instance_2 = create_dummy_transform_2();
            let transform_instance_3 = InstanceModelViewTransform::identity();
            let feature_1_instance_1 = Feature(22);
            let feature_1_instance_2 = Feature(43);
            let feature_1_instance_3 = Feature(31);
            let feature_2_instance_1 = DifferentFeature(-73.1);
            let feature_2_instance_2 = DifferentFeature(32.7);
            let feature_2_instance_3 = DifferentFeature(2.72);

            let id_1_instance_1 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1_instance_1);
            let id_2_instance_1 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2_instance_1);
            let id_1_instance_2 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1_instance_2);
            let id_2_instance_2 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2_instance_2);
            let id_1_instance_3 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1_instance_3);
            let id_2_instance_3 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2_instance_3);

            manager.buffer_multiple_instances(
                model_id,
                [transform_instance_1, transform_instance_2].into_iter(),
                &[
                    vec![id_1_instance_1, id_1_instance_2],
                    vec![id_2_instance_1, id_2_instance_2],
                ],
            );

            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(buffers[0].n_valid_features(), 2);
            assert_eq!(
                buffers[0].valid_features::<InstanceModelViewTransform>(),
                &[transform_instance_1, transform_instance_2]
            );
            assert_eq!(buffers[1].n_valid_features(), 2);
            assert_eq!(
                buffers[1].valid_features::<Feature>(),
                &[feature_1_instance_1, feature_1_instance_2]
            );
            assert_eq!(buffers[2].n_valid_features(), 2);
            assert_eq!(
                buffers[2].valid_features::<DifferentFeature>(),
                &[feature_2_instance_1, feature_2_instance_2]
            );

            manager.buffer_multiple_instances(
                model_id,
                [transform_instance_3].into_iter(),
                &[vec![id_1_instance_3], vec![id_2_instance_3]],
            );

            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(buffers[0].n_valid_features(), 3);
            assert_eq!(
                buffers[0].valid_features::<InstanceModelViewTransform>(),
                &[
                    transform_instance_1,
                    transform_instance_2,
                    transform_instance_3
                ]
            );
            assert_eq!(buffers[1].n_valid_features(), 3);
            assert_eq!(
                buffers[1].valid_features::<Feature>(),
                &[
                    feature_1_instance_1,
                    feature_1_instance_2,
                    feature_1_instance_3
                ]
            );
            assert_eq!(buffers[2].n_valid_features(), 3);
            assert_eq!(
                buffers[2].valid_features::<DifferentFeature>(),
                &[
                    feature_2_instance_1,
                    feature_2_instance_2,
                    feature_2_instance_3
                ]
            );
        }

        #[test]
        fn buffering_multiple_instances_with_repeated_feature_in_instance_feature_manager_works() {
            let mut manager = InstanceFeatureManager::new();

            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<DifferentFeature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(
                model_id,
                &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
            );

            let transform_instance_1 = create_dummy_transform();
            let transform_instance_2 = create_dummy_transform_2();
            let transform_instance_3 = InstanceModelViewTransform::identity();
            let feature_1 = Feature(22);
            let feature_2 = DifferentFeature(-73.1);

            let id_1 = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&feature_1);
            let id_2 = manager
                .get_storage_mut::<DifferentFeature>()
                .unwrap()
                .add_feature(&feature_2);

            manager.buffer_multiple_instances(
                model_id,
                [
                    transform_instance_1,
                    transform_instance_2,
                    transform_instance_3,
                ]
                .into_iter(),
                &[vec![id_1], vec![id_2]],
            );

            let buffers = manager.get_buffers(model_id).unwrap();
            assert_eq!(buffers.len(), 3);
            assert_eq!(buffers[0].n_valid_features(), 3);
            assert_eq!(
                buffers[0].valid_features::<InstanceModelViewTransform>(),
                &[
                    transform_instance_1,
                    transform_instance_2,
                    transform_instance_3
                ]
            );
            assert_eq!(buffers[1].n_valid_features(), 3);
            assert_eq!(
                buffers[1].valid_features::<Feature>(),
                &[feature_1, feature_1, feature_1]
            );
            assert_eq!(buffers[2].n_valid_features(), 3);
            assert_eq!(
                buffers[2].valid_features::<DifferentFeature>(),
                &[feature_2, feature_2, feature_2]
            );
        }

        #[test]
        #[should_panic]
        fn buffering_instance_with_too_many_feature_ids_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let id = storage.add_feature(&Feature(33));

            manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[id]);
        }

        #[test]
        #[should_panic]
        fn buffering_multiple_instances_with_too_many_feature_ids_in_instance_feature_manager_fails(
        ) {
            let mut manager = InstanceFeatureManager::new();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[]);

            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let id = storage.add_feature(&Feature(33));

            manager.buffer_multiple_instances(
                model_id,
                [InstanceModelViewTransform::identity()].into_iter(),
                &[vec![id]],
            );
        }

        #[test]
        #[should_panic]
        fn buffering_instance_with_too_few_feature_ids_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
            manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[]);
        }

        #[test]
        #[should_panic]
        fn buffering_multiple_instances_with_too_few_feature_ids_in_instance_feature_manager_fails()
        {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();
            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
            manager.buffer_multiple_instances(
                model_id,
                [InstanceModelViewTransform::identity()].into_iter(),
                &[],
            );
        }

        #[test]
        #[should_panic]
        fn buffering_instance_with_invalid_feature_id_in_instance_feature_manager_fails() {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

            let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
            let id = storage.add_feature(&DifferentFeature(-0.2));

            manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[id]);
        }

        #[test]
        #[should_panic]
        fn buffering_multiple_instances_with_invalid_feature_id_in_instance_feature_manager_fails()
        {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

            let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
            let id = storage.add_feature(&DifferentFeature(-0.2));

            manager.buffer_multiple_instances(
                model_id,
                [InstanceModelViewTransform::identity()].into_iter(),
                &[vec![id]],
            );
        }

        #[test]
        #[should_panic]
        fn buffering_multiple_instances_with_different_transform_and_feature_id_counts_in_instance_feature_manager_fails(
        ) {
            let mut manager = InstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();

            let model_id = create_dummy_model_id("");
            manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

            let id = manager
                .get_storage_mut::<Feature>()
                .unwrap()
                .add_feature(&Feature(42));

            manager.buffer_multiple_instances(
                model_id,
                [
                    create_dummy_transform(),
                    InstanceModelViewTransform::identity(),
                ]
                .into_iter(),
                &[vec![id; 3]],
            );
        }
    }

    mod storage_and_buffer {
        use super::*;
        use nalgebra::{Similarity3, Translation3, UnitQuaternion};

        type Feature = InstanceModelViewTransform;

        #[repr(transparent)]
        #[derive(Clone, Copy, Zeroable, Pod)]
        struct DifferentFeature(u8);

        #[repr(transparent)]
        #[derive(Clone, Copy, Zeroable, Pod)]
        struct ZeroSizedFeature;

        impl_InstanceFeature!(DifferentFeature, [], InstanceFeatureShaderInput::None);
        impl_InstanceFeature!(ZeroSizedFeature, [], InstanceFeatureShaderInput::None);

        fn create_dummy_feature() -> InstanceModelViewTransform {
            InstanceModelViewTransform::with_model_view_transform(Similarity3::from_parts(
                Translation3::new(2.1, -5.9, 0.01),
                UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
                7.0,
            ))
        }

        #[test]
        fn creating_new_instance_feature_storage_works() {
            let storage = InstanceFeatureStorage::new::<Feature>();

            assert_eq!(storage.feature_type_id(), Feature::FEATURE_TYPE_ID);
            assert_eq!(storage.feature_size(), Feature::FEATURE_SIZE);
            assert_eq!(storage.feature_count(), 0);
        }

        #[test]
        fn adding_features_to_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = create_dummy_feature();
            let feature_2 = InstanceModelViewTransform::identity();

            let id_1 = storage.add_feature(&feature_1);

            assert_eq!(storage.feature_count(), 1);
            assert!(storage.has_feature(id_1));
            assert_eq!(storage.feature::<Feature>(id_1), &feature_1);

            let id_2 = storage.add_feature(&feature_2);

            assert_eq!(storage.feature_count(), 2);
            assert!(storage.has_feature(id_1));
            assert!(storage.has_feature(id_2));
            assert_eq!(storage.feature::<Feature>(id_1), &feature_1);
            assert_eq!(storage.feature::<Feature>(id_2), &feature_2);
        }

        #[test]
        #[should_panic]
        fn adding_different_feature_type_to_instance_feature_storage_fails() {
            let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature = create_dummy_feature();
            storage.add_feature(&feature);
        }

        #[test]
        #[should_panic]
        fn checking_existence_of_feature_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = create_dummy_feature();
            let id_1 = storage_1.add_feature(&feature_1);
            storage_2.has_feature(id_1);
        }

        #[test]
        #[should_panic]
        fn retrieving_feature_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = create_dummy_feature();
            let id_1 = storage_1.add_feature(&feature_1);
            storage_2.feature::<Feature>(id_1);
        }

        #[test]
        #[should_panic]
        fn retrieving_feature_mutably_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let mut storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = create_dummy_feature();
            let id_1 = storage_1.add_feature(&feature_1);
            storage_2.feature_mut::<Feature>(id_1);
        }

        #[test]
        fn modifying_feature_in_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature = DifferentFeature(7);
            let id = storage.add_feature(&feature);
            let stored_feature = storage.feature_mut::<DifferentFeature>(id);
            assert_eq!(stored_feature.0, 7);
            stored_feature.0 = 42;
            assert_eq!(storage.feature::<DifferentFeature>(id).0, 42);
        }

        #[test]
        fn removing_features_from_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = create_dummy_feature();
            let feature_2 = InstanceModelViewTransform::identity();

            let id_1 = storage.add_feature(&feature_1);
            let id_2 = storage.add_feature(&feature_2);

            storage.remove_feature(id_1);

            assert_eq!(storage.feature_count(), 1);
            assert!(!storage.has_feature(id_1));
            assert!(storage.has_feature(id_2));
            assert_eq!(storage.feature::<Feature>(id_2), &feature_2);

            storage.remove_feature(id_2);

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
            assert!(!storage.has_feature(id_2));

            let id_1 = storage.add_feature(&feature_1);
            let id_2 = storage.add_feature(&feature_2);

            storage.remove_feature(id_2);

            assert_eq!(storage.feature_count(), 1);
            assert!(!storage.has_feature(id_2));
            assert!(storage.has_feature(id_1));
            assert_eq!(storage.feature::<Feature>(id_1), &feature_1);

            storage.remove_feature(id_1);

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
            assert!(!storage.has_feature(id_2));
        }

        #[test]
        #[should_panic]
        fn removing_missing_feature_in_instance_feature_storage_fails() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature = create_dummy_feature();
            let id = storage.add_feature(&feature);
            storage.remove_feature(id);
            storage.remove_feature(id);
        }

        #[test]
        fn removing_all_features_from_empty_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            storage.remove_all_features();
            assert_eq!(storage.feature_count(), 0);
        }

        #[test]
        fn removing_all_features_from_single_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = create_dummy_feature();

            let id_1 = storage.add_feature(&feature_1);

            storage.remove_all_features();

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
        }

        #[test]
        fn removing_all_features_from_multi_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = create_dummy_feature();
            let feature_2 = InstanceModelViewTransform::identity();

            let id_1 = storage.add_feature(&feature_1);
            let id_2 = storage.add_feature(&feature_2);

            storage.remove_all_features();

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
            assert!(!storage.has_feature(id_2));
        }

        #[test]
        fn adding_zero_sized_features_to_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<ZeroSizedFeature>();

            let id_1 = storage.add_feature(&ZeroSizedFeature);

            assert_eq!(storage.feature_count(), 1);
            assert!(storage.has_feature(id_1));

            let id_2 = storage.add_feature(&ZeroSizedFeature);

            assert_eq!(storage.feature_count(), 2);
            assert!(storage.has_feature(id_1));
            assert!(storage.has_feature(id_2));
            assert_ne!(id_1, id_2);
        }

        #[test]
        #[should_panic]
        fn retrieving_zero_sized_feature_in_instance_feature_storage_fails() {
            let mut storage = InstanceFeatureStorage::new::<ZeroSizedFeature>();
            let id_1 = storage.add_feature(&ZeroSizedFeature);
            storage.feature::<ZeroSizedFeature>(id_1);
        }

        #[test]
        #[should_panic]
        fn retrieving_zero_sized_feature_mutably_in_instance_feature_storage_fails() {
            let mut storage = InstanceFeatureStorage::new::<ZeroSizedFeature>();
            let id_1 = storage.add_feature(&ZeroSizedFeature);
            storage.feature_mut::<ZeroSizedFeature>(id_1);
        }

        #[test]
        fn removing_zero_sized_features_from_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<ZeroSizedFeature>();

            let id_1 = storage.add_feature(&ZeroSizedFeature);
            let id_2 = storage.add_feature(&ZeroSizedFeature);

            storage.remove_feature(id_1);

            assert_eq!(storage.feature_count(), 1);
            assert!(!storage.has_feature(id_1));
            assert!(storage.has_feature(id_2));

            storage.remove_feature(id_2);

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
            assert!(!storage.has_feature(id_2));

            let id_1 = storage.add_feature(&ZeroSizedFeature);
            let id_2 = storage.add_feature(&ZeroSizedFeature);

            storage.remove_feature(id_2);

            assert_eq!(storage.feature_count(), 1);
            assert!(!storage.has_feature(id_2));
            assert!(storage.has_feature(id_1));

            storage.remove_feature(id_1);

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
            assert!(!storage.has_feature(id_2));
        }

        #[test]
        fn creating_instance_feature_buffer_works() {
            let buffer = DynamicInstanceFeatureBuffer::new::<Feature>();

            assert_eq!(buffer.feature_type_id(), Feature::FEATURE_TYPE_ID);
            assert_eq!(buffer.feature_size(), Feature::FEATURE_SIZE);
            assert_eq!(buffer.n_valid_bytes(), 0);
            assert_eq!(buffer.n_valid_features(), 0);
            assert!(buffer.valid_bytes().is_empty());
        }
        #[test]
        fn adding_one_feature_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = create_dummy_feature();
            buffer.add_feature(&feature);

            assert_eq!(buffer.n_valid_bytes(), mem::size_of::<Feature>());
            assert_eq!(buffer.n_valid_features(), 1);
            assert_eq!(buffer.valid_bytes(), bytemuck::bytes_of(&feature));
            assert_eq!(buffer.valid_features::<Feature>(), &[feature]);
        }

        #[test]
        fn adding_two_features_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();
            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);

            let feature_slice = &[feature_1, feature_2];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_three_features_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = create_dummy_feature();
            let feature_2 = InstanceModelViewTransform::identity();
            let feature_3 = create_dummy_feature();

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);
            buffer.add_feature(&feature_3);

            let feature_slice = &[feature_1, feature_2, feature_3];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_feature_slice_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();
            buffer.add_feature_slice(&[feature_1, feature_2]);

            let feature_slice = &[feature_1, feature_2];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_two_feature_slices_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();
            let feature_3 = create_dummy_feature();

            buffer.add_feature_slice(&[feature_1, feature_2]);
            buffer.add_feature_slice(&[feature_3]);

            let feature_slice = &[feature_1, feature_2, feature_3];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_features_from_iterator_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let features = vec![
                InstanceModelViewTransform::identity(),
                create_dummy_feature(),
                create_dummy_feature(),
            ];

            buffer.add_features_from_iterator(features.iter().cloned());

            let feature_slice = features.as_slice();
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_feature_from_storage_to_instance_feature_buffer_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature = create_dummy_feature();
            let id = storage.add_feature(&feature);

            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.add_feature_from_storage(&storage, id);

            assert_eq!(buffer.n_valid_bytes(), mem::size_of::<Feature>());
            assert_eq!(buffer.n_valid_features(), 1);
            assert_eq!(buffer.valid_bytes(), bytemuck::bytes_of(&feature));
            assert_eq!(buffer.valid_features::<Feature>(), &[feature]);
        }

        #[test]
        fn adding_two_features_from_storage_to_instance_feature_buffer_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();
            let id_1 = storage.add_feature(&feature_1);
            let id_2 = storage.add_feature(&feature_2);

            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.add_feature_from_storage(&storage, id_2);
            buffer.add_feature_from_storage(&storage, id_1);

            let feature_slice = &[feature_2, feature_1];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn adding_feature_from_storage_repeatedly_to_instance_feature_buffer_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature = create_dummy_feature();
            let id = storage.add_feature(&feature);

            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.add_feature_from_storage_repeatedly(&storage, id, 3);

            let feature_slice = &[feature; 3];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn clearing_empty_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.clear();

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert_eq!(buffer.n_valid_features(), 0);
            assert!(buffer.valid_bytes().is_empty());
            assert_eq!(buffer.valid_features::<Feature>(), &[]);
        }

        #[test]
        fn clearing_one_feature_from_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();

            buffer.add_feature(&feature_1);
            buffer.clear();

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert_eq!(buffer.n_valid_features(), 0);
            assert!(buffer.valid_bytes().is_empty());
            assert_eq!(buffer.valid_features::<Feature>(), &[]);

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);

            let feature_slice = &[feature_1, feature_2];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn clearing_two_features_from_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = InstanceModelViewTransform::identity();
            let feature_2 = create_dummy_feature();

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);
            buffer.clear();

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert_eq!(buffer.n_valid_features(), 0);
            assert!(buffer.valid_bytes().is_empty());
            assert_eq!(buffer.valid_features::<Feature>(), &[]);

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);

            let feature_slice = &[feature_1, feature_2];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 2);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        fn clearing_three_features_from_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = create_dummy_feature();
            let feature_2 = InstanceModelViewTransform::identity();
            let feature_3 = create_dummy_feature();

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);
            buffer.add_feature(&feature_3);
            buffer.clear();

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert_eq!(buffer.n_valid_features(), 0);
            assert!(buffer.valid_bytes().is_empty());
            assert_eq!(buffer.valid_features::<Feature>(), &[]);

            buffer.add_feature(&feature_1);
            buffer.add_feature(&feature_2);
            buffer.add_feature(&feature_3);

            let feature_slice = &[feature_1, feature_2, feature_3];
            let feature_bytes: &[u8] = bytemuck::cast_slice(feature_slice);

            assert_eq!(buffer.n_valid_bytes(), feature_bytes.len());
            assert_eq!(buffer.n_valid_features(), 3);
            assert_eq!(buffer.valid_bytes(), feature_bytes);
            assert_eq!(buffer.valid_features::<Feature>(), feature_slice);
        }

        #[test]
        #[should_panic]
        fn adding_feature_of_different_type_to_instance_feature_buffer_fails() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = DifferentFeature(5);
            buffer.add_feature(&feature);
        }

        #[test]
        #[should_panic]
        fn requesting_valid_features_of_different_type_from_instance_feature_buffer_fails() {
            let buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.valid_features::<DifferentFeature>();
        }

        #[test]
        fn creating_instance_feature_buffer_with_zero_sized_feature_works() {
            let buffer = DynamicInstanceFeatureBuffer::new::<ZeroSizedFeature>();

            assert_eq!(buffer.feature_type_id(), ZeroSizedFeature::FEATURE_TYPE_ID);
            assert_eq!(buffer.feature_size(), ZeroSizedFeature::FEATURE_SIZE);
            assert_eq!(buffer.n_valid_bytes(), 0);
            assert!(buffer.valid_bytes().is_empty());
        }

        #[test]
        fn adding_zero_sized_features_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<ZeroSizedFeature>();

            buffer.add_feature(&ZeroSizedFeature);

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert!(buffer.valid_bytes().is_empty());

            buffer.add_feature(&ZeroSizedFeature);

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert!(buffer.valid_bytes().is_empty());

            buffer.clear();

            assert_eq!(buffer.n_valid_bytes(), 0);
            assert!(buffer.valid_bytes().is_empty());
        }

        #[test]
        #[should_panic]
        fn requesting_valid_zero_sized_features_from_instance_feature_buffer_fails() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<ZeroSizedFeature>();
            buffer.add_feature(&ZeroSizedFeature);
            buffer.valid_features::<ZeroSizedFeature>();
        }

        #[test]
        #[should_panic]
        fn requesting_number_of_valid_zero_sized_features_from_instance_feature_buffer_fails() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<ZeroSizedFeature>();
            buffer.add_feature(&ZeroSizedFeature);
            buffer.n_valid_features();
        }

        #[test]
        fn requesting_initial_range_in_empty_instance_feature_buffer_works() {
            let buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            assert_eq!(buffer.initial_valid_feature_range(), 0..0);
        }

        #[test]
        #[should_panic]
        fn defining_range_with_existing_id_in_instance_feature_buffer_fails() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.begin_range(0);
            buffer.begin_range(0);
        }

        #[test]
        #[should_panic]
        fn requesting_range_with_invalid_id_in_instance_feature_buffer_fails() {
            let buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.valid_feature_range(0);
        }

        #[test]
        #[should_panic]
        fn requesting_range_with_id_invalidated_by_clearing_instance_feature_buffer_fails() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            buffer.begin_range(0);
            buffer.clear();
            buffer.valid_feature_range(0);
        }

        #[test]
        fn creating_single_range_from_beginning_in_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = create_dummy_feature();

            buffer.begin_range(0);

            assert_eq!(buffer.valid_feature_range(0), 0..0);

            buffer.add_feature(&feature);
            assert_eq!(buffer.valid_feature_range(0), 0..1);

            buffer.add_feature(&feature);
            assert_eq!(buffer.valid_feature_range(0), 0..2);
        }

        #[test]
        fn creating_single_range_not_from_beginning_in_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = create_dummy_feature();

            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            let range_id = 0;
            buffer.begin_range(range_id);

            assert_eq!(buffer.valid_feature_range(range_id), 2..2);

            buffer.add_feature(&feature);
            assert_eq!(buffer.valid_feature_range(range_id), 2..3);

            buffer.add_feature(&feature);
            assert_eq!(buffer.valid_feature_range(range_id), 2..4);
        }

        #[test]
        fn creating_multiple_ranges_from_beginning_in_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = create_dummy_feature();

            buffer.begin_range(0);

            buffer.begin_range(1);
            buffer.add_feature(&feature);

            buffer.begin_range(2);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            buffer.begin_range(3);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            assert_eq!(buffer.initial_valid_feature_range(), 0..0);
            assert_eq!(buffer.valid_feature_range(0), 0..0);
            assert_eq!(buffer.valid_feature_range(1), 0..1);
            assert_eq!(buffer.valid_feature_range(2), 1..3);
            assert_eq!(buffer.valid_feature_range(3), 3..6);
        }

        #[test]
        fn creating_multiple_ranges_not_from_beginning_in_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature = create_dummy_feature();

            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            buffer.begin_range(0);

            buffer.begin_range(1);
            buffer.add_feature(&feature);

            buffer.begin_range(2);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            buffer.begin_range(3);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);
            buffer.add_feature(&feature);

            assert_eq!(buffer.initial_valid_feature_range(), 0..2);
            assert_eq!(buffer.valid_feature_range(0), 2..2);
            assert_eq!(buffer.valid_feature_range(1), 2..3);
            assert_eq!(buffer.valid_feature_range(2), 3..5);
            assert_eq!(buffer.valid_feature_range(3), 5..8);
        }
    }
}
