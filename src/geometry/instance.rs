//! Model instances.

use crate::{
    impl_InstanceFeature,
    num::Float,
    rendering::{fre, InstanceFeatureShaderInput, ModelViewTransformShaderInput},
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_utils::{AlignedByteVec, Alignment, Hash64, KeyIndexMapper};
use nalgebra::{Similarity3, UnitQuaternion, Vector3};
use nohash_hasher::BuildNoHashHasher;
use simba::scalar::{SubsetOf, SupersetOf};
use std::{collections::HashMap, fmt::Debug, mem, ops::Range};

/// Represents a piece of data associated with a model instance.
pub trait InstanceFeature: Pod {
    /// A unique ID representing the feature type.
    const FEATURE_TYPE_ID: InstanceFeatureTypeID;

    /// The size of the feature type in bytes.
    const FEATURE_SIZE: usize = mem::size_of::<Self>();

    /// The memory alignment of the feature type.
    const FEATURE_ALIGNMENT: Alignment = Alignment::of::<Self>();

    /// The layout of the vertex render buffer that
    /// can be used to pass the feature to the GPU.
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static>;

    /// The input required for a shader to access this
    /// feature.
    const SHADER_INPUT: InstanceFeatureShaderInput;

    /// Returns a slice with the raw bytes representing the
    /// feature.
    fn feature_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Returns a slice with the raw bytes representing the
    /// given slice of features.
    fn feature_slice_bytes(slice: &[Self]) -> &[u8] {
        bytemuck::cast_slice(slice)
    }
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
/// The storage is designed for efficient insertion of, access
/// to and removal of individual feature values.
///
/// Stores the raw bytes of the features to avoid exposing
/// the feature type signature. The type information is
/// extracted on construction and used to validate access
/// requests.
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
/// The buffer is grown on demand, but never shrunk.
/// Instead, a counter keeps track of the position
/// of the last valid byte in the buffer, and the
/// counter is reset to zero when the buffer is cleared.
/// This allows the it to be filled and emptied
/// repeatedly without unneccesary allocations.
///
/// Stores the raw bytes of the features to avoid exposing
/// the feature type signature. The type information is
/// extracted on construction and used to validate access
/// requests.
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

/// A transform from the space of an instance in a cluster to the space of the
/// whole cluster.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterInstanceTransform<F: Float> {
    translation: Vector3<F>,
    scaling: F,
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

    /// Creates a new empty storage with preallocated capacity for the
    /// given number of features of type `Fe`.
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

    /// Returns the size in bytes of the type of feature this storage
    /// can store.
    pub fn feature_size(&self) -> usize {
        self.type_descriptor.size()
    }

    /// Returns the layout of the vertex render buffer that can be used
    /// for the stored features.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the input required for accessing the features in a
    /// shader.
    pub fn shader_input(&self) -> &InstanceFeatureShaderInput {
        &self.shader_input
    }

    /// Returns the number of stored features.
    pub fn feature_count(&self) -> usize {
        self.index_map.len()
    }

    /// Whether a feature with the given identifier exists in the
    /// storage.
    ///
    /// # Panics
    /// If the given feature ID was issued from a storage for a different
    /// feature type.
    pub fn has_feature(&self, feature_id: InstanceFeatureID) -> bool {
        self.type_descriptor
            .validate_feature_type_id(feature_id.feature_type_id);
        self.index_map.contains_key(feature_id.idx)
    }

    /// Returns a reference to the value of the feature stored under the
    /// given identifier.
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

    /// Returns a mutable reference to the value of the feature stored under
    /// the given identifier.
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

            // Copy over with last feature unless the feature to
            // remove is the last one
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
    /// constructing empty render buffers when synchronizing this buffer with
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

    /// Creates a new empty buffer for the same type of features
    /// as stored in the given storage.
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

    /// Returns the size in bytes of the type of feature this buffer
    /// can store.
    pub fn feature_size(&self) -> usize {
        self.type_descriptor.size()
    }

    /// Returns the layout of the vertex render buffer that can be used
    /// for the stored features.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the input required for accessing the features in a
    /// shader.
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

    /// Returns the number of bytes from the beginning of the buffer
    /// that are currently valid.
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

        // Make sure not to call `cast_slice` on an empty slice, as
        // an empty slice is not guaranteed to have the correct alignment
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

    /// Returns a slice with all the bytes in the buffer, including
    /// currently invalid ones.
    ///
    /// # Warning
    /// Only the bytes below [`n_valid_bytes`](Self::n_valid_bytes)
    /// are considered to have valid values.
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

    /// Pushes a copy of the feature value stored in the given
    /// storage under the given identifier onto the buffer.
    ///
    /// # Panics
    /// - If the feature types of the storage and buffer are not the
    ///   same.
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
    /// Creates a new model-to-light transform corresponding
    /// to the given similarity transform.
    pub fn with_model_light_transform(transform: Similarity3<fre>) -> Self {
        Self::with_model_view_transform(transform)
    }
}

impl Default for InstanceModelViewTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl<F: Float> ClusterInstanceTransform<F> {
    /// Creates a new transform with the given translation and scaling.
    pub fn new(translation: Vector3<F>, scaling: F) -> Self {
        Self {
            translation,
            scaling,
        }
    }

    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Self {
            translation: Vector3::zeros(),
            scaling: F::ONE,
        }
    }

    /// Returns a reference to the translational part of the transform.
    pub fn translation(&self) -> &Vector3<F> {
        &self.translation
    }

    /// Returns the scaling part of the transform.
    pub fn scaling(&self) -> F {
        self.scaling
    }

    /// Applies the given transform from the space of the cluster to camera
    /// space, yielding the model view transform of the instance.
    pub fn transform_into_model_view_transform(
        &self,
        transform_from_cluster_to_camera_space: &Similarity3<F>,
    ) -> InstanceModelViewTransform
    where
        F: SubsetOf<fre>,
    {
        let scaling_from_cluster_to_camera_space = transform_from_cluster_to_camera_space.scaling();
        let rotation_from_cluster_to_camera_space =
            transform_from_cluster_to_camera_space.isometry.rotation;
        let translation_from_cluster_to_camera_space = transform_from_cluster_to_camera_space
            .isometry
            .translation
            .vector;

        let new_scaling = scaling_from_cluster_to_camera_space * self.scaling;

        let new_translation = translation_from_cluster_to_camera_space
            + rotation_from_cluster_to_camera_space.transform_vector(&self.translation)
                * scaling_from_cluster_to_camera_space;

        InstanceModelViewTransform {
            rotation: rotation_from_cluster_to_camera_space.cast::<fre>(),
            translation: new_translation.cast::<fre>(),
            scaling: fre::from_subset(&new_scaling),
        }
    }
}

