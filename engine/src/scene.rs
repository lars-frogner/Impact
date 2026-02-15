//! Scene containing data to render.

use crate::lock_order::OrderedRwLock;
use impact_camera::{CameraContext, CameraManager};
use impact_id::EntityIDManager;
use impact_intersection::IntersectionManager;
use impact_light::LightManager;
use impact_scene::{
    graph::{SceneGraph, SceneGroupID},
    model::{ModelInstanceManager, ModelInstanceManagerState},
    skybox::Skybox,
};
use impact_voxel::VoxelManager;
use parking_lot::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    skybox: RwLock<Option<Skybox>>,
    camera_manager: RwLock<CameraManager>,
    light_manager: RwLock<LightManager>,
    voxel_manager: RwLock<VoxelManager>,
    model_instance_manager: RwLock<ModelInstanceManager>,
    initial_model_instance_manager_state: ModelInstanceManagerState,
    intersection_manager: RwLock<IntersectionManager>,
    scene_graph: RwLock<SceneGraph>,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(
        entity_id_manager: &mut EntityIDManager,
        camera_context: CameraContext,
        model_instance_manager: ModelInstanceManager,
    ) -> Self {
        let scene_graph_root_node_id = SceneGroupID::from_entity_id(entity_id_manager.provide_id());
        let initial_model_instance_manager_state = model_instance_manager.record_state();
        Self {
            skybox: RwLock::new(None),
            camera_manager: RwLock::new(CameraManager::new(camera_context)),
            light_manager: RwLock::new(LightManager::new()),
            voxel_manager: RwLock::new(VoxelManager::new()),
            model_instance_manager: RwLock::new(model_instance_manager),
            initial_model_instance_manager_state,
            intersection_manager: RwLock::new(IntersectionManager::new()),
            scene_graph: RwLock::new(SceneGraph::new(scene_graph_root_node_id)),
        }
    }

    /// Returns a reference to the [`Skybox`], or [`None`] if no skybox has
    /// been set, guarded by a [`RwLock`].
    pub fn skybox(&self) -> &RwLock<Option<Skybox>> {
        &self.skybox
    }

    /// Returns a reference to the [`CameraManager`], guarded by a [`RwLock`].
    pub fn camera_manager(&self) -> &RwLock<CameraManager> {
        &self.camera_manager
    }

    /// Returns a reference to the [`LightManager`], guarded by a [`RwLock`].
    pub fn light_manager(&self) -> &RwLock<LightManager> {
        &self.light_manager
    }

    /// Returns a reference to the [`VoxelManager`], guarded by a [`RwLock`].
    pub fn voxel_manager(&self) -> &RwLock<VoxelManager> {
        &self.voxel_manager
    }

    /// Returns a reference to the [`ModelInstanceManager`], guarded by a
    /// [`RwLock`].
    pub fn model_instance_manager(&self) -> &RwLock<ModelInstanceManager> {
        &self.model_instance_manager
    }

    /// Returns a reference to the [`IntersectionManager`], guarded by a
    /// [`RwLock`].
    pub fn intersection_manager(&self) -> &RwLock<IntersectionManager> {
        &self.intersection_manager
    }

    /// Returns a reference to the [`SceneGraph`], guarded by a [`RwLock`].
    pub fn scene_graph(&self) -> &RwLock<SceneGraph> {
        &self.scene_graph
    }

    pub fn set_skybox(&self, skybox: Option<Skybox>) {
        **self.skybox.owrite() = skybox;
    }

    pub fn handle_aspect_ratio_changed(&self, new_aspect_ratio: f32) {
        self.camera_manager
            .owrite()
            .set_aspect_ratio(new_aspect_ratio);
    }

    /// Resets the scene to the initial empty state.
    pub fn clear(&self) {
        self.skybox.owrite().take();

        self.camera_manager.owrite().remove_all_cameras();

        self.light_manager.owrite().remove_all_lights();

        self.voxel_manager.owrite().remove_all_voxel_entities();

        self.model_instance_manager
            .owrite()
            .reset_to_state(&self.initial_model_instance_manager_state);

        self.intersection_manager
            .owrite()
            .remove_all_intersection_state();

        self.scene_graph.owrite().clear_nodes();
    }
}
