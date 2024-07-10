//! Tasks for coordination between systems in the scene.

use crate::{
    application::{tasks::AppTaskScheduler, Application},
    define_task,
    gpu::rendering::tasks::RenderingTag,
    scene::{self, Scene},
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
};
use anyhow::{anyhow, Result};

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform of
    /// each [`SceneGraph`](crate::scene::SceneGraph) node representing an
    /// entity that also has the [`ReferenceFrameComp`] component so that the
    /// translational, rotational and scaling parts match the origin offset,
    /// position, orientation and scaling.
    [pub] SyncSceneObjectTransforms,
    depends_on = [],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Synchronizing scene graph node transforms"; {
            let ecs_world = app.ecs_world().read().unwrap();
            let scene = app.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();
            scene::systems::sync_scene_object_transforms(&ecs_world, &mut scene_graph);
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the properties (position,
    /// direction, emission and extent) of every light source in the
    /// [`LightStorage`](crate::light::LightStorage).
    [pub] SyncLightsInStorage,
    depends_on = [
        UpdateSceneGroupToWorldTransforms,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Synchronizing lights in storage"; {
            let scene = app.scene().read().unwrap();
            let ecs_world = app.ecs_world().read().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let mut light_storage = scene.light_storage().write().unwrap();
            scene::systems::sync_lights_in_storage(
                &ecs_world,
                &scene_graph,
                scene.scene_camera().read().unwrap().as_ref(),
                &mut light_storage,
            );
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the group-to-world
    /// transforms of all [`SceneGraph`](crate::scene::SceneGraph) group nodes.
    [pub] UpdateSceneGroupToWorldTransforms,
    depends_on = [SyncSceneObjectTransforms],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Updating scene object group-to-world transforms"; {
            let scene = app.scene().read().unwrap();
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
    |app: &Application| {
        with_debug_logging!("Synchronizing scene camera view transform"; {
            let scene = app.scene().read().unwrap();
            if let Some(scene_camera) = scene.scene_camera().write().unwrap().as_mut() {
                scene.scene_graph()
                    .read()
                    .unwrap()
                    .sync_camera_view_transform(scene_camera);

                app
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
    |app: &Application| {
        with_debug_logging!("Updating scene object bounding spheres"; {
            let scene = app.scene().read().unwrap();
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
    |app: &Application| {
        with_debug_logging!("Buffering visible model instances"; {
            let scene = app.scene().read().unwrap();
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

            app
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
        SyncLightsInStorage,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Bounding omnidirectional lights and buffering shadow casting model instances"; {
            if app.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = app.scene().read().unwrap();
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

                app
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
        SyncLightsInStorage,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Bounding unidirectional lights and buffering shadow casting model instances"; {
            if app.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = app.scene().read().unwrap();
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

                app
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
    /// Identifies scene-related errors that need special handling in the given
    /// set of task errors and handles them.
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

/// Registers all tasks needed for coordinate between systems in the scene in
/// the given task scheduler.
pub fn register_scene_tasks(task_scheduler: &mut AppTaskScheduler) -> Result<()> {
    task_scheduler.register_task(SyncSceneObjectTransforms)?;
    task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
    task_scheduler.register_task(SyncSceneCameraViewTransform)?;
    task_scheduler.register_task(UpdateSceneObjectBoundingSpheres)?;
    task_scheduler.register_task(BufferVisibleModelInstances)?;
    task_scheduler.register_task(SyncLightsInStorage)?;
    task_scheduler.register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
    task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)
}
