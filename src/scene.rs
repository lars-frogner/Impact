//! Scene containing data to render.

pub mod components;
pub mod entity;
mod graph;
mod systems;
mod tasks;

pub use graph::{
    CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage, NodeTransform, SceneGraph,
    SceneGraphNodeID, VoxelTreeNode, VoxelTreeNodeID,
};
pub use systems::{SyncLightsInStorage, SyncSceneObjectTransforms};
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform, UpdateSceneGroupToWorldTransforms,
};

use crate::{
    camera::SceneCamera, gpu::rendering::fre, light::LightStorage, material::MaterialLibrary,
    mesh::MeshRepository, model::InstanceFeatureManager, voxel::VoxelManager, window,
};
use num_traits::FromPrimitive;
use std::{num::NonZeroU32, sync::RwLock};

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    mesh_repository: RwLock<MeshRepository<fre>>,
    material_library: RwLock<MaterialLibrary>,
    light_storage: RwLock<LightStorage>,
    instance_feature_manager: RwLock<InstanceFeatureManager>,
    voxel_manager: RwLock<VoxelManager<fre>>,
    scene_graph: RwLock<SceneGraph<fre>>,
    scene_camera: RwLock<Option<SceneCamera<fre>>>,
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
        initial_mesh_repository: MeshRepository<fre>,
        initial_material_library: MaterialLibrary,
        initial_instance_feature_manager: InstanceFeatureManager,
        initial_voxel_manager: VoxelManager<fre>,
    ) -> Self {
        Self {
            mesh_repository: RwLock::new(initial_mesh_repository),
            material_library: RwLock::new(initial_material_library),
            light_storage: RwLock::new(LightStorage::new()),
            instance_feature_manager: RwLock::new(initial_instance_feature_manager),
            voxel_manager: RwLock::new(initial_voxel_manager),
            scene_graph: RwLock::new(SceneGraph::new()),
            scene_camera: RwLock::new(None),
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
    pub fn voxel_manager(&self) -> &RwLock<VoxelManager<fre>> {
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

    pub fn handle_window_resized(
        &self,
        _old_width: NonZeroU32,
        old_height: NonZeroU32,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        if let Some(scene_camera) = self.scene_camera().write().unwrap().as_mut() {
            scene_camera.set_aspect_ratio(window::calculate_aspect_ratio(new_width, new_height));
            desynchronized = RenderResourcesDesynchronized::Yes;
        }

        self.voxel_manager()
            .write()
            .unwrap()
            .scale_min_angular_voxel_extent_for_lod(
                fre::from_u32(old_height.into()).unwrap()
                    / fre::from_u32(new_height.into()).unwrap(),
            );

        desynchronized
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
