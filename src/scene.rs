//! Scene containing data to render.

mod events;
mod graph;
mod systems;
mod tasks;

pub use events::RenderResourcesDesynchronized;
pub use graph::{
    register_scene_graph_components, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
    NodeTransform, ParentComp, SceneGraph, SceneGraphCameraNodeComp, SceneGraphGroupComp,
    SceneGraphGroupNodeComp, SceneGraphModelInstanceNodeComp, SceneGraphNodeComp, SceneGraphNodeID,
    SceneGraphParentNodeComp, UncullableComp, VoxelTreeNode, VoxelTreeNodeID,
};
pub use systems::{SyncLightsInStorage, SyncSceneObjectTransforms};
pub use tasks::{
    BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
    SyncSceneCameraViewTransform, UpdateSceneGroupToWorldTransforms,
};

use crate::{
    camera::SceneCamera, gpu::rendering::fre, light::LightStorage, material::MaterialLibrary,
    mesh::MeshRepository, model::InstanceFeatureManager, voxel::VoxelManager,
};
use std::sync::RwLock;

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
}
