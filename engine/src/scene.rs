//! Scene containing data to render.

pub mod command;
pub mod components;
pub mod entity;
mod graph;
pub mod systems;
pub mod tasks;

pub use graph::{
    CameraNode, CameraNodeID, GroupNodeID, ModelInstanceNode, ModelInstanceNodeID, NodeStorage,
    NodeTransform, SceneGraph, SceneGraphNodeID,
};

use crate::{
    camera::SceneCamera, model::InstanceFeatureManager, skybox::Skybox, voxel::VoxelManager,
};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_light::LightStorage;
use impact_material::{MaterialLibrary, MaterialLibraryState};
use impact_mesh::{MeshRepository, MeshRepositoryState};
use roc_integration::roc;
use std::sync::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    mesh_repository: RwLock<MeshRepository>,
    initial_mesh_repository_state: MeshRepositoryState,
    material_library: RwLock<MaterialLibrary>,
    initial_material_library_state: MaterialLibraryState,
    light_storage: RwLock<LightStorage>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    voxel_manager: RwLock<VoxelManager>,
    scene_graph: RwLock<SceneGraph>,
    scene_camera: RwLock<Option<SceneCamera>>,
    skybox: RwLock<Option<Skybox>>,
}

bitflags! {
    /// Bitflags encoding a set of binary states or properties for an entity in
    /// a scene.
    #[roc(parents="Scene", category="primitive")] // <- Not auto-generated, so keep Roc code synced
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct SceneEntityFlags: u8 {
        /// The entity should not affect the scene in any way.
        const IS_DISABLED      = 1 << 0;
        /// The entity should not participate in shadow maps.
        const CASTS_NO_SHADOWS = 1 << 1;
    }
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
        mesh_repository: MeshRepository,
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
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository> {
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
    pub fn scene_graph(&self) -> &RwLock<SceneGraph> {
        &self.scene_graph
    }

    /// Returns a reference to the [`SceneCamera`], or [`None`] if no scene
    /// camera has been set, guarded by a [`RwLock`].
    pub fn scene_camera(&self) -> &RwLock<Option<SceneCamera>> {
        &self.scene_camera
    }

    /// Returns a reference to the [`Skybox`], or [`None`] if no skybox has
    /// been set, guarded by a [`RwLock`].
    pub fn skybox(&self) -> &RwLock<Option<Skybox>> {
        &self.skybox
    }

    pub fn handle_aspect_ratio_changed(
        &self,
        new_aspect_ratio: f32,
    ) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        if let Some(scene_camera) = self.scene_camera().write().unwrap().as_mut() {
            scene_camera.set_aspect_ratio(new_aspect_ratio);
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
            .object_manager
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
