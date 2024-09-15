//! Scene containing data to render.

pub mod components;
pub mod entity;
mod graph;
pub mod systems;
pub mod tasks;

pub use graph::{
    CameraNodeID, GroupNodeID, ModelInstanceNode, ModelInstanceNodeID, NodeStorage, NodeTransform,
    SceneGraph, SceneGraphNodeID,
};

use crate::{
    camera::SceneCamera,
    gpu::rendering::fre,
    light::LightStorage,
    material::{MaterialLibrary, MaterialLibraryState},
    mesh::{MeshRepository, MeshRepositoryState},
    model::InstanceFeatureManager,
    skybox::Skybox,
    voxel::VoxelManager,
    window,
};
use std::{num::NonZeroU32, sync::RwLock};

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    mesh_repository: RwLock<MeshRepository<fre>>,
    initial_mesh_repository_state: MeshRepositoryState,
    material_library: RwLock<MaterialLibrary>,
    initial_material_library_state: MaterialLibraryState,
    light_storage: RwLock<LightStorage>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    voxel_manager: RwLock<VoxelManager>,
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
    skybox: RwLock<Option<Skybox>>,
}

/// Indicates whether the render resources are out of sync with its source scene
/// data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderResourcesDesynchronized {
    Yes,
    No,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(
        mesh_repository: MeshRepository<fre>,
        material_library: MaterialLibrary,
        instance_feature_manager: InstanceFeatureManager,
        voxel_manager: VoxelManager,
    ) -> Self {
        let initial_mesh_repository_state = mesh_repository.record_state();
        let initial_material_library_state = material_library.record_state();
        Self {
            mesh_repository: RwLock::new(mesh_repository),
            initial_mesh_repository_state,
            material_library: RwLock::new(material_library),
            initial_material_library_state,
            light_storage: RwLock::new(LightStorage::new()),
            instance_feature_manager: RwLock::new(instance_feature_manager),
            voxel_manager: RwLock::new(voxel_manager),
            scene_graph: RwLock::new(SceneGraph::new()),
            scene_camera: RwLock::new(None),
            skybox: RwLock::new(None),
        }
    }

    /// Returns a reference to the [`MeshRepository`], guarded by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<fre>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`MaterialLibrary`], guarded by a [`RwLock`].
    pub fn material_library(&self) -> &RwLock<MaterialLibrary> {
        &self.material_library
    }

    /// Returns a reference to the [`LightStorage`], guarded by a [`RwLock`].
    pub fn light_storage(&self) -> &RwLock<LightStorage> {
        &self.light_storage
    }

    /// Returns a reference to the [`InstanceFeatureManager`], guarded by a
    /// [`RwLock`].
    pub fn instance_feature_manager(&self) -> &RwLock<InstanceFeatureManager> {
        &self.instance_feature_manager
    }

    /// Returns a reference to the [`VoxelManager`], guarded by a [`RwLock`].
    pub fn voxel_manager(&self) -> &RwLock<VoxelManager> {
        &self.voxel_manager
    }

    /// Returns a reference to the [`SceneGraph`], guarded by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph<fre>> {
        &self.scene_graph
    }

    /// Returns a reference to the [`SceneCamera`], or [`None`] if no scene
    /// camera has been set, guarded by a [`RwLock`].
    pub fn scene_camera(&self) -> &RwLock<Option<SceneCamera<fre>>> {
        &self.scene_camera
    }

    /// Returns a reference to the [`Skybox`], or [`None`] if no skybox has
    /// been set, guarded by a [`RwLock`].
    pub fn skybox(&self) -> &RwLock<Option<Skybox>> {
        &self.skybox
    }

    pub fn handle_window_resized(
        &self,
        _old_width: NonZeroU32,
        _old_height: NonZeroU32,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        if let Some(scene_camera) = self.scene_camera().write().unwrap().as_mut() {
            scene_camera.set_aspect_ratio(window::calculate_aspect_ratio(new_width, new_height));
            desynchronized = RenderResourcesDesynchronized::Yes;
        }

        desynchronized
    }

    /// Resets the scene to the initial empty state.
    pub fn clear(&self) {
        self.mesh_repository
            .write()
            .unwrap()
            .reset_to_state(&self.initial_mesh_repository_state);

        self.material_library
            .write()
            .unwrap()
            .reset_to_state(&self.initial_material_library_state);

        self.light_storage.write().unwrap().remove_all_lights();

        self.instance_feature_manager
            .write()
            .unwrap()
            .clear_storages_and_buffers();

        self.voxel_manager
            .write()
            .unwrap()
            .remove_all_voxel_objects();

        self.scene_graph.write().unwrap().clear_nodes();

        self.scene_camera.write().unwrap().take();

        self.skybox.write().unwrap().take();
    }
}

impl RenderResourcesDesynchronized {
    pub fn is_yes(&self) -> bool {
        *self == Self::Yes
    }

    pub fn set_yes(&mut self) {
        *self = Self::Yes;
    }
}
