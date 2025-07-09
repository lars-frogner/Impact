//! Management of data associated with models that may have many instances.

#[macro_use]
pub mod macros;

pub mod buffer;
pub mod transform;

pub use transform::register_model_feature_types;

use buffer::InstanceFeatureGPUBufferManager;
use bytemuck::{Pod, Zeroable};
use impact_containers::{
    AlignedByteVec, Alignment, HashMap, HashSet, KeyIndexMapper, NoHashKeyIndexMapper,
};
use impact_gpu::{device::GraphicsDevice, wgpu};
use impact_math::{self, Hash64};
use roc_integration::roc;
use std::{borrow::Cow, hash::Hash, mem, ops::Range};

/// Represents a piece of data associated with a model instance.
pub trait InstanceFeature: Pod {
    /// A unique ID representing the feature type.
    const FEATURE_TYPE_ID: InstanceFeatureTypeID;

    /// The size of the feature type in bytes.
    const FEATURE_SIZE: usize = mem::size_of::<Self>();

    /// The memory alignment of the feature type.
    const FEATURE_ALIGNMENT: Alignment = Alignment::of::<Self>();

    /// The layout of the vertex GPU buffer that can be used to pass the feature
    /// to the GPU. If [`None`], no GPU buffer will be created for instance
    /// features of this type.
    const BUFFER_LAYOUT: Option<wgpu::VertexBufferLayout<'static>>;

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

/// Container for features (distinct pieces of data) associated with instances
/// of specific models.
///
/// Holds a set of [`InstanceFeatureStorage`]s, one storage for each feature
/// type. These storages are presistent and can be accessed to add, remove or
/// modify feature values for individual instances.
///
/// Additionally, a set of [`DynamicInstanceFeatureBuffer`]s is kept for each
/// model that has instances, one buffer for each feature type associated with
/// the model. These buffers are filled with feature values from the
/// `InstanceFeatureStorage`s for all instances that are to be rendered. Their
/// contents can then be copied directly to the corresponding GPU buffers,
/// before they are cleared in preparation for the next frame.
#[derive(Debug, Default)]
pub struct InstanceFeatureManager<MID> {
    feature_storages: HashMap<InstanceFeatureTypeID, InstanceFeatureStorage>,
    instance_buffers: HashMap<MID, ModelInstanceBuffer>,
}

/// Record of the state of an [`InstanceFeatureManager`].
#[derive(Clone, Debug)]
pub struct InstanceFeatureManagerState<MID> {
    model_ids: HashSet<MID>,
}

/// Container for the [`DynamicInstanceFeatureBuffer`]s holding buffered
/// features for all instances of one specific model.
#[derive(Debug, Default)]
pub struct ModelInstanceBuffer {
    feature_buffers: Vec<DynamicInstanceFeatureBuffer>,
    buffer_index_map: KeyIndexMapper<InstanceFeatureTypeID>,
    instance_count: usize,
}

/// Identifier for a type of instance feature.
pub type InstanceFeatureTypeID = Hash64;

/// Identifier for an instance feature value.
#[roc(parents = "Model")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct InstanceFeatureID {
    feature_type_id: InstanceFeatureTypeID,
    idx: u64,
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
    vertex_buffer_layout: Option<wgpu::VertexBufferLayout<'static>>,
    bytes: AlignedByteVec,
    index_map: NoHashKeyIndexMapper<usize>,
    feature_id_count: usize,
}

