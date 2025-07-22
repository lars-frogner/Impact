//! Task definitions, arranged in dependency-consistent order.

use crate::{
    gizmo,
    gpu::rendering::resource::DesynchronizedRenderResources,
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
};
use anyhow::Result;
use impact_scheduling::{define_execution_tag, define_task};

// =============================================================================
// EXECUTION TAGS
// =============================================================================

define_execution_tag!(
    /// Execution tag for user interface tasks.
    [pub] UserInterfaceTag
);

define_execution_tag!(
    /// Execution tag for physics-related tasks.
    [pub] PhysicsTag
);

define_execution_tag!(
    /// Execution tag for rendering-related tasks.
    [pub] RenderingTag
);

// =============================================================================
// USER INTERFACE
// =============================================================================

define_task!(
    /// Handles all UI logic and processes and stores the output.
    ///
    /// Since running the UI logic may change configuration parameters in the
    /// engine, this task must run before other tasks that may depend on those
    /// parameters.
    [pub] ProcessUserInterface,
    depends_on = [],
    execute_on = [UserInterfaceTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Processing user interface", engine, {
            ctx.user_interface().process(engine)
        })
    }
);

// =============================================================================
// INSTANCE BUFFER MANAGEMENT
// =============================================================================

define_task!(
    /// Clears any previously buffered instance features in the
    /// [`InstanceFeatureManager`](crate::model::InstanceFeatureManager).
    [pub] ClearModelInstanceBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Clearing model instance buffers", engine, {
            let scene = engine.scene().read().unwrap();
            scene.instance_feature_manager().write().unwrap().clear_buffer_contents();
            Ok(())
        })
    }
);

// =============================================================================
// SCENE GRAPH, TRANSFORMS AND CULLING
// =============================================================================

define_task!(
    /// Updates the model transform of each [`SceneGraph`](crate::scene::SceneGraph)
    /// node representing an entity that also has the
    /// [`ReferenceFrame`](impact_geometry::ReferenceFrame) component so that the
    /// translational, rotational and scaling parts match the origin offset,
    /// position, orientation and scaling. Also updates any flags for the node
    /// to match the entity's [`SceneEntityFlags`](crate::scene::SceneEntityFlags).
    [pub] SyncSceneObjectTransformsAndFlags,
    depends_on = [ProcessUserInterface],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing scene graph node transforms and flags", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();
            impact_scene::systems::sync_scene_object_transforms_and_flags(&ecs_world, &mut scene_graph);
            Ok(())
        })
    }
);

define_task!(
    /// Updates the group-to-world transforms of all
    /// [`SceneGraph`](crate::scene::SceneGraph) group nodes.
    [pub] UpdateSceneGroupToWorldTransforms,
    depends_on = [SyncSceneObjectTransformsAndFlags],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating scene object group-to-world transforms", engine, {
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
    /// Updates the bounding spheres of all [`SceneGraph`](crate::scene::SceneGraph) nodes.
    [pub] UpdateSceneObjectBoundingSpheres,
    depends_on = [SyncSceneObjectTransformsAndFlags],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating scene object bounding spheres", engine, {
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
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to update the view
    /// transform of the scene camera.
    [pub] SyncSceneCameraViewTransform,
    depends_on = [
        SyncSceneObjectTransformsAndFlags,
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing scene camera view transform", engine, {
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

// =============================================================================
// LIGHT PROCESSING
// =============================================================================

define_task!(
    /// Updates the properties (position, direction, emission, extent and flags)
    /// of every light source in the [`LightStorage`](crate::light::LightStorage).
    [pub] SyncLightsInStorage,
    depends_on = [
        UpdateSceneGroupToWorldTransforms,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing lights in storage", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let mut light_storage = scene.light_storage().write().unwrap();
            impact_scene::systems::sync_lights_in_storage(
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
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to determine which
    /// model instances are visible with the scene camera, update
    /// their model-to-camera space transforms and buffer their
    /// features for rendering.
    [pub] BufferModelInstancesForRendering,
    depends_on = [
        UpdateSceneObjectBoundingSpheres,
        SyncSceneCameraViewTransform,
        ClearModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Buffering visible model instances", engine, {
            let renderer = engine.renderer().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let scene_camera = scene.scene_camera().read().unwrap();
            if let Some(scene_camera) = scene_camera.as_ref() {
                scene.scene_graph()
                    .read()
                    .unwrap()
                    .buffer_model_instances_for_rendering(
                        &mut scene.instance_feature_manager().write().unwrap(),
                        scene_camera,
                        renderer.current_frame_count(),
                    );

                renderer.declare_render_resources_desynchronized();
            }

            Ok(())
        })
    }
);

// =============================================================================
// PHYSICS SIMULATION AND CONTROLLED ENTITIES
// =============================================================================

define_task!(
    /// Updates the orientations and motion of all controlled entities.
    [pub] UpdateControlledEntities,
    depends_on = [
        SyncSceneObjectTransformsAndFlags,
        SyncLightsInStorage
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating controlled entities", engine, {
            engine.update_controlled_entities();
            Ok(())
        })
    }
);

