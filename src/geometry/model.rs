//! Model instances.

use crate::num::Float;
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;
use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A buffer for instances of the same model.
///
/// The buffer is grown on demand, but never shrunk.
/// Instead, a counter keeps track of the position
/// of the last valid instance in the buffer, and the
/// counter is reset to zero when the buffer is cleared.
/// This allows the it to be filled and emptied
/// repeatedly without unneccesary allocations.
#[derive(Debug)]
pub struct ModelInstanceBuffer<F: Float> {
    raw_buffer: Vec<ModelInstance<F>>,
    n_valid_instances: AtomicUsize,
}

/// Wrapper around a [`ModelInstanceBuffer`] that enables
/// counting the number of uses of the buffer.
#[derive(Debug)]
pub struct UserCountingModelInstanceBuffer<F: Float> {
    user_count: u64,
    buffer: ModelInstanceBuffer<F>,
}

/// An instance of a model with a specific model-to-camera
/// transform.
///
/// Used to represent multiple versions of the same basic model.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelInstance<F: Float> {
    transform_matrix: Matrix4<F>,
}

impl<F: Float> ModelInstanceBuffer<F> {
    /// Creates a new empty buffer for model instances.
    pub fn new() -> Self {
        Self {
            raw_buffer: Vec::new(),
            n_valid_instances: AtomicUsize::new(0),
        }
    }

    /// Returns the current number of valid instances in the buffer.
    pub fn n_valid_instances(&self) -> usize {
        self.n_valid_instances.load(Ordering::Acquire)
    }

    /// Returns a slice with all the instances in the buffer,
    /// including invalid ones.
    ///
    /// # Warning
    /// Only the elements below
    /// [`n_valid_instances`](Self::n_valid_instances) are
    /// considered to have valid values.
    pub fn raw_buffer(&self) -> &[ModelInstance<F>] {
        &self.raw_buffer
    }

    /// Returns a slice with the valid instances in the buffer.
    pub fn valid_instances(&self) -> &[ModelInstance<F>] {
        &self.raw_buffer[0..self.n_valid_instances()]
    }

    /// Inserts the given instance into the buffer.
    pub fn add_instance(&mut self, instance: ModelInstance<F>) {
        let buffer_length = self.raw_buffer.len();
        let idx = self.n_valid_instances.fetch_add(1, Ordering::SeqCst);
        assert!(idx <= buffer_length);

        // If the buffer is full, grow it first
        if idx == buffer_length {
            self.grow_buffer();
        }

        self.raw_buffer[idx] = instance;
    }

    /// Empties the buffer of instances.
    ///
    /// Does not actually drop anything, just resets the count of
    /// valid instances to zero.
    pub fn clear(&self) {
        self.n_valid_instances.store(0, Ordering::Release);
    }

    fn grow_buffer(&mut self) {
        let old_buffer_length = self.raw_buffer.len();

        // Add one before doubling to avoid getting stuck at zero
        let new_buffer_length = (old_buffer_length + 1).checked_mul(2).unwrap();

        let mut new_buffer = vec![ModelInstance::new(); new_buffer_length];
        new_buffer[0..old_buffer_length].copy_from_slice(&self.raw_buffer);

        self.raw_buffer = new_buffer;
    }
}

impl<F: Float> Default for ModelInstanceBuffer<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Float> UserCountingModelInstanceBuffer<F> {
    /// Creates a new model instance buffer with a user count
    /// of one.
    pub fn new() -> Self {
        Self {
            user_count: 1,
            buffer: ModelInstanceBuffer::new(),
        }
    }

    /// Returns a reference to the wrapped model instance buffer.
    pub fn inner(&self) -> &ModelInstanceBuffer<F> {
        &self.buffer
    }

