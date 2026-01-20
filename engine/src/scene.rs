//! Scene containing data to render.

use crate::lock_order::OrderedRwLock;
use impact_light::{LightConfig, LightManager};
use impact_scene::{
    camera::{CameraContext, CameraManager},
    graph::SceneGraph,
    model::{ModelInstanceManager, ModelInstanceManagerState},
    skybox::Skybox,
};
use impact_voxel::VoxelObjectManager;
use parking_lot::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    skybox: RwLock<Option<Skybox>>,
    camera_manager: RwLock<CameraManager>,
    light_manager: RwLock<LightManager>,
    voxel_object_manager: RwLock<VoxelObjectManager>,
    model_instance_manager: RwLock<ModelInstanceManager>,
    initial_model_instance_manager_state: ModelInstanceManagerState,
    scene_graph: RwLock<SceneGraph>,
}

impl Scene {
    /// Creates a new scene data container.
    pub fn new(
        camera_context: CameraContext,
        light_config: LightConfig,
        model_instance_manager: ModelInstanceManager,
    ) -> Self {
        let initial_model_instance_manager_state = model_instance_manager.record_state();
        Self {
            skybox: RwLock::new(None),
            camera_manager: RwLock::new(CameraManager::new(camera_context)),
            light_manager: RwLock::new(LightManager::new(light_config)),
            voxel_object_manager: RwLock::new(VoxelObjectManager::new()),
            model_instance_manager: RwLock::new(model_instance_manager),
            initial_model_instance_manager_state,
            scene_graph: RwLock::new(SceneGraph::new()),
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

    /// Returns a reference to the [`VoxelObjectManager`], guarded by a
    /// [`RwLock`].
    pub fn voxel_object_manager(&self) -> &RwLock<VoxelObjectManager> {
        &self.voxel_object_manager
    }

    /// Returns a reference to the [`ModelInstanceManager`], guarded by a
    /// [`RwLock`].
    pub fn model_instance_manager(&self) -> &RwLock<ModelInstanceManager> {
        &self.model_instance_manager
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

        self.voxel_object_manager
            .owrite()
            .remove_all_voxel_objects();

        self.model_instance_manager
            .owrite()
            .reset_to_state(&self.initial_model_instance_manager_state);

        self.scene_graph.owrite().clear_nodes();
    }
}