define_task!(
    /// Advances the physics simulation by one time step.
    [pub] AdvanceSimulation,
    depends_on = [
        SyncSceneObjectTransformsAndFlags,
        SyncLightsInStorage,
        UpdateControlledEntities
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Advancing simulation", engine, {
            engine.simulator()
                .write()
                .unwrap()
                .advance_simulation(
                    &engine.scene()
                        .read()
                        .unwrap()
                        .voxel_manager()
                        .read()
                        .unwrap()
                        .object_manager
                );
            Ok(())
        })
    }
);

define_task!(
    /// Updates the [`ReferenceFrame`](impact_geometry::ReferenceFrame) and
    /// [`Motion`](impact_physics::quantities::Motion) components of entities
    /// with the [`DynamicRigidBodyID`](impact_physics::rigid_body::DynamicRigidBodyID)
    /// or [`KinematicRigidBodyID`](impact_physics::rigid_body::KinematicRigidBodyID)
    /// component to match the current state of the rigid body.
    [pub] SyncRigidBodyComponents,
    depends_on = [AdvanceSimulation, ApplyVoxelAbsorption],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing rigid body components", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let simulator = engine.simulator().read().unwrap();
            let rigid_body_manager = simulator.rigid_body_manager().read().unwrap();
            impact_physics::systems::synchronize_rigid_body_components(&ecs_world, &rigid_body_manager);
            Ok(())
        })
    }
);

// =============================================================================
// VOXEL PROCESSING
// =============================================================================

define_task!(
    /// Recomputes invalidated mesh data for all meshed voxel objects.
    [pub] SyncVoxelObjectMeshes,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object meshes", engine, {
            let scene = engine.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();

            let mut desynchronized = false;

            voxel_manager.object_manager.sync_voxel_object_meshes(&mut desynchronized);

            if desynchronized {
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
    /// Applies each voxel absorber to the affected voxel objects.
    [pub] ApplyVoxelAbsorption,
    depends_on = [
        AdvanceSimulation,
        UpdateSceneGroupToWorldTransforms,
        SyncVoxelObjectMeshes
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Applying voxel absorbers", engine, {
            let mut entity_stager = engine.entity_stager().lock().unwrap();
            let ecs_world = engine.ecs_world().read().unwrap();
            let simulator = engine.simulator().read().unwrap();
            let mut rigid_body_manager = simulator.rigid_body_manager().write().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();

            impact_voxel::interaction::systems::apply_absorption(
                &mut entity_stager,
                &ecs_world,
                &scene_graph,
                &mut voxel_manager,
                &mut rigid_body_manager,
                simulator.time_step_duration(),
            );

            Ok(())
        })
    }
);