    /// Returns a mutable reference to the wrapped model instance
    /// buffer.
    pub fn inner_mut(&mut self) -> &mut ModelInstanceBuffer<F> {
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

impl<F: Float> Default for UserCountingModelInstanceBuffer<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Float> ModelInstance<F> {
    /// Creates a new model instance with no model-to-camera transform.
    pub fn new() -> Self {
        Self::with_model_to_camera_transform(Matrix4::identity())
    }

    /// Creates a new model instance with the given model-to-camera transform.
    pub fn with_model_to_camera_transform(transform_matrix: Matrix4<F>) -> Self {
        Self { transform_matrix }
    }

    /// Returns the transform matrix describing the configuration of
    /// this model instance in relation to the default configuration of
    /// the model.
    pub fn transform_matrix(&self) -> &Matrix4<F> {
        &self.transform_matrix
    }
}

impl<F: Float> Default for ModelInstance<F> {
    fn default() -> Self {
        Self::new()
    }
}

// Since `MeshInstance` is `#[repr(transparent)]`, it will be
// `Zeroable` and `Pod` as long as its field, `Matrix4`, is so.
unsafe impl<F: Float> Zeroable for ModelInstance<F> where Matrix4<F>: Zeroable {}

unsafe impl<F> Pod for ModelInstance<F>
where
    F: Float,
    Matrix4<F>: Pod,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};

    fn create_dummy_instance() -> ModelInstance<f32> {
        ModelInstance::with_model_to_camera_transform(
            Similarity3::from_parts(
                Translation3::new(2.1, -5.9, 0.01),
                UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
                7.0,
            )
            .to_homogeneous(),
        )
    }

    #[test]
    fn creating_model_instance_buffer_works() {
        let buffer = ModelInstanceBuffer::<f32>::new();
        assert_eq!(buffer.n_valid_instances(), 0);
        assert_eq!(buffer.valid_instances(), &[]);
    }

    #[test]
    fn adding_one_instance_to_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance = create_dummy_instance();
        buffer.add_instance(instance);
        assert_eq!(buffer.n_valid_instances(), 1);
        assert_eq!(buffer.valid_instances(), &[instance]);
    }

    #[test]
    fn adding_two_instances_to_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance_1 = ModelInstance::new();
        let instance_2 = create_dummy_instance();
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        assert_eq!(buffer.n_valid_instances(), 2);
        assert_eq!(buffer.valid_instances(), &[instance_1, instance_2]);
    }

    #[test]
    fn adding_three_instances_to_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance_1 = create_dummy_instance();
        let instance_2 = ModelInstance::new();
        let instance_3 = create_dummy_instance();
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        buffer.add_instance(instance_3);
        assert_eq!(buffer.n_valid_instances(), 3);
        assert_eq!(
            buffer.valid_instances(),
            &[instance_1, instance_2, instance_3]
        );
    }

    #[test]
    fn clearing_empty_model_instance_buffer_works() {
        let buffer = ModelInstanceBuffer::<f32>::new();
        buffer.clear();
        assert_eq!(buffer.n_valid_instances(), 0);
        assert_eq!(buffer.valid_instances(), &[]);
    }

    #[test]
    fn clearing_one_instance_from_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance_1 = ModelInstance::new();
        let instance_2 = create_dummy_instance();
        buffer.add_instance(instance_1);
        buffer.clear();
        assert_eq!(buffer.n_valid_instances(), 0);
        assert_eq!(buffer.valid_instances(), &[]);
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        assert_eq!(buffer.n_valid_instances(), 2);
        assert_eq!(buffer.valid_instances(), &[instance_1, instance_2]);
    }

    #[test]
    fn clearing_two_instances_from_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance_1 = ModelInstance::new();
        let instance_2 = create_dummy_instance();
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        buffer.clear();
        assert_eq!(buffer.n_valid_instances(), 0);
        assert_eq!(buffer.valid_instances(), &[]);
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        assert_eq!(buffer.n_valid_instances(), 2);
        assert_eq!(buffer.valid_instances(), &[instance_1, instance_2]);
    }

    #[test]
    fn clearing_three_instances_from_model_instance_buffer_works() {
        let mut buffer = ModelInstanceBuffer::<f32>::new();
        let instance_1 = create_dummy_instance();
        let instance_2 = ModelInstance::new();
        let instance_3 = create_dummy_instance();
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        buffer.add_instance(instance_3);
        buffer.clear();
        assert_eq!(buffer.n_valid_instances(), 0);
        assert_eq!(buffer.valid_instances(), &[]);
        buffer.add_instance(instance_1);
        buffer.add_instance(instance_2);
        buffer.add_instance(instance_3);
        assert_eq!(buffer.n_valid_instances(), 3);
        assert_eq!(
            buffer.valid_instances(),
            &[instance_1, instance_2, instance_3]
        );
    }
}
