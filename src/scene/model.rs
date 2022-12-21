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
pub struct ModelInstancePool<F> {
    /// Buffers each holding the instances of a specific model.
    pub model_instance_buffers: HashMap<ModelID, ModelInstanceBuffer<F>>,
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
pub struct ModelInstanceBuffer<F> {
    raw_buffer: Vec<ModelInstance<F>>,
    n_valid_instances: AtomicUsize,
}

/// An instance of a model with a certain transformation
/// applied to it.
///
/// Used to represent multiple versions of the same basic model.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct ModelInstance<F> {
    transform_matrix: Matrix4<F>,
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

    /// Creates a model instance pool for the models
    /// with the given IDs.
    pub fn for_models(model_ids: impl IntoIterator<Item = ModelID>) -> Self {
        Self {
            model_instance_buffers: model_ids
                .into_iter()
                .map(|model_id| (model_id, ModelInstanceBuffer::new()))
                .collect(),
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
unsafe impl<F> Zeroable for ModelInstance<F> where Matrix4<F>: Zeroable {}

unsafe impl<F> Pod for ModelInstance<F>
where
    F: Float,
    Matrix4<F>: Pod,
{
}
