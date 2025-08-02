//! Scene containing data to render.

use impact_light::LightManager;
use impact_scene::{
    camera::SceneCamera,
    graph::SceneGraph,
    model::{ModelInstanceManager, ModelInstanceManagerState},
    skybox::Skybox,
};
use impact_voxel::VoxelObjectManager;
use parking_lot::RwLock;

/// Container for data needed to render a scene.
#[derive(Debug)]
pub struct Scene {
    light_manager: RwLock<LightManager>,
    model_instance_manager: RwLock<ModelInstanceManager>,
    initial_model_instance_manager_state: ModelInstanceManagerState,
    voxel_object_manager: RwLock<VoxelObjectManager>,
    scene_graph: RwLock<SceneGraph>,
    scene_camera: RwLock<Option<SceneCamera>>,
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
    pub fn new(model_instance_manager: ModelInstanceManager) -> Self {
        let initial_model_instance_manager_state = model_instance_manager.record_state();
        Self {
            light_manager: RwLock::new(LightManager::new()),
            model_instance_manager: RwLock::new(model_instance_manager),
            initial_model_instance_manager_state,
            voxel_object_manager: RwLock::new(VoxelObjectManager::new()),
            scene_graph: RwLock::new(SceneGraph::new()),
            scene_camera: RwLock::new(None),
            skybox: RwLock::new(None),
        }
    }

    /// Returns a reference to the [`LightManager`], guarded by a [`RwLock`].
    pub fn light_manager(&self) -> &RwLock<LightManager> {
        &self.light_manager
    }

    /// Returns a reference to the [`ModelInstanceManager`], guarded by a
    /// [`RwLock`].
    pub fn model_instance_manager(&self) -> &RwLock<ModelInstanceManager> {
        &self.model_instance_manager
    }

    /// Returns a reference to the [`VoxelObjectManager`], guarded by a
    /// [`RwLock`].
    pub fn voxel_object_manager(&self) -> &RwLock<VoxelObjectManager> {
        &self.voxel_object_manager
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

    pub fn set_skybox(&self, skybox: Option<Skybox>) {
        *self.skybox.write() = skybox;
    }

    pub fn handle_aspect_ratio_changed(
        &self,
        new_aspect_ratio: f32,
    ) -> RenderResourcesDesynchronized {
        let mut desynchronized = RenderResourcesDesynchronized::No;

        if let Some(scene_camera) = self.scene_camera().write().as_mut() {
            scene_camera.set_aspect_ratio(new_aspect_ratio);
            desynchronized = RenderResourcesDesynchronized::Yes;
        }

        desynchronized
    }

    /// Resets the scene to the initial empty state.
    pub fn clear(&self) {
        self.light_manager.write().remove_all_lights();

        self.model_instance_manager
            .write()
            .reset_to_state(&self.initial_model_instance_manager_state);

        self.voxel_object_manager.write().remove_all_voxel_objects();

        self.scene_graph.write().clear_nodes();

        self.scene_camera.write().take();

        self.skybox.write().take();
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
