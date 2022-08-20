//! Container for all data in the world.

mod tasks;

pub use tasks::SyncVisibleModelInstances;

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::{
        CameraID, CameraNodeID, CameraRepository, MeshRepository, ModelID, ModelInstanceNodeID,
        ModelInstancePool, SceneGraph,
    },
    rendering::{ModelLibrary, RenderingSystem},
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
};
use anyhow::{anyhow, Result};
use nalgebra::Similarity3;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    model_library: RwLock<ModelLibrary>,
    camera_repository: RwLock<CameraRepository<f32>>,
    mesh_repository: RwLock<MeshRepository<f32>>,
    model_instance_pool: RwLock<ModelInstancePool<f32>>,
    scene_graph: RwLock<SceneGraph<f32>>,
    renderer: RwLock<RenderingSystem>,
    motion_controller: Mutex<Box<dyn MotionController<f32>>>,
    active_camera: Option<(CameraID, CameraNodeID)>,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        model_library: ModelLibrary,
        camera_repository: CameraRepository<f32>,
        mesh_repository: MeshRepository<f32>,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        let model_instance_pool = ModelInstancePool::for_models(model_library.model_ids());
        Self {
            model_library: RwLock::new(model_library),
            camera_repository: RwLock::new(camera_repository),
            mesh_repository: RwLock::new(mesh_repository),
            model_instance_pool: RwLock::new(model_instance_pool),
            scene_graph: RwLock::new(SceneGraph::new()),
            renderer: RwLock::new(renderer),
            motion_controller: Mutex::new(Box::new(controller)),
            active_camera: None,
        }
    }

    /// Returns a reference to the [`ModelLibrary`], guarded
    /// by a [`RwLock`].
    pub fn model_library(&self) -> &RwLock<ModelLibrary> {
        &self.model_library
    }

    /// Returns a reference to the [`CameraRepository`], guarded
    /// by a [`RwLock`].
    pub fn camera_repository(&self) -> &RwLock<CameraRepository<f32>> {
        &self.camera_repository
    }

    /// Returns a reference to the [`MeshRepository`], guarded
    /// by a [`RwLock`].
    pub fn mesh_repository(&self) -> &RwLock<MeshRepository<f32>> {
        &self.mesh_repository
    }

    /// Returns a reference to the [`ModelInstancePool`], guarded
    /// by a [`RwLock`].
    pub fn model_instance_pool(&self) -> &RwLock<ModelInstancePool<f32>> {
        &self.model_instance_pool
    }

    /// Returns a reference to the [`RenderingSystem`], guarded
    /// by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    pub fn get_active_camera_id(&self) -> Option<CameraID> {
        self.active_camera.map(|(camera_id, _)| camera_id)
    }

    pub fn get_active_camera_node_id(&self) -> Option<CameraNodeID> {
        self.active_camera.map(|(_, camera_node_id)| camera_node_id)
    }

    pub fn spawn_camera(&self, camera_id: CameraID, transform: Similarity3<f32>) -> CameraNodeID {
        let mut scene_graph = self.scene_graph.write().unwrap();
        let parent_node_id = scene_graph.root_node_id();
        scene_graph.create_camera_node(parent_node_id, transform, camera_id)
    }

    pub fn spawn_model_instances(
        &self,
        model_id: ModelID,
        transforms: impl IntoIterator<Item = Similarity3<f32>>,
    ) -> Result<Vec<ModelInstanceNodeID>> {
        let mesh_id = self
            .model_library
            .read()
            .unwrap()
            .get_model(model_id)
            .ok_or_else(|| anyhow!("Model {} not present in model library", model_id))?
            .mesh_id;

        let bounding_sphere = self
            .mesh_repository()
            .read()
            .unwrap()
            .get_mesh(mesh_id)
            .ok_or_else(|| anyhow!("Mesh {} not present in mesh repository", mesh_id))?
            .bounding_sphere()
            .ok_or_else(|| anyhow!("Mesh {} is empty", mesh_id))?;

        let mut scene_graph = self.scene_graph.write().unwrap();
        let parent_node_id = scene_graph.root_node_id();
        Ok(transforms
            .into_iter()
            .map(|transform| {
                scene_graph.create_model_instance_node(
                    parent_node_id,
                    bounding_sphere.clone(),
                    transform,
                    model_id,
                )
            })
            .collect())
    }

    pub fn set_active_camera(&mut self, camera_node_id: CameraNodeID) {
        let camera_id = self.scene_graph.read().unwrap().camera_id(camera_node_id);
        self.active_camera = Some((camera_id, camera_node_id));
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        log::debug!(
            "Updating motion controller to state {:?} and direction {:?}",
            state,
            direction
        );

        let mut motion_controller = self.motion_controller.lock().unwrap();

        motion_controller.update_motion(state, direction);

        if let Some(translation) = motion_controller.next_translation() {
            drop(motion_controller); // Don't hold lock longer than neccessary

            // self.geometrical_data
            //     .write()
            //     .unwrap()
            //     .transform_cameras(&translation.into());
        }
    }

    pub fn create_task_scheduler(
        self,
        n_workers: NonZeroUsize,
    ) -> Result<(Arc<Self>, WorldTaskScheduler)> {
        let world = Arc::new(self);
        let mut task_scheduler = WorldTaskScheduler::new(n_workers, Arc::clone(&world));

        Self::register_all_tasks(&mut task_scheduler)?;

        Ok((world, task_scheduler))
    }

    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        self.handle_world_task_errors(task_errors, control_flow);
        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);
    }

    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        Self::register_world_tasks(task_scheduler)?;
        RenderingSystem::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }

    fn sync_visible_model_instances(&self) -> Result<()> {
        self.scene_graph
            .write()
            .unwrap()
            .sync_visible_model_instances(
                &mut self.model_instance_pool.write().unwrap(),
                &self.camera_repository.read().unwrap(),
                self.get_active_camera_node_id(),
            )
    }
}