define_task!(
    /// Updates the [`ModelTransform`](impact_geometry::ModelTransform) component
    /// of each voxel object to match its center of mass.
    [pub] SyncVoxelObjectModelTransforms,
    depends_on = [ApplyVoxelAbsorption],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object model transforms", engine, {
            let mut ecs_world = engine.ecs_world().write().unwrap();
            let scene = engine.scene().read().unwrap();
            let voxel_manager = scene.voxel_manager().read().unwrap();

            impact_voxel::interaction::systems::sync_voxel_object_model_transforms(
                &mut ecs_world,
                &voxel_manager.object_manager,
            );

            Ok(())
        })
    }
);

// =============================================================================
// GIZMO PROCESSING
// =============================================================================

define_task!(
    /// Updates the appropriate gizmo visibility flags for all applicable
    /// entities based on which gizmos have been newly configured to be
    /// globally visible or hidden.
    [pub] UpdateVisibilityFlagsForGizmos,
    depends_on = [ProcessUserInterface],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating visibility flags for gizmos", engine, {
            let mut gizmo_manager = engine.gizmo_manager().write().unwrap();
            gizmo::systems::update_visibility_flags_for_gizmos(&mut gizmo_manager, engine.ecs_world());
            Ok(())
        })
    }
);

define_task!(
    /// Finds entities for which gizmos should be displayed and writes their
    /// model-view transforms to the dedicated buffers for the gizmos.
    [pub] BufferTransformsForGizmos,
    depends_on = [
        UpdateVisibilityFlagsForGizmos,
        ClearModelInstanceBuffers,
        BufferModelInstancesForRendering,
        SyncLightsInStorage,
        AdvanceSimulation
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Buffering transforms for gizmos", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let current_frame_count = engine.renderer().read().unwrap().current_frame_count();
            let gizmo_manager = engine.gizmo_manager().read().unwrap();
            let simulator = engine.simulator().read().unwrap();
            let rigid_body_manager = simulator.rigid_body_manager().read().unwrap();
            let collision_world = simulator.collision_world().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut instance_feature_manager = scene.instance_feature_manager().write().unwrap();
            let voxel_manager = scene.voxel_manager().read().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let light_storage = scene.light_storage().read().unwrap();
            let scene_camera = scene.scene_camera().read().unwrap();

            gizmo::systems::buffer_transforms_for_gizmos(
                &ecs_world,
                &rigid_body_manager,
                &mut instance_feature_manager,
                &gizmo_manager,
                &collision_world,
                &voxel_manager,
                &scene_graph,
                &light_storage,
                scene_camera.as_ref(),
                current_frame_count,
            );
            Ok(())
        })
    }
);

// =============================================================================
// SHADOW MAPPING AND LIGHT CULLING
// =============================================================================

