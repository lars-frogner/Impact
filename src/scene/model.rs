//! Management of models.

use crate::{
    hash::{self, Hash64},
    num::Float,
    rendering::MaterialID,
    scene::MeshID,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;
use std::{
    cmp,
    collections::HashMap,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    sync::atomic::{AtomicUsize, Ordering},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_id: MaterialID,
    hash: Hash64,
}

/// Container for instances of specific models identified
/// by [`ModelID`]s.
///
/// The data in each contained [`ModelInstanceBuffer`] is
/// intended to be short-lived, as it will consist of the
/// final transforms for model instances that will be passed
/// on to the renderer.
#[derive(Debug, Default)]
pub struct ModelInstancePool<F: Float> {
    /// Buffers each holding the instances of a specific model.
    model_instance_buffers: HashMap<ModelID, UserCountingModelInstanceBuffer<F>>,
}

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

/// An instance of a model with a certain transformation
/// applied to it.
///
/// Used to represent multiple versions of the same basic model.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelInstance<F: Float> {
    transform_matrix: Matrix4<F>,
}

#[derive(Debug)]
struct UserCountingModelInstanceBuffer<F: Float> {
    user_count: u64,
    buffer: ModelInstanceBuffer<F>,
}

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the
    /// mesh and material with the given IDs.
    pub fn for_mesh_and_material(mesh_id: MeshID, material_id: MaterialID) -> Self {
        let hash = hash::compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());
        Self {
            mesh_id,
            material_id,
            hash,
        }
    }

    /// The ID of the model mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The ID of the model material.
    pub fn material_id(&self) -> MaterialID {
        self.material_id
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}}}",
            self.mesh_id, self.material_id
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

impl<F: Float> ModelInstancePool<F> {
    /// Creates a new empty model instance pool.
    pub fn new() -> Self {
        Self {
            model_instance_buffers: HashMap::new(),
        }
    }

    /// Returns an iterator over the model IDs and the associated
    /// instance buffers in the pool.
    pub fn models_and_buffers<'a>(
        &'a self,
    ) -> impl Iterator<Item = (ModelID, &'a ModelInstanceBuffer<F>)> {
        self.model_instance_buffers
            .iter()
            .map(|(model_id, buffer)| (*model_id, &buffer.buffer))
    }

    /// Whether the pool has an instance buffer for the model with
    /// the given ID.
    pub fn has_buffer_for_model(&self, model_id: ModelID) -> bool {
        self.model_instance_buffers.contains_key(&model_id)
    }

    /// Returns a reference to the  [`ModelInstanceBuffer`] for
    /// the model with the given ID, or [`None`] if the model is
    /// not present.
    pub fn get_buffer(&mut self, model_id: ModelID) -> Option<&ModelInstanceBuffer<F>> {
        self.model_instance_buffers
            .get(&model_id)
            .map(|buffer| &buffer.buffer)
    }

    /// Returns a mutable reference to the  [`ModelInstanceBuffer`]
    /// for the model with the given ID, or [`None`] if the model is
    /// not present.
    pub fn get_buffer_mut(&mut self, model_id: ModelID) -> Option<&mut ModelInstanceBuffer<F>> {
        self.model_instance_buffers
            .get_mut(&model_id)
            .map(|buffer| &mut buffer.buffer)
    }

    /// Increments the count of users of the model with the given ID.
    pub fn increment_user_count(&mut self, model_id: ModelID) {
        self.model_instance_buffers
            .entry(model_id)
            .and_modify(|buffer| buffer.increment_user_count())
            .or_default();
    }

    /// Decrements the count of users of the model with the given ID.
    ///
    /// # Panics
    /// If the specified model is not represented in the pool.
    pub fn decrement_user_count(&mut self, model_id: ModelID) {
        let buffer = self
            .model_instance_buffers
            .get_mut(&model_id)
            .expect("Tried to decrement user count of model missing from pool");
        buffer.decrement_user_count();
        if buffer.no_users() {
            self.model_instance_buffers.remove(&model_id);
        }
    }
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

impl<F: Float> ModelInstance<F> {
    /// Creates a new model instance with no transform.
    pub fn new() -> Self {
        Self::with_transform(Matrix4::identity())
    }

    /// Creates a new model instance with the given transform.
    pub fn with_transform(transform_matrix: Matrix4<F>) -> Self {
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

impl<F: Float> UserCountingModelInstanceBuffer<F> {
    fn new() -> Self {
        Self {
            user_count: 1,
            buffer: ModelInstanceBuffer::new(),
        }
    }

    fn no_users(&self) -> bool {
        self.user_count == 0
    }

    fn increment_user_count(&mut self) {
        self.user_count += 1;
    }

    fn decrement_user_count(&mut self) {
        assert!(self.user_count >= 1);
        self.user_count -= 1;
    }
}

impl<F: Float> Default for UserCountingModelInstanceBuffer<F> {
    fn default() -> Self {
        Self::new()
    }
}