/// A buffer for instance feature values of the same type.
///
/// The buffer is grown on demand, but never shrunk. Instead, a counter keeps
/// track of the position of the last valid byte in the buffer, and the counter
/// is reset to zero when the buffer is cleared. This allows it to be filled
/// and emptied repeatedly without unneccesary allocations.
///
/// Stores the raw bytes of the features to avoid exposing the feature type
/// signature. The type information is extracted on construction and used to
/// validate access requests.
#[derive(Debug)]
pub struct DynamicInstanceFeatureBuffer {
    type_descriptor: InstanceFeatureTypeDescriptor,
    vertex_buffer_layout: Option<wgpu::VertexBufferLayout<'static>>,
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

#[derive(Copy, Clone, Debug)]
struct InstanceFeatureTypeDescriptor {
    id: InstanceFeatureTypeID,
    size: usize,
    alignment: Alignment,
}

impl<MID: Clone + Eq + Hash> InstanceFeatureManager<MID> {
    /// Creates a new empty instance feature manager.
    pub fn new() -> Self {
        Self {
            feature_storages: HashMap::default(),
            instance_buffers: HashMap::default(),
        }
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

    /// Records the current state of the instance feature manager and returns it as a
    /// [`InstanceFeatureManagerState`].
    pub fn record_state(&self) -> InstanceFeatureManagerState<MID> {
        InstanceFeatureManagerState {
            model_ids: self.instance_buffers.keys().cloned().collect(),
        }
    }

    /// Whether the manager has instance feature buffers for the model with the
    /// given ID.
    pub fn has_model_id(&self, model_id: &MID) -> bool {
        self.instance_buffers.contains_key(model_id)
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

    /// Returns a reference to the value of the feature stored under the given ID.
    ///
    /// # Panics
    /// - If the feature's type has not been registered.
    /// - If no feature with the given ID exists in the associated storage.
    pub fn feature<Fe: InstanceFeature>(&self, feature_id: InstanceFeatureID) -> &Fe {
        self.get_storage::<Fe>()
            .expect("Missing storage for instance feature type")
            .feature::<Fe>(feature_id)
    }

    /// Returns a mutable reference to the value of the feature stored under the
    /// given ID.
    ///
    /// # Panics
    /// - If the feature's type has not been registered.
    /// - If no feature with the given ID exists in the associated storage.
    pub fn feature_mut<Fe: InstanceFeature>(&mut self, feature_id: InstanceFeatureID) -> &mut Fe {
        self.get_storage_mut::<Fe>()
            .expect("Missing storage for instance feature type")
            .feature_mut::<Fe>(feature_id)
    }

    /// Returns a reference to the [`ModelInstanceBuffer`] for the model with
    /// the given ID, or [`None`] if the model is not present.
    pub fn get_model_instance_buffer(&self, model_id: &MID) -> Option<&ModelInstanceBuffer> {
        self.instance_buffers.get(model_id)
    }

    /// Returns a mutable reference to the [`ModelInstanceBuffer`] for the model
    /// with the given ID, or [`None`] if the model is not present.
    pub fn get_model_instance_buffer_mut(
        &mut self,
        model_id: &MID,
    ) -> Option<&mut ModelInstanceBuffer> {
        self.instance_buffers.get_mut(model_id)
    }

    /// Returns an iterator over the model IDs and their associated instance
    /// buffers.
    pub fn model_ids_and_instance_buffers(
        &self,
    ) -> impl Iterator<Item = (&MID, &'_ ModelInstanceBuffer)> {
        self.instance_buffers.iter()
    }

    /// Returns an iterator over the model IDs and their associated instance
    /// buffers, with the buffers being mutable.
    pub fn model_ids_and_mutable_instance_buffers(
        &mut self,
    ) -> impl Iterator<Item = (&MID, &'_ mut ModelInstanceBuffer)> {
        self.instance_buffers.iter_mut()
    }

    /// Initialize the [`ModelInstanceBuffer`] associated with the given model
    /// for the given feature types.
    ///
    /// # Panics
    /// - If the `ModelInstanceBuffer` for the model has already been
    ///   initialized with a different set of feature types than provided.
    /// - If any of the model's feature types have not been registered with
    ///   [`Self::register_feature_type`].
    pub fn initialize_instance_buffer(
        &mut self,
        model_id: MID,
        feature_type_ids: &[InstanceFeatureTypeID],
    ) {
        self.instance_buffers
            .entry(model_id)
            .and_modify(|instance_buffer| {
                assert_eq!(instance_buffer.n_feature_types(), feature_type_ids.len());
            })
            .or_insert_with(|| {
                ModelInstanceBuffer::new(feature_type_ids.iter().map(|feature_type_id| {
                    self.feature_storages.get(feature_type_id).expect(
                        "Missing storage for instance feature type \
                              (all feature types must be registered with `register_feature_type`)",
                    )
                }))
            });
    }

    /// Registers the existence of a new instance of the model with the given
    /// ID, where the model has associated features of the given types. This
    /// involves initializing the [`ModelInstanceBuffer`] associated with the
    /// model if it do not already exist.
    ///
    /// # Panics
    /// - If the `ModelInstanceBuffer` for the model has already been
    ///   initialized with a different set of feature types than provided.
    /// - If any of the model's feature types have not been registered with
    ///   [`Self::register_feature_type`].
    pub fn register_instance(&mut self, model_id: MID, feature_type_ids: &[InstanceFeatureTypeID]) {
        self.instance_buffers
            .entry(model_id)
            .and_modify(|instance_buffer| {
                assert_eq!(instance_buffer.n_feature_types(), feature_type_ids.len());
                instance_buffer.register_instance();
            })
            .or_insert_with(|| {
                ModelInstanceBuffer::new(feature_type_ids.iter().map(|feature_type_id| {
                    self.feature_storages.get(feature_type_id).expect(
                        "Missing storage for instance feature type \
                              (all feature types must be registered with `register_feature_type`)",
                    )
                }))
            });
    }

    /// Informs the manager that an instance of the model with the given ID has
    /// been deleted, so that the associated [`ModelInstanceBuffer`] can be
    /// deleted if this was the last instance of that model.
    ///
    /// # Panics
    /// If no instance of the specified model exists.
    pub fn unregister_instance(&mut self, model_id: &MID) {
        let instance_buffer = self
            .get_model_instance_buffer_mut(model_id)
            .expect("Tried to unregister instance of model that has no instances");

        instance_buffer.unregister_instance();

        if !instance_buffer.has_instances() {
            self.instance_buffers.remove(model_id);
        }
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the values of the features with the given IDs from their storages
    /// onto the buffers.
    ///
    /// # Panics
    /// - If no [`ModelInstanceBuffer`] exists for the model with the given ID.
    /// - If any of the feature types are not used by this model.
    pub fn buffer_instance_features_from_storages(
        &mut self,
        model_id: &MID,
        feature_ids: &[InstanceFeatureID],
    ) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to buffer instance of missing model");

        instance_buffer.buffer_instance_features_from_storage(&self.feature_storages, feature_ids);
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the value of the feature with the given IDs from its storage
    /// onto the corresponding buffer.
    ///
    /// # Panics
    /// - If no [`ModelInstanceBuffer`] exists for the model with the given ID.
    /// - If the model does not have a buffer for the feature type.
    /// - If there is no storage for features of the given type.
    pub fn buffer_instance_feature_from_storage(
        &mut self,
        model_id: &MID,
        feature_id: InstanceFeatureID,
    ) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to buffer instance of missing model");

        instance_buffer.buffer_instance_feature_from_storage(&self.feature_storages, feature_id);
    }

    /// Pushes the given feature value onto the associated buffer for the model
    /// with the given ID.
    ///
    /// # Panics
    /// - If no [`ModelInstanceBuffer`] exists for the model with the given ID.
    /// - If the model does not have a buffer for the feature type.
    pub fn buffer_instance_feature<Fe: InstanceFeature>(&mut self, model_id: &MID, feature: &Fe) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to buffer instance of missing model");

        instance_buffer.buffer_instance_feature::<Fe>(feature);
    }

    /// Pushes a copy of the given slice of feature values onto the associated
    /// buffer for the model with the given ID.
    ///
    /// # Panics
    /// - If no [`ModelInstanceBuffer`] exists for the model with the given ID.
    /// - If the model does not have a buffer for the feature type.
    pub fn buffer_instance_feature_slice<Fe: InstanceFeature>(
        &mut self,
        model_id: &MID,
        features: &[Fe],
    ) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to buffer instances of missing model");

        instance_buffer.buffer_instance_feature_slice::<Fe>(features);
    }