define_task!(
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to determine which model
    /// instances may cast a visible shadows for each omnidirectional light,
    /// bounds the light's cubemap projections to encompass these and buffer
    /// their model to cubemap face space transforms for shadow mapping.
    [pub] BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        SyncLightsInStorage,
        ClearModelInstanceBuffers,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first
        BufferModelInstancesForRendering,
        BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Bounding omnidirectional lights and buffering shadow casting model instances", engine, {
            let shadow_mapping_enabled = engine.renderer().read().unwrap().shadow_mapping_config().enabled;
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
                        shadow_mapping_enabled,
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
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to determine which model
    /// instances may cast a visible shadows for each unidirectional light,
    /// bounds the light's orthographic projection to encompass these and buffer
    /// their model to light transforms for shadow mapping.
    [pub] BoundUnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        SyncLightsInStorage,
        ClearModelInstanceBuffers,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first
        BufferModelInstancesForRendering,
        BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Bounding unidirectional lights and buffering shadow casting model instances", engine, {
            let shadow_mapping_enabled = engine.renderer().read().unwrap().shadow_mapping_config().enabled;
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
                        shadow_mapping_enabled,
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

// =============================================================================
// GPU RESOURCE SYNCHRONIZATION
// =============================================================================

define_task!(
    /// Synchronizes mesh GPU buffers with triangle and line segment meshes.
    [pub] SyncMeshGPUBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing mesh GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_triangle_mesh_buffers_with_triangle_meshes(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .triangle_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    engine
                        .scene()
                        .read()
                        .unwrap()
                        .mesh_repository()
                        .read()
                        .unwrap()
                        .triangle_meshes(),
                );
                DesynchronizedRenderResources::sync_line_segment_mesh_buffers_with_line_segment_meshes(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .line_segment_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    engine
                        .scene()
                        .read()
                        .unwrap()
                        .mesh_repository()
                        .read()
                        .unwrap()
                        .line_segment_meshes(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes camera and skybox GPU resources.
    [pub] SyncMinorResources,
    depends_on = [SyncSceneCameraViewTransform],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing camera and skybox GPU resources", engine, {
            let renderer = engine.renderer().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_camera_buffer_with_scene_camera(
                    renderer.graphics_device(),
                    renderer.bind_group_layout_registry(),
                    render_resource_manager
                        .desynchronized()
                        .camera_buffer_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                    scene.scene_camera().read().unwrap().as_ref(),
                );
                DesynchronizedRenderResources::sync_skybox_resources_with_scene_skybox(
                    renderer.graphics_device(),
                    &engine.assets().read().unwrap(),
                    render_resource_manager
                        .desynchronized()
                        .skybox_resource_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                    scene.skybox().read().unwrap().as_ref(),
                )?;
            }
            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes voxel object GPU buffers.
    [pub] SyncVoxelObjectGPUBuffers,
    depends_on = [SyncVoxelObjectMeshes],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_voxel_resources_with_voxel_manager(
                    renderer.graphics_device(),
                    engine.assets(),
                    renderer.bind_group_layout_registry(),
                    render_resource_manager
                        .desynchronized()
                        .voxel_resource_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    &mut engine
                        .scene()
                        .read()
                        .unwrap()
                        .voxel_manager()
                        .write()
                        .unwrap(),
                )?;
            }
            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes light GPU buffers.
    [pub] SyncLightGPUBuffers,
    depends_on = [
        SyncLightsInStorage,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing light GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                let scene = engine.scene().read().unwrap();
                let light_storage = scene.light_storage().read().unwrap();
                DesynchronizedRenderResources::sync_light_buffers_with_light_storage(
                    renderer.graphics_device(),
                    renderer.bind_group_layout_registry(),
                    render_resource_manager
                        .desynchronized()
                        .light_buffer_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                    &light_storage,
                    renderer.shadow_mapping_config(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes model instance feature GPU buffers.
    [pub] SyncInstanceFeatureBuffers,
    depends_on = [
        BufferModelInstancesForRendering,
        BufferTransformsForGizmos,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!(
            "Synchronizing model instance feature GPU buffers",
            engine,
            {
                let renderer = engine.renderer().read().unwrap();
                let render_resource_manager = renderer.render_resource_manager().read().unwrap();
                if render_resource_manager.is_desynchronized() {
                    DesynchronizedRenderResources::sync_instance_feature_buffers_with_manager(
                        renderer.graphics_device(),
                        render_resource_manager
                            .desynchronized()
                            .instance_feature_buffer_managers
                            .lock()
                            .unwrap()
                            .as_mut(),
                        &mut engine
                            .scene()
                            .read()
                            .unwrap()
                            .instance_feature_manager()
                            .write()
                            .unwrap(),
                    );
                }
                Ok(())
            }
        )
    }
);

define_task!(
    /// Performs any required updates for keeping the engine's render resources
    /// in sync with the source data.
    ///
    /// GPU resources whose source data no longer exists will be removed, and
    /// missing render resources for new source data will be created.
    [pub] SyncRenderResources,
    depends_on = [
        SyncMinorResources,
        SyncMeshGPUBuffers,
        SyncVoxelObjectGPUBuffers,
        SyncLightGPUBuffers,
        SyncInstanceFeatureBuffers
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Completing synchronization of render resources", engine, {
            let renderer = engine.renderer().read().unwrap();
            let mut render_resource_manager = renderer.render_resource_manager().write().unwrap();
            render_resource_manager.declare_synchronized();
            Ok(())
        })
    }
);

// =============================================================================
// RENDER PIPELINE EXECUTION
// =============================================================================

define_task!(
    /// Ensures that all render commands required for rendering the entities
    /// are up to date with the current render resources.
    [pub] SyncRenderCommands,
    depends_on = [SyncRenderResources],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing render commands", engine, {
            let renderer = engine.renderer().read().unwrap();
            let mut shader_manager = renderer.shader_manager().write().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            let mut render_command_manager = renderer.render_command_manager().write().unwrap();
            let scene = engine.scene().read().unwrap();
            let material_library = scene.material_library().read().unwrap();

            render_command_manager.sync_with_render_resources(
                renderer.graphics_device(),
                &mut shader_manager,
                &material_library,
                render_resource_manager.synchronized(),
                renderer.bind_group_layout_registry(),
            )
        })
    }
);

