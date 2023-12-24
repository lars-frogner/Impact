//! Tasks for coordination between systems in the scene.

use super::Scene;
use crate::{
    define_task,
    rendering::RenderingTag,
    scene::systems::{
        SyncLightPositionsAndDirectionsInStorage, SyncLightRadiancesInStorage,
        SyncSceneObjectTransforms,
    },
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
    world::{World, WorldTaskScheduler},
};
use anyhow::{anyhow, Result};

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the group-to-world
    /// transforms of all [`SceneGraph`](crate::scene::SceneGraph) group nodes.
    [pub] UpdateSceneGroupToWorldTransforms,
    depends_on = [SyncSceneObjectTransforms],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Updating scene object group-to-world transforms"; {
            let scene = world.scene().read().unwrap();
            scene.scene_graph()
                .write()
                .unwrap()
                .update_all_group_to_root_transforms();

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to update the view transform of
    /// the scene camera.
    [pub] SyncSceneCameraViewTransform,
    depends_on = [
        SyncSceneObjectTransforms,
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene camera view transform"; {
            let scene = world.scene().read().unwrap();
            if let Some(scene_camera) = scene.scene_camera().write().unwrap().as_mut() {
                scene.scene_graph()
                    .read()
                    .unwrap()
                    .sync_camera_view_transform(scene_camera);

                world
                    .renderer()
                    .read()
                    .unwrap()
                    .declare_render_resources_desynchronized();
            }
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the bounding spheres of
    /// all [`SceneGraph`](crate::scene::SceneGraph) nodes.
    [pub] UpdateSceneObjectBoundingSpheres,
    depends_on = [SyncSceneObjectTransforms],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Updating scene object bounding spheres"; {
            let scene = world.scene().read().unwrap();
            scene.scene_graph()
                .write()
                .unwrap()
                .update_all_bounding_spheres();

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to determine which
    /// model instances are visible with the scene camera, update
    /// their model-to-camera space transforms and buffer their
    /// features for rendering.
    [pub] BufferVisibleModelInstances,
    depends_on = [
        UpdateSceneObjectBoundingSpheres,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Buffering visible model instances"; {
            let scene = world.scene().read().unwrap();
            let maybe_scene_camera = scene.scene_camera().read().unwrap();
            let scene_camera = maybe_scene_camera.as_ref().ok_or_else(|| {
                anyhow!("Tried to buffer visible model instances without scene camera")
            })?;

            scene.scene_graph()
                .read()
                .unwrap()
                .buffer_transforms_of_visible_model_instances(
                    &mut scene.instance_feature_manager().write().unwrap(),
                    &scene.voxel_manager().read().unwrap(),
                    scene_camera,
                );

            world
                .renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to determine which model
    /// instances may cast a visible shadows for each omnidirectional light,
    /// bounds the light's cubemap projections to encompass these and buffer
    /// their model to cubemap face space transforms for shadow mapping.
    [pub] BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        SyncLightPositionsAndDirectionsInStorage,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Bounding omnidirectional lights and buffering shadow casting model instances"; {
            if world.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = world.scene().read().unwrap();
                let maybe_scene_camera = scene.scene_camera().read().unwrap();
                let scene_camera = maybe_scene_camera.as_ref().ok_or_else(|| {
                    anyhow!("Tried to bound omnidirectional lights without scene camera")
                })?;

                scene.scene_graph()
                    .read()
                    .unwrap()
                    .bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut scene.light_storage().write().unwrap(),
                        &mut scene.instance_feature_manager().write().unwrap(),
                        &scene.voxel_manager().read().unwrap(),
                        scene_camera,
                    );

                world
                    .renderer()
                    .read()
                    .unwrap()
                    .declare_render_resources_desynchronized();
            }
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to determine which model
    /// instances may cast a visible shadows for each unidirectional light,
    /// bounds the light's orthographic projection to encompass these and buffer
    /// their model to light transforms for shadow mapping.
    [pub] BoundUnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        SyncLightPositionsAndDirectionsInStorage,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Bounding unidirectional lights and buffering shadow casting model instances"; {
            if world.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = world.scene().read().unwrap();
                let maybe_scene_camera = scene.scene_camera().read().unwrap();
                let scene_camera = maybe_scene_camera.as_ref().ok_or_else(|| {
                    anyhow!("Tried to bound unidirectional lights without scene camera")
                })?;

                scene.scene_graph()
                    .read()
                    .unwrap()
                    .bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut scene.light_storage().write().unwrap(),
                        &mut scene.instance_feature_manager().write().unwrap(),
                        &scene.voxel_manager().read().unwrap(),
                        scene_camera,
                    );

                world
                    .renderer()
                    .read()
                    .unwrap()
                    .declare_render_resources_desynchronized();
            }
            Ok(())
        })
    }
);

impl Scene {
    /// Registers all tasks needed for coordinate between systems
    /// in the scene in the given task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncSceneObjectTransforms)?;
        task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
        task_scheduler.register_task(SyncSceneCameraViewTransform)?;
        task_scheduler.register_task(UpdateSceneObjectBoundingSpheres)?;
        task_scheduler.register_task(BufferVisibleModelInstances)?;
        task_scheduler.register_task(SyncLightPositionsAndDirectionsInStorage)?;
        task_scheduler.register_task(SyncLightRadiancesInStorage)?;
        task_scheduler
            .register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
        task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)
    }

    /// Identifies scene-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &ThreadPoolTaskErrors,
        event_loop_controller: &EventLoopController<'_>,
    ) {
        if task_errors.n_errors() > 0 {
            log::error!("Aborting due to fatal errors");
            event_loop_controller.exit();
        }
    }
}
