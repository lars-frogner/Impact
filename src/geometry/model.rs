//! Model instances.

use crate::num::Float;
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;
use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A buffer for transforms associated with instances
/// of the same model.
///
/// The buffer is grown on demand, but never shrunk.
/// Instead, a counter keeps track of the position
/// of the last valid transform in the buffer, and the
/// counter is reset to zero when the buffer is cleared.
/// This allows the it to be filled and emptied
/// repeatedly without unneccesary allocations.
#[derive(Debug)]
pub struct ModelInstanceTransformBuffer<F: Float> {
    raw_buffer: Vec<ModelInstanceTransform<F>>,
    n_valid_transforms: AtomicUsize,
}

/// Wrapper around a [`ModelInstanceTransformBuffer`] that enables
/// counting the number of uses of the buffer.
#[derive(Debug)]
pub struct UserCountingModelInstanceTransformBuffer<F: Float> {
    user_count: u64,
    buffer: ModelInstanceTransformBuffer<F>,
}

/// A model-to-camera transform for a specific instance of a model.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelInstanceTransform<F: Float> {
    transform_matrix: Matrix4<F>,
}

impl<F: Float> ModelInstanceTransformBuffer<F> {
    /// Creates a new empty buffer for model instance transforms.
    pub fn new() -> Self {
        Self {
            raw_buffer: Vec::new(),
            n_valid_transforms: AtomicUsize::new(0),
        }
    }

    /// Returns the current number of valid transforms in the buffer.
    pub fn n_valid_transforms(&self) -> usize {
        self.n_valid_transforms.load(Ordering::Acquire)
    }

    /// Returns a slice with all the transforms in the buffer,
    /// including invalid ones.
    ///
    /// # Warning
    /// Only the elements below
    /// [`n_valid_transforms`](Self::n_valid_transforms) are
    /// considered to have valid values.
    pub fn raw_buffer(&self) -> &[ModelInstanceTransform<F>] {
        &self.raw_buffer
    }

    /// Returns a slice with the valid transforms in the buffer.
    pub fn valid_transforms(&self) -> &[ModelInstanceTransform<F>] {
        &self.raw_buffer[0..self.n_valid_transforms()]
    }

    /// Inserts the given transform into the buffer.
    pub fn add_transform(&mut self, transform: ModelInstanceTransform<F>) {
        let buffer_length = self.raw_buffer.len();
        let idx = self.n_valid_transforms.fetch_add(1, Ordering::SeqCst);
        assert!(idx <= buffer_length);

        // If the buffer is full, grow it first
        if idx == buffer_length {
            self.grow_buffer();
        }

        self.raw_buffer[idx] = transform;
    }

    /// Empties the buffer of transforms.
    ///
    /// Does not actually drop anything, just resets the count of
    /// valid transforms to zero.
    pub fn clear(&self) {
        self.n_valid_transforms.store(0, Ordering::Release);
    }

    fn grow_buffer(&mut self) {
        let old_buffer_length = self.raw_buffer.len();

        // Add one before doubling to avoid getting stuck at zero
        let new_buffer_length = (old_buffer_length + 1).checked_mul(2).unwrap();

        let mut new_buffer = vec![ModelInstanceTransform::identity(); new_buffer_length];
        new_buffer[0..old_buffer_length].copy_from_slice(&self.raw_buffer);

        self.raw_buffer = new_buffer;
    }
}

impl<F: Float> Default for ModelInstanceTransformBuffer<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Float> UserCountingModelInstanceTransformBuffer<F> {
    /// Creates a new model instance transform buffer with a user count
    /// of one.
    pub fn new() -> Self {
        Self {
            user_count: 1,
            buffer: ModelInstanceTransformBuffer::new(),
        }
    }

    /// Returns a reference to the wrapped model instance transform buffer.
    pub fn inner(&self) -> &ModelInstanceTransformBuffer<F> {
        &self.buffer
    }

    /// Returns a mutable reference to the wrapped model instance
    /// transform buffer.
    pub fn inner_mut(&mut self) -> &mut ModelInstanceTransformBuffer<F> {
        &mut self.buffer
    }

    /// Whether the user count is zero.
    pub fn no_users(&self) -> bool {
        self.user_count == 0
    }

