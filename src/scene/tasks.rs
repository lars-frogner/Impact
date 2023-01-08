//! Tasks for coordination between systems in the scene.

use super::Scene;
use crate::{
    define_task,
    rendering::RenderingTag,
    scene::systems::{
        SyncSceneObjectTransformsWithOrientations, SyncSceneObjectTransformsWithPositions,
    },
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to determine which
    /// model instances are visible with the active camera, update
    /// their model-to-camera space transforms and buffer their
    /// features for rendering.
    [pub] BufferVisibleModelInstances,
    depends_on = [SyncSceneObjectTransformsWithPositions],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing visible model instances"; {
            let scene = world.scene().read().unwrap();
            let result = scene.scene_graph()
                .write()
                .unwrap()
                .sync_transforms_of_visible_model_instances(
                    &mut scene.instance_feature_manager().write().unwrap(),
                    &scene.camera_repository().read().unwrap(),
                    scene.get_active_camera_node_id(),
                );

            world
                .renderer()
                .read()
                .unwrap()
                .render_resource_manager()
                .write()
                .unwrap()
                .declare_desynchronized();

            result
        })
    }
);

impl Scene {
    /// Registers all tasks needed for coordinate between systems
    /// in the scene in the given task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncSceneObjectTransformsWithPositions)?;
        task_scheduler.register_task(SyncSceneObjectTransformsWithOrientations)?;
        task_scheduler.register_task(BufferVisibleModelInstances)
    }

    /// Identifies scene-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        _task_errors: &mut ThreadPoolTaskErrors,
        _control_flow: &mut ControlFlow<'_>,
    ) {
    }
}
