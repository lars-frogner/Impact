//! Tasks for coordination between systems in the scene.

use crate::{
    define_task,
    engine::{Engine, tasks::AppTaskScheduler},
    gpu::rendering::tasks::RenderingTag,
    scene::{self, Scene},
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform of
    /// each [`SceneGraph`](crate::scene::SceneGraph) node representing an
    /// entity that also has the [`ReferenceFrameComp`] component so that the
    /// translational, rotational and scaling parts match the origin offset,
    /// position, orientation and scaling.  Also updates any flags for the node
    /// to match the entity's [`SceneEntityFlags`].
    [pub] SyncSceneObjectTransformsAndFlags,
    depends_on = [],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Synchronizing scene graph node transforms and flags"; {
            let ecs_world = engine.ecs_world().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();
            scene::systems::sync_scene_object_transforms_and_flags(&ecs_world, &mut scene_graph);
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the properties (position,
    /// direction, emission, extent and flags) of every light source in the
    /// [`LightStorage`](crate::light::LightStorage).
    [pub] SyncLightsInStorage,
    depends_on = [
        UpdateSceneGroupToWorldTransforms,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Synchronizing lights in storage"; {
            let scene = engine.scene().read().unwrap();
            let ecs_world = engine.ecs_world().read().unwrap();
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
    depends_on = [SyncSceneObjectTransformsAndFlags],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Updating scene object group-to-world transforms"; {
            let scene = engine.scene().read().unwrap();
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
        SyncSceneObjectTransformsAndFlags,
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Synchronizing scene camera view transform"; {
            let scene = engine.scene().read().unwrap();
            if let Some(scene_camera) = scene.scene_camera().write().unwrap().as_mut() {
                scene.scene_graph()
                    .read()
                    .unwrap()
                    .sync_camera_view_transform(scene_camera);

                engine
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
    depends_on = [SyncSceneObjectTransformsAndFlags],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Updating scene object bounding spheres"; {
            let scene = engine.scene().read().unwrap();
            scene.scene_graph()
                .write()
                .unwrap()
                .update_all_bounding_spheres();

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) clears any previously buffered
    /// instance features in the
    /// [`InstanceFeatureManager`](crate::model::InstanceFeatureManager).
    [pub] ClearModelInstanceBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Clearing model instance buffers"; {
            let scene = engine.scene().read().unwrap();
            scene.instance_feature_manager().write().unwrap().clear_buffer_contents();
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
        SyncSceneCameraViewTransform,
        ClearModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Buffering visible model instances"; {
            let scene = engine.scene().read().unwrap();
            let scene_camera = scene.scene_camera().read().unwrap();
            if let Some(scene_camera) = scene_camera.as_ref() {
                scene.scene_graph()
                    .read()
                    .unwrap()
                    .buffer_transforms_of_visible_model_instances(
                        &mut scene.instance_feature_manager().write().unwrap(),
                        scene_camera,
                    );

                engine
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
    /// instances may cast a visible shadows for each omnidirectional light,
    /// bounds the light's cubemap projections to encompass these and buffer
    /// their model to cubemap face space transforms for shadow mapping.
    [pub] BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        SyncLightsInStorage,
        ClearModelInstanceBuffers,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Bounding omnidirectional lights and buffering shadow casting model instances"; {
            if engine.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = engine.scene().read().unwrap();
                let scene_camera = scene.scene_camera().read().unwrap();
                if let Some(scene_camera) = scene_camera.as_ref() {
                    scene.scene_graph()
                        .read()
                        .unwrap()
                        .bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                            &mut scene.light_storage().write().unwrap(),
                            &mut scene.instance_feature_manager().write().unwrap(),
                            scene_camera,
                        );

                    engine
                        .renderer()
                        .read()
                        .unwrap()
                        .declare_render_resources_desynchronized();
                }
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
        ClearModelInstanceBuffers,
        BufferVisibleModelInstances
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        with_trace_logging!("Bounding unidirectional lights and buffering shadow casting model instances"; {
            if engine.renderer().read().unwrap().config().shadow_mapping_enabled {
                let scene = engine.scene().read().unwrap();
                let scene_camera = scene.scene_camera().read().unwrap();
                if let Some(scene_camera) = scene_camera.as_ref() {
                    scene.scene_graph()
                        .read()
                        .unwrap()
                        .bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                            &mut scene.light_storage().write().unwrap(),
                            &mut scene.instance_feature_manager().write().unwrap(),
                            scene_camera,
                        );

                    engine
                        .renderer()
                        .read()
                        .unwrap()
                        .declare_render_resources_desynchronized();
                }
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
    task_scheduler.register_task(SyncSceneObjectTransformsAndFlags)?;
    task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
    task_scheduler.register_task(SyncSceneCameraViewTransform)?;
    task_scheduler.register_task(UpdateSceneObjectBoundingSpheres)?;
    task_scheduler.register_task(ClearModelInstanceBuffers)?;
    task_scheduler.register_task(BufferVisibleModelInstances)?;
    task_scheduler.register_task(SyncLightsInStorage)?;
    task_scheduler.register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
    task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)
}