    /// Calls [`DynamicInstanceFeatureBuffer::begin_range`] with the given range
    /// ID for all instance feature buffers holding the given feature type
    /// (across all models).
    pub fn begin_range_in_feature_buffers(
        &mut self,
        feature_type_id: InstanceFeatureTypeID,
        range_id: InstanceFeatureBufferRangeID,
    ) {
        for instance_buffer in self.instance_buffers.values_mut() {
            instance_buffer.begin_range_in_feature_buffer(feature_type_id, range_id);
        }
    }

    /// Calls [`DynamicInstanceFeatureBuffer::begin_range`] with the given range
    /// ID for the instance feature buffers holding the given feature types
    /// for the given model.
    pub fn begin_ranges_in_feature_buffers_for_model(
        &mut self,
        model_id: &MID,
        feature_type_ids: &[InstanceFeatureTypeID],
        range_id: InstanceFeatureBufferRangeID,
    ) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to begin range in feature buffer for missing model");

        for feature_type_id in feature_type_ids {
            instance_buffer.begin_range_in_feature_buffer(*feature_type_id, range_id);
        }
    }

    /// Clears any previously buffered feature values from all instance feature
    /// buffers.
    pub fn clear_buffer_contents(&mut self) {
        for instance_buffer in self.instance_buffers.values_mut() {
            instance_buffer.clear_buffer_contents();
        }
    }

    /// Removes the instance buffers that are not part of the given manager
    /// state. Also removes all buffered features in the feature storages.
    pub fn reset_to_state(&mut self, state: &InstanceFeatureManagerState<MID>) {
        self.instance_buffers
            .retain(|model_id, _| state.model_ids.contains(model_id));

        for storage in self.feature_storages.values_mut() {
            storage.remove_all_features();
        }
    }

    /// Returns mutable references to the first and second instance feature
    /// buffers of the model with the given ID, along with references to the
    /// associated storages.
    ///
    /// # Panics
    /// If any of the requested buffers are not present.
    pub fn first_and_second_feature_buffer_mut_with_storages(
        &mut self,
        model_id: &MID,
    ) -> (
        (&mut DynamicInstanceFeatureBuffer, &InstanceFeatureStorage),
        (&mut DynamicInstanceFeatureBuffer, &InstanceFeatureStorage),
    ) {
        let instance_buffer = self
            .instance_buffers
            .get_mut(model_id)
            .expect("Tried to buffer instances of missing model");

        let (first_buffer, second_buffer) =
            instance_buffer.first_and_second_feature_buffer_mut_with_storage();

        let first_storage = self
            .feature_storages
            .get(&first_buffer.feature_type_id())
            .expect("Missing storage associated with first instance feature buffer");

        let second_storage = self
            .feature_storages
            .get(&second_buffer.feature_type_id())
            .expect("Missing storage associated with second instance feature buffer");

        (
            (first_buffer, first_storage),
            (second_buffer, second_storage),
        )
    }
}