impl<F: Float> Default for ClusterInstanceTransform<F> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<F> AbsDiffEq for ClusterInstanceTransform<F>
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

/// Convenience macro for implementing the [`InstanceFeature`] trait.
/// The feature type ID is created by hashing the name of the
/// implementing type.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_InstanceFeature {
    ($ty:ty, $vertex_attr_array:expr, $shader_input:expr) => {
        impl $crate::geometry::InstanceFeature for $ty {
            const FEATURE_TYPE_ID: $crate::geometry::InstanceFeatureTypeID =
                impact_utils::ConstStringHash64::new(stringify!($ty)).into_hash();

            const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
                $crate::rendering::create_vertex_buffer_layout_for_instance::<Self>(
                    &$vertex_attr_array,
                );

            const SHADER_INPUT: InstanceFeatureShaderInput = $shader_input;
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::{vector, Similarity3, Translation3, UnitQuaternion};

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
        assert_eq!(buffer.valid_bytes(), &[]);
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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        assert_eq!(buffer.valid_bytes(), &[]);
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
        assert_eq!(buffer.valid_bytes(), &[]);
        assert_eq!(buffer.valid_features::<Feature>(), &[]);

        buffer.add_feature(&feature_1);
        buffer.add_feature(&feature_2);

        let feature_slice = &[feature_1, feature_2];
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        assert_eq!(buffer.valid_bytes(), &[]);
        assert_eq!(buffer.valid_features::<Feature>(), &[]);

        buffer.add_feature(&feature_1);
        buffer.add_feature(&feature_2);

        let feature_slice = &[feature_1, feature_2];
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        assert_eq!(buffer.valid_bytes(), &[]);
        assert_eq!(buffer.valid_features::<Feature>(), &[]);

        buffer.add_feature(&feature_1);
        buffer.add_feature(&feature_2);
        buffer.add_feature(&feature_3);

        let feature_slice = &[feature_1, feature_2, feature_3];
        let feature_bytes = bytemuck::cast_slice(feature_slice);

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
        assert_eq!(buffer.valid_bytes(), &[]);
    }

    #[test]
    fn adding_zero_sized_features_to_instance_feature_buffer_works() {
        let mut buffer = DynamicInstanceFeatureBuffer::new::<ZeroSizedFeature>();

        buffer.add_feature(&ZeroSizedFeature);

        assert_eq!(buffer.n_valid_bytes(), 0);
        assert_eq!(buffer.valid_bytes(), &[]);

        buffer.add_feature(&ZeroSizedFeature);

        assert_eq!(buffer.n_valid_bytes(), 0);
        assert_eq!(buffer.valid_bytes(), &[]);

        buffer.clear();

        assert_eq!(buffer.n_valid_bytes(), 0);
        assert_eq!(buffer.valid_bytes(), &[]);
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

    #[test]
    fn transforming_cluster_instance_transform_works() {
        let translation = vector![0.1, -0.2, 0.3];
        let scaling = 0.8;

        let cluster_instance_transform = ClusterInstanceTransform::new(translation, scaling);

        let cluster_instance_similarity =
            Similarity3::from_parts(translation.into(), UnitQuaternion::identity(), scaling);

        let transform_from_cluster_to_camera_space = Similarity3::from_parts(
            vector![-1.2, 9.7, 0.4].into(),
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 1.1),
            2.7,
        );

        let model_view_transform = cluster_instance_transform
            .transform_into_model_view_transform(&transform_from_cluster_to_camera_space);

        let correct_model_view_transform =
            transform_from_cluster_to_camera_space * cluster_instance_similarity;

        assert_abs_diff_eq!(
            model_view_transform.translation,
            correct_model_view_transform.isometry.translation.vector
        );
        assert_abs_diff_eq!(
            model_view_transform.rotation,
            correct_model_view_transform.isometry.rotation
        );
        assert_abs_diff_eq!(
            model_view_transform.scaling,
            correct_model_view_transform.scaling()
        );
    }
}
