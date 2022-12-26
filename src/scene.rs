//! Scene containing data to render.

mod camera;
mod components;
mod graph;
mod mesh;
mod model;
mod systems;
mod tasks;

pub use camera::{CameraID, CameraRepository};
pub use components::{CameraComp, MeshComp, SceneGraphNodeComp};
pub use graph::{
    CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage, NodeTransform, SceneGraph,
    SceneGraphNodeID,
};
pub use mesh::{MeshID, MeshRepository};
pub use model::{ModelID, ModelInstance, ModelInstanceBuffer, ModelInstancePool};
pub use tasks::SyncVisibleModelInstances;

use crate::rendering::{fre, MaterialLibrary};
use anyhow::{anyhow, Result};
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    camera_repository: RwLock<CameraRepository<fre>>,
    mesh_repository: RwLock<MeshRepository<fre>>,
    material_library: RwLock<MaterialLibrary>,
    scene_graph: RwLock<SceneGraph<fre>>,
    model_instance_pool: RwLock<ModelInstancePool<fre>>,
    active_camera: Option<(CameraID, CameraNodeID)>,
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
            active_camera: None,
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
        self.active_camera.map(|(camera_id, _)| camera_id)
    }

    /// Returns the [`CameraNodeID`] if the currently active camera,
    /// or [`None`] if there is no active camera.
    pub fn get_active_camera_node_id(&self) -> Option<CameraNodeID> {
        self.active_camera.map(|(_, camera_node_id)| camera_node_id)
    }

    pub fn spawn_camera(&self, camera_id: CameraID, transform: NodeTransform<fre>) -> CameraNodeID {
        let mut scene_graph = self.scene_graph.write().unwrap();
        let parent_node_id = scene_graph.root_node_id();
        scene_graph.create_camera_node(parent_node_id, transform, camera_id)
    }

    pub fn spawn_model_instances(
        &self,
        model_id: ModelID,
        transforms: impl IntoIterator<Item = NodeTransform<fre>>,
    ) -> Result<Vec<ModelInstanceNodeID>> {
        // let mesh_id = self
        //     .model_library
        //     .read()
        //     .unwrap()
        //     .get_model(model_id)
        //     .ok_or_else(|| anyhow!("Model {} not present in model library", model_id))?
        //     .mesh_id;

        // let bounding_sphere = self
        //     .mesh_repository()
        //     .read()
        //     .unwrap()
        //     .get_mesh(mesh_id)
        //     .ok_or_else(|| anyhow!("Mesh {} not present in mesh repository", mesh_id))?
        //     .bounding_sphere()
        //     .ok_or_else(|| anyhow!("Mesh {} is empty", mesh_id))?;

        // let mut scene_graph = self.scene_graph.write().unwrap();
        // let parent_node_id = scene_graph.root_node_id();
        // Ok(transforms
        //     .into_iter()
        //     .map(|transform| {
        //         scene_graph.create_model_instance_node(
        //             parent_node_id,
        //             transform,
        //             model_id,
        //             bounding_sphere.clone(),
        //         )
        //     })
        //     .collect())
        todo!()
    }

    /// Uses the camera with the given node ID in the [`SceneGraph`]
    /// as the active camera.
    ///
    /// # Panics
    /// If there is no node with the given [`CameraNodeID`].
    pub fn set_active_camera(&mut self, camera_node_id: CameraNodeID) {
        let camera_id = self
            .scene_graph
            .read()
            .unwrap()
            .camera_nodes()
            .node(camera_node_id)
            .camera_id();
        self.active_camera = Some((camera_id, camera_node_id));
    }
}