define_task!(
    /// Executes the [`RenderingSystem::render_to_surface`] method.
    [pub] Render,
    depends_on = [SyncRenderCommands],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Rendering", engine, {
            let scene = engine.scene().read().unwrap();
            engine.renderer().write().unwrap().render_to_surface(
                &scene,
                ctx.user_interface(),
            )?;
            engine.save_requested_screenshots()
        })
    }
);

// =============================================================================
// TASK REGISTRATION
// =============================================================================

/// Registers all tasks in the given task scheduler.
///
/// Tasks are registered in functional groups arranged in dependency-consistent
/// order, making the overall execution flow clear and grouping related tasks.
pub fn register_all_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    // User Interface
    task_scheduler.register_task(ProcessUserInterface)?;

    // Instance Buffer Management
    task_scheduler.register_task(ClearModelInstanceBuffers)?;

    // Scene Graph, Transforms and Culling
    task_scheduler.register_task(SyncSceneObjectTransformsAndFlags)?;
    task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
    task_scheduler.register_task(UpdateSceneObjectBoundingSpheres)?;
    task_scheduler.register_task(SyncSceneCameraViewTransform)?;
    task_scheduler.register_task(BufferModelInstancesForRendering)?;

    // Light Processing
    task_scheduler.register_task(SyncLightsInStorage)?;

    // Physics Simulation and Controlled Entities
    task_scheduler.register_task(UpdateControlledEntities)?;
    task_scheduler.register_task(AdvanceSimulation)?;
    task_scheduler.register_task(SyncRigidBodyComponents)?;

    // Voxel Processing
    task_scheduler.register_task(SyncVoxelObjectMeshes)?;
    task_scheduler.register_task(ApplyVoxelAbsorption)?;
    task_scheduler.register_task(SyncVoxelObjectModelTransforms)?;

    // Gizmo Processing
    task_scheduler.register_task(UpdateVisibilityFlagsForGizmos)?;
    task_scheduler.register_task(BufferTransformsForGizmos)?;

    // Shadow Mapping and Light Culling
    task_scheduler.register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
    task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)?;

    // GPU Resource Synchronization
    task_scheduler.register_task(SyncMeshGPUBuffers)?;
    task_scheduler.register_task(SyncMinorResources)?;
    task_scheduler.register_task(SyncVoxelObjectGPUBuffers)?;
    task_scheduler.register_task(SyncLightGPUBuffers)?;
    task_scheduler.register_task(SyncInstanceFeatureBuffers)?;
    task_scheduler.register_task(SyncRenderResources)?;

    // Render Pipeline Execution
    task_scheduler.register_task(SyncRenderCommands)?;
    task_scheduler.register_task(Render)
}