impl ModelInstanceBuffer {
    fn new<'a>(feature_storages: impl IntoIterator<Item = &'a InstanceFeatureStorage>) -> Self {
        let mut buffer_index_map = KeyIndexMapper::new();
        let mut feature_buffers = Vec::new();

        for storage in feature_storages {
            let buffer = DynamicInstanceFeatureBuffer::new_for_storage(storage);
            buffer_index_map.push_key(buffer.feature_type_id());
            feature_buffers.push(buffer);
        }

        Self {
            feature_buffers,
            buffer_index_map,
            instance_count: 1,
        }
    }

    /// Creates a new GPU buffer manager for each feature type associated with
    /// the model and copies the buffered feature values to the new GPU buffers
    /// The buffer managers are returned in the same order as the feature types
    /// passed to the [`InstanceFeatureManager::register_instance`] calls
    /// for this model.
    ///
    /// Call [`Self::copy_buffered_instance_features_to_gpu_buffers`] with the
    /// same list of GPU buffer managers for subsequent moves of buffered
    /// feature values to the GPU buffers.
    pub fn copy_buffered_instance_features_to_new_gpu_buffers(
        &mut self,
        graphics_device: &GraphicsDevice,
        label: Cow<'static, str>,
    ) -> Vec<InstanceFeatureGPUBufferManager> {
        self.feature_buffers
            .iter_mut()
            .filter_map(|feature_buffer| {
                InstanceFeatureGPUBufferManager::new(graphics_device, feature_buffer, label.clone())
            })
            .collect()
    }

    /// Copies all buffered feature values to the given GPU buffers.
    ///
    /// # Panics
    /// If the GPU buffer managers are not given in the same order as returned
    /// from [`Self::copy_buffered_instance_features_to_new_gpu_buffers`].
    pub fn copy_buffered_instance_features_to_gpu_buffers(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_buffer_managers: &mut [InstanceFeatureGPUBufferManager],
    ) {
        for (feature_buffer, gpu_buffer_manager) in self
            .feature_buffers
            .iter_mut()
            .filter(|buffer| buffer.vertex_buffer_layout().is_some())
            .zip(gpu_buffer_managers)
        {
            gpu_buffer_manager
                .copy_instance_features_to_gpu_buffer(graphics_device, feature_buffer);
        }
    }

    /// Returns the number of feature types associated with the model.
    fn n_feature_types(&self) -> usize {
        self.feature_buffers.len()
    }

    /// Returns a reference to the [`DynamicInstanceFeatureBuffer`] for
    /// features of the given type, or [`None`] if the modes does not use this
    /// feature type.
    pub fn get_feature_buffer(
        &self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&DynamicInstanceFeatureBuffer> {
        let idx = self.buffer_index_map.get(feature_type_id)?;
        Some(&self.feature_buffers[idx])
    }

    /// Returns a mutable reference to the [`DynamicInstanceFeatureBuffer`] for
    /// features of the given type, or [`None`] if the modes does not use this
    /// feature type.
    pub fn get_feature_buffer_mut(
        &mut self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&mut DynamicInstanceFeatureBuffer> {
        let idx = self.buffer_index_map.get(feature_type_id)?;
        Some(&mut self.feature_buffers[idx])
    }

    /// Whether any instances of the model have been registered.
    fn has_instances(&self) -> bool {
        self.instance_count > 0
    }

    /// Registers the existence of a new instance of the model.
    fn register_instance(&mut self) {
        self.instance_count += 1;
    }

    /// Informs that an instance of the model has been deleted.
    fn unregister_instance(&mut self) {
        assert!(self.instance_count > 0);
        self.instance_count -= 1;
    }

    /// Finds the instance feature buffers for the model and pushes the values
    /// of the features with the given IDs from the given storages onto the
    /// buffers.
    ///
    /// # Panics
    /// - If any of the feature types are not used by this model.
    /// - If any of the feature types are missing a storage.
    fn buffer_instance_features_from_storage(
        &mut self,
        feature_storages: &HashMap<InstanceFeatureTypeID, InstanceFeatureStorage>,
        feature_ids: &[InstanceFeatureID],
    ) {
        for &feature_id in feature_ids {
            let feature_buffer = self
                .get_feature_buffer_mut(feature_id.feature_type_id())
                .expect("Missing buffer for model instance feature");

            let storage = feature_storages
                .get(&feature_id.feature_type_id())
                .expect("Missing storage for model instance feature");

            feature_buffer.add_feature_from_storage(storage, feature_id);
        }
    }

    /// Finds the instance feature buffers for the model and pushes the value
    /// of the feature with the given IDs from the given storages onto the
    /// buffers.
    ///
    /// # Panics
    /// - If the model does not have a buffer for the feature type.
    /// - If none of the storages are for features of the given type.
    fn buffer_instance_feature_from_storage(
        &mut self,
        feature_storages: &HashMap<InstanceFeatureTypeID, InstanceFeatureStorage>,
        feature_id: InstanceFeatureID,
    ) {
        self.get_feature_buffer_mut(feature_id.feature_type_id())
            .expect("Missing feature buffer for feature type")
            .add_feature_from_storage(
                feature_storages
                    .get(&feature_id.feature_type_id())
                    .expect("Missing storage for model instance feature"),
                feature_id,
            );
    }

    /// Pushes the given feature value onto the associated buffer.
    ///
    /// # Panics
    /// If the model does not have a buffer for the feature type.
    fn buffer_instance_feature<Fe: InstanceFeature>(&mut self, feature: &Fe) {
        let feature_buffer = self
            .get_feature_buffer_mut(Fe::FEATURE_TYPE_ID)
            .expect("Missing feature buffer for feature type");

        feature_buffer.add_feature(feature);
    }

    /// Pushes a copy of the given slice of feature values onto the associated buffer.
    ///
    /// # Panics
    /// If the model does not have a buffer for the feature type.
    fn buffer_instance_feature_slice<Fe: InstanceFeature>(&mut self, features: &[Fe]) {
        let feature_buffer = self
            .get_feature_buffer_mut(Fe::FEATURE_TYPE_ID)
            .expect("Missing feature buffer for feature type");

        feature_buffer.add_feature_slice(features);
    }

    /// Calls [`DynamicInstanceFeatureBuffer::begin_range`] with the given range
    /// ID for the instance feature buffer holding the given feature type.
    /// If the model has no instance feature buffer for the given feature type,
    /// nothing happens.
    fn begin_range_in_feature_buffer(
        &mut self,
        feature_type_id: InstanceFeatureTypeID,
        range_id: InstanceFeatureBufferRangeID,
    ) {
        if let Some(feature_buffer) = self.get_feature_buffer_mut(feature_type_id) {
            feature_buffer.begin_range(range_id);
        }
    }

    fn first_and_second_feature_buffer_mut_with_storage(
        &mut self,
    ) -> (
        &mut DynamicInstanceFeatureBuffer,
        &mut DynamicInstanceFeatureBuffer,
    ) {
        let (first_buffer, remaining_buffers) = self
            .feature_buffers
            .split_first_mut()
            .expect("Missing first instance feature buffer");

        let (second_buffer, _) = remaining_buffers
            .split_first_mut()
            .expect("Missing second instance feature buffer");

        (first_buffer, second_buffer)
    }

    fn clear_buffer_contents(&mut self) {
        for buffer in &mut self.feature_buffers {
            buffer.clear();
        }
    }
}

impl InstanceFeatureID {
    /// Creates an ID that does not represent a valid feature.
    pub fn not_applicable() -> Self {
        Self {
            feature_type_id: Hash64::zeroed(),
            idx: u64::MAX,
        }
    }

    pub const fn idx(&self) -> usize {
        self.idx as usize
    }

    /// Returns the ID of the type of feature this ID identifies.
    pub fn feature_type_id(&self) -> InstanceFeatureTypeID {
        self.feature_type_id
    }

    /// Returns `true` if this ID does not represent a valid feature.
    pub fn is_not_applicable(&self) -> bool {
        self.feature_type_id == Hash64::zeroed() && self.idx == u64::MAX
    }
}

impl InstanceFeatureStorage {
    /// Creates a new empty storage for features of type `Fe`.
    pub fn new<Fe: InstanceFeature>() -> Self {
        Self {
            type_descriptor: InstanceFeatureTypeDescriptor::for_type::<Fe>(),
            vertex_buffer_layout: Fe::BUFFER_LAYOUT,
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
        self.index_map.contains_key(feature_id.idx())
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
        self.index_map.push_key(feature_id.idx());
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
        let feature_idx = self.index_map.swap_remove_key(feature_id.idx());

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

    /// Returns the layout of the vertex GPU buffer that can be used for the
    /// stored features.
    pub(crate) fn vertex_buffer_layout(&self) -> Option<wgpu::VertexBufferLayout<'static>> {
        self.vertex_buffer_layout.clone()
    }

    fn type_descriptor(&self) -> InstanceFeatureTypeDescriptor {
        self.type_descriptor
    }

    fn feature_bytes(&self, feature_id: InstanceFeatureID) -> &[u8] {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        let feature_idx = self.index_map.idx(feature_id.idx());
        let byte_range = self.feature_byte_range(feature_idx);
        &self.bytes[byte_range]
    }

    fn feature_bytes_mut(&mut self, feature_id: InstanceFeatureID) -> &mut [u8] {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        let feature_idx = self.index_map.idx(feature_id.idx());
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
            idx: self.feature_id_count as u64,
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

    /// Returns the current number of valid features in the buffer.
    ///
    /// # Panics
    /// If the feature type the buffer stores is a zero-sized type.
    pub fn n_valid_features(&self) -> usize {
        assert_ne!(self.feature_size(), 0);
        self.n_valid_bytes() / self.feature_size()
    }

    /// Returns the range of valid feature indices with the given ID. Ranges are
    /// defined by calling [`Self::begin_range`]. The range spans from and
    /// including the first feature added after the `begin_range` call to and
    /// including the last feature added before the next `begin_range` call, or
    /// to the last valid feature if the `begin_range` call was the last one.
    /// Calling [`Self::clear`] removes all range information.
    ///
    /// # Panics
    /// If no range with the given ID exists.
    pub fn valid_feature_range(&self, range_id: InstanceFeatureBufferRangeID) -> Range<u32> {
        self.range_manager
            .get_range(range_id, || self.n_valid_features())
    }

    /// Returns the range of valid feature indices encompassing all features
    /// added before defining any explicit ranges with [`Self::begin_range`].
    pub fn initial_valid_feature_range(&self) -> Range<u32> {
        self.valid_feature_range(InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID)
    }

    /// Creates an [`InstanceFeatureBufferRangeMap`] containing the information
    /// describing the ranges that have been defined with [`Self::begin_range`].
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

    /// Returns the range with the given ID and a slice with the features in
    /// that range. Ranges are defined by calling [`Self::begin_range`]. The
    /// range spans from and including the first feature added after the
    /// `begin_range` call to and including the last feature added before the
    /// next `begin_range` call, or to the last valid feature if the
    /// `begin_range` call was the last one. Calling [`Self::clear`] removes all
    /// range information.
    ///
    /// # Panics
    /// - If no range with the given ID exists.
    /// - If `Fe` is not the feature type the buffer was initialized with.
    /// - If `Fe` is a zero-sized type.
    pub fn range_with_valid_features<Fe: InstanceFeature>(
        &self,
        range_id: InstanceFeatureBufferRangeID,
    ) -> (Range<u32>, &[Fe]) {
        let range = self.valid_feature_range(range_id);
        let features = &self.valid_features()[range.start as usize..range.end as usize];
        (range, features)
    }

    /// Returns a slice with the currently valid features added before defining
    /// any explicit ranges with [`Self::begin_range`]
    ///
    /// # Panics
    /// - If `Fe` is not the feature type the buffer was initialized with.
    /// - If `Fe` is a zero-sized type.
    pub fn valid_features_in_initial_range<Fe: InstanceFeature>(&self) -> &[Fe] {
        self.range_with_valid_features(InstanceFeatureBufferRangeManager::INITIAL_RANGE_ID)
            .1
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
    /// next `begin_range` call will be considered part of this new range.
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

    /// Returns the layout of the vertex GPU buffer that can be used for the
    /// stored features.
    pub(crate) fn vertex_buffer_layout(&self) -> Option<wgpu::VertexBufferLayout<'static>> {
        self.vertex_buffer_layout.clone()
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
    /// ID of the initial range created when calling
    /// [`Self::new_with_initial_range`].
    pub const INITIAL_RANGE_ID: InstanceFeatureBufferRangeID = InstanceFeatureBufferRangeID::MAX;

    /// Creates a new `InstanceFeatureBufferRangeManager` with a single range
    /// starting at index 0. The ID of the initial range is available in the
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct ModelID(u32);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
    struct Feature(u8);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct DifferentFeature(f64);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
    struct ZeroSizedFeature;

    impl_InstanceFeature!(Feature);
    impl_InstanceFeature!(DifferentFeature);
    impl_InstanceFeature!(ZeroSizedFeature);

    type TestInstanceFeatureManager = InstanceFeatureManager<ModelID>;

    mod manager {
        use super::*;

        #[test]
        fn creating_instance_feature_manager_works() {
            let manager = TestInstanceFeatureManager::new();
            assert!(manager.model_ids_and_instance_buffers().next().is_none());
        }

        #[test]
        fn registering_feature_types_works() {
            let mut manager = TestInstanceFeatureManager::new();

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
        fn registering_one_instance_of_one_model_with_no_features_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[]);

            let mut models_and_buffers = manager.model_ids_and_instance_buffers();
            let (registered_model_id, buffer) = models_and_buffers.next().unwrap();
            assert_eq!(registered_model_id, &model_id);
            assert!(buffer.n_feature_types() == 0);

            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(&model_id));
            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert!(buffer.n_feature_types() == 0);
        }

        #[test]
        #[should_panic]
        fn registering_instance_with_unregistered_features_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[Feature::FEATURE_TYPE_ID]);
        }

        #[test]
        fn registering_one_instance_of_one_model_with_features_works() {
            let mut manager = TestInstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<ZeroSizedFeature>();

            let model_id = ModelID(0);

            manager.register_instance(
                model_id,
                &[ZeroSizedFeature::FEATURE_TYPE_ID, Feature::FEATURE_TYPE_ID],
            );

            assert!(manager.has_model_id(&model_id));
            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let zero_sized_feature_buffer = buffer
                .get_feature_buffer(ZeroSizedFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(
                zero_sized_feature_buffer.feature_type_id(),
                ZeroSizedFeature::FEATURE_TYPE_ID
            );
            assert_eq!(zero_sized_feature_buffer.n_valid_bytes(), 0);

            let feature_buffer = buffer.get_feature_buffer(Feature::FEATURE_TYPE_ID).unwrap();
            assert_eq!(feature_buffer.feature_type_id(), Feature::FEATURE_TYPE_ID);
            assert_eq!(feature_buffer.n_valid_bytes(), 0);
        }

        #[test]
        fn registering_one_instance_of_two_models_with_no_features_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id_1 = ModelID(1);
            let model_id_2 = ModelID(2);
            manager.register_instance(model_id_1, &[]);
            manager.register_instance(model_id_2, &[]);

            let mut models_and_buffers = manager.model_ids_and_instance_buffers();
            assert_ne!(
                models_and_buffers.next().unwrap().0,
                models_and_buffers.next().unwrap().0
            );
            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(&model_id_1));
            assert!(manager.has_model_id(&model_id_2));
            assert_eq!(
                manager
                    .get_model_instance_buffer(&model_id_1)
                    .unwrap()
                    .n_feature_types(),
                0
            );
            assert_eq!(
                manager
                    .get_model_instance_buffer(&model_id_2)
                    .unwrap()
                    .n_feature_types(),
                0
            );
        }

        #[test]
        fn registering_two_instances_of_one_model_with_no_features_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[]);
            manager.register_instance(model_id, &[]);

            let mut models_and_buffers = manager.model_ids_and_instance_buffers();
            assert_eq!(models_and_buffers.next().unwrap().0, &model_id);
            assert!(models_and_buffers.next().is_none());
            drop(models_and_buffers);

            assert!(manager.has_model_id(&model_id));
            assert_eq!(
                manager
                    .get_model_instance_buffer(&model_id)
                    .unwrap()
                    .n_feature_types(),
                0
            );
        }

        #[test]
        fn registering_and_then_unregistering_one_instance_of_model_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);

            manager.register_instance(model_id, &[]);
            manager.unregister_instance(&model_id);

            assert!(manager.model_ids_and_instance_buffers().next().is_none());
            assert!(!manager.has_model_id(&model_id));
            assert!(manager.get_model_instance_buffer(&model_id).is_none());

            manager.register_instance(model_id, &[]);
            manager.unregister_instance(&model_id);

            assert!(manager.model_ids_and_instance_buffers().next().is_none());
            assert!(!manager.has_model_id(&model_id));
            assert!(manager.get_model_instance_buffer(&model_id).is_none());
        }

        #[test]
        fn registering_and_then_unregistering_two_instances_of_model_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[]);
            manager.register_instance(model_id, &[]);
            manager.unregister_instance(&model_id);
            manager.unregister_instance(&model_id);

            assert!(manager.model_ids_and_instance_buffers().next().is_none());
            assert!(!manager.has_model_id(&model_id));
            assert!(manager.get_model_instance_buffer(&model_id).is_none());
        }

        #[test]
        #[should_panic]
        fn unregistering_instance_in_empty_instance_feature_manager_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.unregister_instance(&model_id);
        }

        #[test]
        #[should_panic]
        fn buffering_unregistered_features_from_storages_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.buffer_instance_features_from_storages(&model_id, &[]);
        }

        #[test]
        #[should_panic]
        fn buffering_unregistered_feature_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.buffer_instance_feature(&model_id, &Feature(42));
        }

        #[test]
        fn buffering_features_from_storages_for_model_with_no_features_works() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[]);
            manager.buffer_instance_features_from_storages(&model_id, &[]);
        }

        #[test]
        fn buffering_features_from_storages_for_model_with_multiple_features_works() {
            let mut manager = TestInstanceFeatureManager::new();

            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<DifferentFeature>();

            let model_id = ModelID(0);
            manager.register_instance(
                model_id,
                &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
            );

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

            manager.buffer_instance_features_from_storages(
                &model_id,
                &[id_1_instance_1, id_2_instance_1],
            );

            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let feature_1_buffer = buffer.get_feature_buffer(Feature::FEATURE_TYPE_ID).unwrap();
            assert_eq!(feature_1_buffer.n_valid_features(), 1);
            assert_eq!(
                feature_1_buffer.valid_features::<Feature>(),
                &[feature_1_instance_1]
            );

            let feature_2_buffer = buffer
                .get_feature_buffer(DifferentFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(feature_2_buffer.n_valid_features(), 1);
            assert_eq!(
                feature_2_buffer.valid_features::<DifferentFeature>(),
                &[feature_2_instance_1]
            );

            // Buffering separately should also work
            manager.buffer_instance_features_from_storages(&model_id, &[id_2_instance_2]);
            manager.buffer_instance_features_from_storages(&model_id, &[id_1_instance_2]);

            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let feature_1_buffer = buffer.get_feature_buffer(Feature::FEATURE_TYPE_ID).unwrap();
            assert_eq!(feature_1_buffer.n_valid_features(), 2);
            assert_eq!(
                feature_1_buffer.valid_features::<Feature>(),
                &[feature_1_instance_1, feature_1_instance_2]
            );

            let feature_2_buffer = buffer
                .get_feature_buffer(DifferentFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(feature_2_buffer.n_valid_features(), 2);
            assert_eq!(
                feature_2_buffer.valid_features::<DifferentFeature>(),
                &[feature_2_instance_1, feature_2_instance_2]
            );
        }

        #[test]
        #[should_panic]
        fn buffering_too_many_feature_types_from_storages_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            let model_id = ModelID(0);
            manager.register_instance(model_id, &[]);

            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let id = storage.add_feature(&Feature(33));

            manager.buffer_instance_features_from_storages(&model_id, &[id]);
        }

        #[test]
        #[should_panic]
        fn buffering_feature_with_invalid_feature_id_from_storages_fails() {
            let mut manager = TestInstanceFeatureManager::new();
            manager.register_feature_type::<Feature>();

            let model_id = ModelID(0);
            manager.register_instance(model_id, &[Feature::FEATURE_TYPE_ID]);

            let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
            let id = storage.add_feature(&DifferentFeature(-0.2));

            manager.buffer_instance_features_from_storages(&model_id, &[id]);
        }

        #[test]
        fn buffering_feature_directly_works() {
            let mut manager = TestInstanceFeatureManager::new();

            manager.register_feature_type::<Feature>();
            manager.register_feature_type::<DifferentFeature>();

            let model_id = ModelID(0);
            manager.register_instance(
                model_id,
                &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
            );

            let feature_1_instance_1 = Feature(22);
            let feature_1_instance_2 = Feature(43);
            let feature_2 = DifferentFeature(-73.1);

            manager.buffer_instance_feature(&model_id, &feature_1_instance_1);

            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let feature_1_buffer = buffer.get_feature_buffer(Feature::FEATURE_TYPE_ID).unwrap();
            assert_eq!(feature_1_buffer.n_valid_features(), 1);
            assert_eq!(
                feature_1_buffer.valid_features::<Feature>(),
                &[feature_1_instance_1]
            );

            let feature_2_buffer = buffer
                .get_feature_buffer(DifferentFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(feature_2_buffer.n_valid_features(), 0);

            manager.buffer_instance_feature(&model_id, &feature_1_instance_2);

            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let feature_1_buffer = buffer.get_feature_buffer(Feature::FEATURE_TYPE_ID).unwrap();
            assert_eq!(feature_1_buffer.n_valid_features(), 2);
            assert_eq!(
                feature_1_buffer.valid_features::<Feature>(),
                &[feature_1_instance_1, feature_1_instance_2]
            );

            let feature_2_buffer = buffer
                .get_feature_buffer(DifferentFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(feature_2_buffer.n_valid_features(), 0);

            manager.buffer_instance_feature(&model_id, &feature_2);

            let buffer = manager.get_model_instance_buffer(&model_id).unwrap();
            assert_eq!(buffer.n_feature_types(), 2);

            let feature_2_buffer = buffer
                .get_feature_buffer(DifferentFeature::FEATURE_TYPE_ID)
                .unwrap();
            assert_eq!(feature_2_buffer.n_valid_features(), 1);
            assert_eq!(
                feature_2_buffer.valid_features::<DifferentFeature>(),
                &[feature_2]
            );
        }
    }

    mod storage_and_buffer {
        use super::*;

        #[repr(transparent)]
        #[derive(Clone, Copy, Zeroable, Pod)]
        struct DifferentFeature(u8);

        #[repr(transparent)]
        #[derive(Clone, Copy, Zeroable, Pod)]
        struct ZeroSizedFeature;

        impl_InstanceFeature!(DifferentFeature);
        impl_InstanceFeature!(ZeroSizedFeature);

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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);

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
            let feature = Feature(42);
            storage.add_feature(&feature);
        }

        #[test]
        #[should_panic]
        fn checking_existence_of_feature_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = Feature(42);
            let id_1 = storage_1.add_feature(&feature_1);
            storage_2.has_feature(id_1);
        }

        #[test]
        #[should_panic]
        fn retrieving_feature_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = Feature(42);
            let id_1 = storage_1.add_feature(&feature_1);
            storage_2.feature::<Feature>(id_1);
        }

        #[test]
        #[should_panic]
        fn retrieving_feature_mutably_with_invalid_id_in_instance_feature_storage_fails() {
            let mut storage_1 = InstanceFeatureStorage::new::<Feature>();
            let mut storage_2 = InstanceFeatureStorage::new::<DifferentFeature>();
            let feature_1 = Feature(42);
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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);

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
            let feature = Feature(42);
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
            let feature_1 = Feature(42);

            let id_1 = storage.add_feature(&feature_1);

            storage.remove_all_features();

            assert_eq!(storage.feature_count(), 0);
            assert!(!storage.has_feature(id_1));
        }

        #[test]
        fn removing_all_features_from_multi_instance_feature_storage_works() {
            let mut storage = InstanceFeatureStorage::new::<Feature>();
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);

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
            let feature = Feature(42);
            buffer.add_feature(&feature);

            assert_eq!(buffer.n_valid_bytes(), mem::size_of::<Feature>());
            assert_eq!(buffer.n_valid_features(), 1);
            assert_eq!(buffer.valid_bytes(), bytemuck::bytes_of(&feature));
            assert_eq!(buffer.valid_features::<Feature>(), &[feature]);
        }

        #[test]
        fn adding_two_features_to_instance_feature_buffer_works() {
            let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
            let feature_3 = Feature(44);

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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
            let feature_3 = Feature(44);

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
            let features = vec![Feature(42), Feature(43), Feature(44)];

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
            let feature = Feature(42);
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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
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
            let feature = Feature(42);
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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);

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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);

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
            let feature_1 = Feature(42);
            let feature_2 = Feature(43);
            let feature_3 = Feature(44);

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
            let feature = Feature(42);

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
            let feature = Feature(42);

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
            let feature = Feature(42);

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
            let feature = Feature(42);

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