    /// Increments the user count by one.
    pub fn increment_user_count(&mut self) {
        self.user_count += 1;
    }

    /// Devrements the user count by one.
    ///
    /// # Panics
    /// If the user count before decrementing is zero.
    pub fn decrement_user_count(&mut self) {
        assert!(self.user_count >= 1);
        self.user_count -= 1;
    }
}

impl<F: Float> Default for UserCountingModelInstanceTransformBuffer<F> {
    fn default() -> Self {
        Self::new()
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

// Since `MeshInstance` is `#[repr(transparent)]`, it will be
// `Zeroable` and `Pod` as long as its field, `Matrix4`, is so.
unsafe impl<F: Float> Zeroable for ModelInstanceTransform<F> where Matrix4<F>: Zeroable {}

unsafe impl<F> Pod for ModelInstanceTransform<F>
where
    F: Float,
    Matrix4<F>: Pod,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};

    fn create_dummy_instance() -> ModelInstanceTransform<f32> {
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
    fn creating_model_instance_transform_buffer_works() {
        let buffer = ModelInstanceTransformBuffer::<f32>::new();
        assert_eq!(buffer.n_valid_transforms(), 0);
        assert_eq!(buffer.valid_transforms(), &[]);
    }

    #[test]
    fn adding_one_instance_to_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform = create_dummy_instance();
        buffer.add_transform(transform);
        assert_eq!(buffer.n_valid_transforms(), 1);
        assert_eq!(buffer.valid_transforms(), &[transform]);
    }

    #[test]
    fn adding_two_instances_to_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform_1 = ModelInstanceTransform::identity();
        let transform_2 = create_dummy_instance();
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        assert_eq!(buffer.n_valid_transforms(), 2);
        assert_eq!(buffer.valid_transforms(), &[transform_1, transform_2]);
    }

    #[test]
    fn adding_three_instances_to_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform_1 = create_dummy_instance();
        let transform_2 = ModelInstanceTransform::identity();
        let transform_3 = create_dummy_instance();
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        buffer.add_transform(transform_3);
        assert_eq!(buffer.n_valid_transforms(), 3);
        assert_eq!(
            buffer.valid_transforms(),
            &[transform_1, transform_2, transform_3]
        );
    }

    #[test]
    fn clearing_empty_model_instance_transform_buffer_works() {
        let buffer = ModelInstanceTransformBuffer::<f32>::new();
        buffer.clear();
        assert_eq!(buffer.n_valid_transforms(), 0);
        assert_eq!(buffer.valid_transforms(), &[]);
    }

    #[test]
    fn clearing_one_instance_from_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform_1 = ModelInstanceTransform::identity();
        let transform_2 = create_dummy_instance();
        buffer.add_transform(transform_1);
        buffer.clear();
        assert_eq!(buffer.n_valid_transforms(), 0);
        assert_eq!(buffer.valid_transforms(), &[]);
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        assert_eq!(buffer.n_valid_transforms(), 2);
        assert_eq!(buffer.valid_transforms(), &[transform_1, transform_2]);
    }

    #[test]
    fn clearing_two_instances_from_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform_1 = ModelInstanceTransform::identity();
        let transform_2 = create_dummy_instance();
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        buffer.clear();
        assert_eq!(buffer.n_valid_transforms(), 0);
        assert_eq!(buffer.valid_transforms(), &[]);
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        assert_eq!(buffer.n_valid_transforms(), 2);
        assert_eq!(buffer.valid_transforms(), &[transform_1, transform_2]);
    }

    #[test]
    fn clearing_three_instances_from_model_instance_transform_buffer_works() {
        let mut buffer = ModelInstanceTransformBuffer::<f32>::new();
        let transform_1 = create_dummy_instance();
        let transform_2 = ModelInstanceTransform::identity();
        let transform_3 = create_dummy_instance();
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        buffer.add_transform(transform_3);
        buffer.clear();
        assert_eq!(buffer.n_valid_transforms(), 0);
        assert_eq!(buffer.valid_transforms(), &[]);
        buffer.add_transform(transform_1);
        buffer.add_transform(transform_2);
        buffer.add_transform(transform_3);
        assert_eq!(buffer.n_valid_transforms(), 3);
        assert_eq!(
            buffer.valid_transforms(),
            &[transform_1, transform_2, transform_3]
        );
    }
}
