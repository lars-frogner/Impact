//! Model instances.

use crate::{
    impl_InstanceFeature,
    num::Float,
    rendering::{fre, InstanceFeatureShaderInput, ModelInstanceTransformShaderInput},
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{AlignedByteVec, Alignment, Hash64, KeyIndexMapper};
use nalgebra::Matrix4;
use std::{
    fmt::Debug,
    mem,
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};

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
    index_map: KeyIndexMapper<usize>,
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
    n_valid_bytes: AtomicUsize,
}

/// A model-to-camera transform for a specific instance of a model.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelInstanceTransform<F: Float> {
    transform_matrix: Matrix4<F>,
}

#[derive(Copy, Clone, Debug)]
struct InstanceFeatureTypeDescriptor {
    id: InstanceFeatureTypeID,
    size: usize,
    alignment: Alignment,
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
            index_map: KeyIndexMapper::new(),
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
            index_map: KeyIndexMapper::new(),
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
    /// Creates a new empty buffer for features of type `Fe`.
    pub fn new<Fe: InstanceFeature>() -> Self {
        Self {
            type_descriptor: InstanceFeatureTypeDescriptor::for_type::<Fe>(),
            vertex_buffer_layout: Fe::BUFFER_LAYOUT,
            shader_input: Fe::SHADER_INPUT,
            bytes: AlignedByteVec::new(Fe::FEATURE_ALIGNMENT),
            n_valid_bytes: AtomicUsize::new(0),
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
            bytes: AlignedByteVec::new(type_descriptor.alignment()),
            n_valid_bytes: AtomicUsize::new(0),
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

    /// Returns the number of bytes from the beginning of the buffer
    /// that are currently valid.
    pub fn n_valid_bytes(&self) -> usize {
        self.n_valid_bytes.load(Ordering::Acquire)
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

    /// Empties the buffer.
    ///
    /// Does not actually drop anything, just resets the count of
    /// valid bytes to zero.
    pub fn clear(&self) {
        self.n_valid_bytes.store(0, Ordering::Release);
    }

    fn add_feature_bytes(&mut self, feature_bytes: &[u8]) {
        let feature_size = self.feature_size();
        assert_eq!(feature_bytes.len(), feature_size);

        if feature_size > 0 {
            let start_byte_idx = self.n_valid_bytes.fetch_add(feature_size, Ordering::SeqCst);
            let end_byte_idx = start_byte_idx + feature_size;

            // If the buffer is full, grow it first
            if end_byte_idx >= self.bytes.len() {
                self.grow_buffer(end_byte_idx);
            }

            self.bytes[start_byte_idx..end_byte_idx]
                .iter_mut()
                .zip(feature_bytes.iter())
                .for_each(|(dest, src)| {
                    *dest = *src;
                });
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

impl<F: Float> ModelInstanceTransform<F> {
    /// Creates a new identity model-to-camera transform.
    pub fn identity() -> Self {
        Self::with_model_to_camera_transform(Matrix4::identity())
    }

    /// Creates a new model-to-camera transform with the given
    /// transform matrix.
    pub fn with_model_to_camera_transform(transform_matrix: Matrix4<F>) -> Self {
        Self { transform_matrix }
    }

    /// Returns the matrix for the model-to-camera transform.
    pub fn transform_matrix(&self) -> &Matrix4<F> {
        &self.transform_matrix
    }
}

impl<F: Float> Default for ModelInstanceTransform<F> {
    fn default() -> Self {
        Self::identity()
    }
}

// Since `ModelInstanceTransform` is `#[repr(transparent)]`, it will be
// `Zeroable` and `Pod` as long as its field, `Matrix4`, is so.
unsafe impl<F: Float> Zeroable for ModelInstanceTransform<F> where Matrix4<F>: Zeroable {}

unsafe impl<F> Pod for ModelInstanceTransform<F>
where
    F: Float,
    Matrix4<F>: Pod,
{
}

impl_InstanceFeature!(
    ModelInstanceTransform<fre>,
    wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
    InstanceFeatureShaderInput::ModelInstanceTransform(ModelInstanceTransformShaderInput {
        model_matrix_locations: (5, 6, 7, 8),
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
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};

    type Feature = ModelInstanceTransform<f32>;

    #[repr(transparent)]
    #[derive(Clone, Copy, Zeroable, Pod)]
    struct DifferentFeature(u8);

    #[repr(transparent)]
    #[derive(Clone, Copy, Zeroable, Pod)]
    struct ZeroSizedFeature;

    impl_InstanceFeature!(DifferentFeature, [], InstanceFeatureShaderInput::None);
    impl_InstanceFeature!(ZeroSizedFeature, [], InstanceFeatureShaderInput::None);

    fn create_dummy_feature() -> ModelInstanceTransform<f32> {
        ModelInstanceTransform::with_model_to_camera_transform(
            Similarity3::from_parts(
                Translation3::new(2.1, -5.9, 0.01),
                UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
                7.0,
            )
            .to_homogeneous(),
        )
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
        let feature_2 = ModelInstanceTransform::<f32>::identity();

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
        let feature_2 = ModelInstanceTransform::<f32>::identity();

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
        let feature_1 = ModelInstanceTransform::identity();
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
        let feature_2 = ModelInstanceTransform::identity();
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
    fn clearing_empty_instance_feature_buffer_works() {
        let buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
        buffer.clear();

        assert_eq!(buffer.n_valid_bytes(), 0);
        assert_eq!(buffer.n_valid_features(), 0);
        assert_eq!(buffer.valid_bytes(), &[]);
        assert_eq!(buffer.valid_features::<Feature>(), &[]);
    }

    #[test]
    fn clearing_one_feature_from_instance_feature_buffer_works() {
        let mut buffer = DynamicInstanceFeatureBuffer::new::<Feature>();
        let feature_1 = ModelInstanceTransform::identity();
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
        let feature_1 = ModelInstanceTransform::identity();
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
        let feature_2 = ModelInstanceTransform::identity();
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
}
