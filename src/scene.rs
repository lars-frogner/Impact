//! Scene containing data to render.

mod camera;
mod components;
mod graph;
mod material;
mod mesh;
mod model;
mod systems;
mod tasks;

pub use camera::{CameraID, CameraRepository};
pub use components::{CameraComp, MeshComp, SceneGraphNodeComp};
pub use graph::{
    model_to_world_transform_from_position_and_orientation, CameraNodeID, GroupNodeID,
    ModelInstanceNodeID, NodeStorage, NodeTransform, SceneGraph, SceneGraphNodeID,
};
pub use material::{MaterialID, MaterialLibrary, MaterialSpecification};
pub use mesh::{MeshID, MeshRepository};
pub use model::{ModelID, ModelInstancePool};
pub use tasks::SyncVisibleModelInstances;

use crate::rendering::fre;
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    camera_repository: RwLock<CameraRepository<fre>>,
    mesh_repository: RwLock<MeshRepository<fre>>,
    material_library: RwLock<MaterialLibrary>,
    scene_graph: RwLock<SceneGraph<fre>>,
    model_instance_pool: RwLock<ModelInstancePool<fre>>,
    active_camera: RwLock<Option<(CameraID, CameraNodeID)>>,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(
        camera_repository: CameraRepository<fre>,
        mesh_repository: MeshRepository<fre>,
        material_library: MaterialLibrary,
    ) -> Self {
        Self {
            camera_repository: RwLock::new(camera_repository),
            mesh_repository: RwLock::new(mesh_repository),
            material_library: RwLock::new(material_library),
            model_instance_pool: RwLock::new(ModelInstancePool::new()),
            scene_graph: RwLock::new(SceneGraph::new()),
            active_camera: RwLock::new(None),
        }
    }

    /// Returns a reference to the [`CameraRepository`], guarded
    /// by a [`RwLock`].
    pub fn camera_repository(&self) -> &RwLock<CameraRepository<fre>> {
        &self.camera_repository
    }

    /// Returns a reference to the [`MeshRepository`], guarded
    /// by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<fre>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`MaterialLibrary`], guarded
    /// by a [`RwLock`].
    pub fn material_library(&self) -> &RwLock<MaterialLibrary> {
        &self.material_library
    }

    /// Returns a reference to the [`ModelInstancePool`], guarded
    /// by a [`RwLock`].
    pub fn model_instance_pool(&self) -> &RwLock<ModelInstancePool<fre>> {
        &self.model_instance_pool
    }

    /// Returns a reference to the [`SceneGraph`], guarded
    /// by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph<fre>> {
        &self.scene_graph
    }

    /// Returns the [`CameraID`] if the currently active camera,
    /// or [`None`] if there is no active camera.
    pub fn get_active_camera_id(&self) -> Option<CameraID> {
        self.active_camera
            .read()
            .unwrap()
            .map(|(camera_id, _)| camera_id)
    }

    /// Returns the [`CameraNodeID`] if the currently active camera,
    /// or [`None`] if there is no active camera.
    pub fn get_active_camera_node_id(&self) -> Option<CameraNodeID> {
        self.active_camera
            .read()
            .unwrap()
            .map(|(_, camera_node_id)| camera_node_id)
    }

    /// Uses the camera with the given camera ID and node ID in the
    /// [`SceneGraph`] as the active camera, or disable the active
    /// camera if the value is [`None`].
    ///
    /// # Note
    /// It is the responsibility of the caller to ensure that the
    /// given combination of camera ID and node ID is valid.
    pub fn set_active_camera(&self, active_camera: Option<(CameraID, CameraNodeID)>) {
        *self.active_camera.write().unwrap() = active_camera;
    }
}
