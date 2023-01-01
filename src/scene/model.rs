//! Management of models.

use crate::{
    geometry::{ModelInstanceBuffer, UserCountingModelInstanceBuffer},
    hash::{self, Hash64},
    num::Float,
    rendering::MaterialID,
    scene::MeshID,
};
use std::{
    cmp,
    collections::HashMap,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
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
            .map(|(model_id, buffer)| (*model_id, buffer.inner()))
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
            .map(|buffer| buffer.inner())
    }

    /// Returns a mutable reference to the  [`ModelInstanceBuffer`]
    /// for the model with the given ID, or [`None`] if the model is
    /// not present.
    pub fn get_buffer_mut(&mut self, model_id: ModelID) -> Option<&mut ModelInstanceBuffer<F>> {
        self.model_instance_buffers
            .get_mut(&model_id)
            .map(|buffer| buffer.inner_mut())
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

#[cfg(test)]
mod test {
    use super::*;

    fn create_dummy_model_id<S: AsRef<str>>(tag: S) -> ModelID {
        ModelID::for_mesh_and_material(
            MeshID(hash!(format!("Test mesh {}", tag.as_ref()))),
            MaterialID(hash!(format!("Test material {}", tag.as_ref()))),
        )
    }

    #[test]
    fn creating_model_instance_pool_works() {
        let pool = ModelInstancePool::<f32>::new();
        assert!(pool.models_and_buffers().next().is_none());
    }

    #[test]
    fn adding_one_use_of_model_in_model_instance_pool_works() {
        let model_id = create_dummy_model_id("");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.increment_user_count(model_id);
        let mut models_and_buffers = pool.models_and_buffers();
        assert_eq!(models_and_buffers.next().unwrap().0, model_id);
        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);
        assert!(pool.has_buffer_for_model(model_id));
        assert!(pool.get_buffer(model_id).is_some());
    }

    #[test]
    fn adding_one_use_of_two_models_in_model_instance_pool_works() {
        let model_id_1 = create_dummy_model_id("1");
        let model_id_2 = create_dummy_model_id("2");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.increment_user_count(model_id_1);
        pool.increment_user_count(model_id_2);
        let mut models_and_buffers = pool.models_and_buffers();
        assert_ne!(
            models_and_buffers.next().unwrap().0,
            models_and_buffers.next().unwrap().0
        );
        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);
        assert!(pool.has_buffer_for_model(model_id_1));
        assert!(pool.has_buffer_for_model(model_id_2));
        assert!(pool.get_buffer(model_id_1).is_some());
        assert!(pool.get_buffer(model_id_2).is_some());
    }

    #[test]
    fn adding_two_uses_of_model_in_model_instance_pool_works() {
        let model_id = create_dummy_model_id("");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.increment_user_count(model_id);
        pool.increment_user_count(model_id);
        let mut models_and_buffers = pool.models_and_buffers();
        assert_eq!(models_and_buffers.next().unwrap().0, model_id);
        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);
        assert!(pool.has_buffer_for_model(model_id));
        assert!(pool.get_buffer(model_id).is_some());
    }

    #[test]
    fn adding_and_then_removing_one_use_of_model_in_model_instance_pool_works() {
        let model_id = create_dummy_model_id("");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.increment_user_count(model_id);
        pool.decrement_user_count(model_id);
        assert!(pool.models_and_buffers().next().is_none());
        assert!(!pool.has_buffer_for_model(model_id));
        assert!(pool.get_buffer(model_id).is_none());
        pool.increment_user_count(model_id);
        pool.decrement_user_count(model_id);
        assert!(pool.models_and_buffers().next().is_none());
        assert!(!pool.has_buffer_for_model(model_id));
        assert!(pool.get_buffer(model_id).is_none());
    }

    #[test]
    fn adding_and_then_removing_two_uses_of_model_in_model_instance_pool_works() {
        let model_id = create_dummy_model_id("");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.increment_user_count(model_id);
        pool.increment_user_count(model_id);
        pool.decrement_user_count(model_id);
        pool.decrement_user_count(model_id);
        assert!(pool.models_and_buffers().next().is_none());
        assert!(!pool.has_buffer_for_model(model_id));
        assert!(pool.get_buffer(model_id).is_none());
    }

    #[test]
    #[should_panic]
    fn removing_use_of_model_in_empty_model_instance_pool_fails() {
        let model_id = create_dummy_model_id("");
        let mut pool = ModelInstancePool::<f32>::new();
        pool.decrement_user_count(model_id);
    }
}
