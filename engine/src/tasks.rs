//! Task definitions, arranged in dependency-consistent order.

use crate::{
    lock_order::{OrderedMutex, OrderedRwLock},
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
};
use anyhow::Result;
use impact_profiling::instrument_task;
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
// APP CALLBACK (for the current frame)
// =============================================================================

define_task!(
    /// Invokes all applicable app callbacks.
    [pub] CallApp,
    depends_on = [],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Calling app", engine.task_timer(), {
            let frame_number = engine.game_loop_controller().oread().iteration();
            engine.app().on_new_frame(frame_number)
        })
    }
);

define_task!(
    /// Handles all queued input events.
    [pub] HandleInputEvents,
    depends_on = [],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Handling input events", engine.task_timer(), {
            engine.handle_queued_input_events()
        })
    }
);

// =============================================================================
// RENDERING (using synchronized GPU resources from the previous frame)
// =============================================================================

define_task!(
    /// Synchronizes the render commands with the current GPU resources (which
    /// represent the state of the previous frame).
    [pub] SyncRenderCommands,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing render commands", engine.task_timer(), {
            let mut renderer = engine.renderer().owrite();
            if renderer.is_initial_frame() {
                // No previous frame to render on the first frame
                return Ok(());
            }
            renderer.synchronize_render_commands()
        })
    }
);

define_task!(
    /// Records and submits all render commands that do not write directly into
    /// the surface texture.
    [pub] RenderBeforeSurface,
    depends_on = [
        // Render commands must be up-to-date with the current resources before
        // we can record them.
        SyncRenderCommands
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Rendering before surface", engine.task_timer(), {
            let mut renderer = engine.renderer().owrite();
            if renderer.is_initial_frame() {
                return Ok(());
            }
            renderer.render_before_surface()
        })
    }
);

define_task!(
    /// Waits for the next surface texture to be ready, before recording and
    /// submitting the final render commands that write into the surface
    /// texture. The surface texture to present (if any) is stored for later
    /// presentation.
    [pub] RenderToSurface,
    depends_on = [RenderBeforeSurface],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();

        let mut renderer = engine.renderer().owrite();

        if renderer.is_initial_frame() {
            return Ok(());
        }

        let (surface_texture_view, surface_texture) = instrument_task!("Obtaining surface", engine.task_timer(), {
            renderer.obtain_surface()
        })?;

        instrument_task!("Rendering to surface", engine.task_timer(), {
            renderer.render_to_surface(
                surface_texture_view,
                surface_texture,
                ctx.user_interface(),
            )
        })
    }
);

define_task!(
    /// Performs minor updates that require the rendering into the surface
    /// texture to be completed.
    [pub] PerformPostRenderingUpdates,
    depends_on = [RenderToSurface],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Performing post-rendering updates", engine.task_timer(), {
            let mut renderer = engine.renderer().owrite();

            if renderer.is_initial_frame() {
                // We have skipped all rendering tasks in the initial frame, so
                // we can signal that the rendering part of the initial frame is
                // over
                renderer.mark_initial_frame_over();
                return Ok(());
            }

            renderer.load_recorded_timing_results()?;

            renderer.downgrade().update_exposure()
        })
    }
);

define_task!(
    /// Captures and saves any screenshots or related textures requested through
    /// the [`ScreenCapturer`]. This must be done before the surface is
    /// presented and the texture is made unavailable.
    [pub] SaveRequestedScreenshots,
    depends_on = [
        // We need the surface texture to be fully rendered to before we can
        // save it as a screenshot.
        RenderToSurface
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Saving requested screenshots", engine.task_timer(), {
            engine.save_requested_screenshots()
        })
    }
);

define_task!(
    /// Presents the rendered surface texture.
    [pub] PresentSurface,
    depends_on = [
        RenderToSurface,
        // Presenting makes the surface texture unavailable, so screenshots must
        // be saved before we present.
        SaveRequestedScreenshots
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Presenting surface", engine.task_timer(), {
            engine.renderer().owrite().present();
            Ok(())
        })
    }
);

// =============================================================================
// COMMANDS (affecting the current frame)
// =============================================================================

define_task!(
    /// Executes all the current commands in the command queues except rendering
    /// and capturing commands.
    ///
    /// Since this may change configuration parameters in the engine, this task
    /// must run before other tasks that may depend on those parameters.
    [pub] ApplyEngineCommands,
    depends_on = [CallApp, HandleInputEvents],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Executing enqueued engine commands", engine.task_timer(), {
            engine.execute_enqueued_scene_commands()?;
            engine.execute_enqueued_control_commands()?;
            engine.execute_enqueued_physics_commands()?;
            engine.execute_enqueued_physics_admin_commands()?;
            engine.execute_enqueued_control_admin_commands()?;
            engine.execute_enqueued_instrumentation_admin_commands()?;
            engine.execute_enqueued_game_loop_admin_commands()?;
            engine.execute_enqueued_gizmo_admin_commands()?;
            engine.execute_enqueued_system_admin_commands()?;
            Ok(())
        })
    }
);

define_task!(
    /// Executes all the current rendering and capturing commands in the queue.
    [pub] ApplyRenderCommands,
    depends_on = [
        CallApp,
        HandleInputEvents,
        // We must wait for the rendering of the previous frame to be completed
        // before we touch the rendering configuration or request a frame
        // capture.
        SaveRequestedScreenshots
    ],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Executing enqueued rendering commands", engine.task_timer(), {
            engine.execute_enqueued_rendering_admin_commands()?;
            engine.execute_enqueued_capture_admin_commands()
        })
    }
);

// =============================================================================
// ENTITY RULES (based on state from previous frame)
// =============================================================================

define_task!(
    /// Stages entities failing their lifetime conditions for removal.
    [pub] HandleDistanceTriggeredEntityRules,
    depends_on = [CallApp, HandleInputEvents],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Handling distance triggered entity rules", engine.task_timer(), {
            let mut entity_stager = engine.entity_stager().olock();
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let mut scene_graph = scene.scene_graph().owrite();
            impact_scene::systems::handle_distance_triggered_rules_for_entities(
                &mut entity_stager,
                &ecs_world,
                &mut scene_graph,
            );
            Ok(())
        })
    }
);

// =============================================================================
// USER INTERFACE
// =============================================================================

define_task!(
    /// Handles all UI logic and processes and stores the output. Since commands
    /// from the UI are queued until executed by `ApplyEngineCommands`, this
    /// task can be performed completely independently.
    [pub] ProcessUserInterface,
    depends_on = [],
    execute_on = [UserInterfaceTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Processing user interface", engine.task_timer(), {
            ctx.user_interface().process()
        })
    }
);

// =============================================================================
// STAGED ENTITIES (for current frame)
// =============================================================================

define_task!(
    /// Creates entities staged for creation and removes entities staged for
    /// removal.
    [pub] HandleStagedEntities,
    depends_on = [
        CallApp,
        HandleInputEvents,
        ApplyEngineCommands,
        HandleDistanceTriggeredEntityRules
    ],
    execute_on = [PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Handling staged entities", engine.task_timer(), {
            engine.handle_staged_entities()
        })
    }
);

// =============================================================================
// VOXEL PROCESSING (for current frame)
// =============================================================================

define_task!(
    /// Updates the [`ModelTransform`](impact_geometry::ModelTransform) component
    /// of each voxel object to match its center of mass.
    [pub] SyncVoxelObjectModelTransforms,
    depends_on = [
        // We need exclusive access to the ECS world, so better to wait until
        // staged entities have been processed.
        HandleStagedEntities
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing voxel object model transforms", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let voxel_manager = scene.voxel_manager().oread();

            impact_voxel::interaction::systems::sync_voxel_object_model_transforms(
                &ecs_world,
                voxel_manager.object_manager(),
            );

            Ok(())
        })
    }
);

define_task!(
    /// Updates the collidables of voxel objects to reflect their current
    /// [`ModelTransform`](impact_geometry::ModelTransform).
    [pub] SyncVoxelObjectCollidables,
    depends_on = [SyncVoxelObjectModelTransforms],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing voxel object collidables", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let simulator = engine.simulator().oread();
            let mut collision_world = simulator.collision_world().owrite();

            impact_voxel::collidable::systems::sync_voxel_object_collidables(
                &ecs_world,
                &mut collision_world,
            );

            Ok(())
        })
    }
);

define_task!(
    /// Recomputes invalidated mesh data for all meshed voxel objects.
    [pub] UpdateVoxelObjectMeshes,
    depends_on = [
        // If voxel objects were staged for removal, we don't want to waste time
        // syncing their meshes. (Created objects already have in-sync meshes,
        // so they are not relevant here.)
        HandleStagedEntities,
        // We need exclusive access to the voxel object manager, so better to
        // let `SyncVoxelObjectModelTransforms` go first so that we don't block
        // `SyncVoxelObjectCollidables`.
        SyncVoxelObjectModelTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Updating voxel object meshes", engine.task_timer(), {
            let scene = engine.scene().oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
            voxel_manager.object_manager_mut().sync_voxel_object_meshes();
            Ok(())
        })
    }
);

// =============================================================================
// CONTROLLED ENTITIES (updates to state for current frame)
// =============================================================================

define_task!(
    /// Updates the linear and angular velocities of all controlled entities.
    [pub] UpdateControlledEntityMotion,
    depends_on = [
        // We want to include entities staged for creation this frame.
        HandleStagedEntities
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Updating controlled entities", engine.task_timer(), {
            engine.update_controlled_entity_motion();
            Ok(())
        })
    }
);

// =============================================================================
// PHYSICS SIMULATION (updates to state for current frame)
// =============================================================================

define_task!(
    /// Advances the physics simulation by one time step.
    ///
    /// The resulting state represents the state for the current frame.
    [pub] AdvanceSimulation,
    depends_on = [
        // Creating or removing entities may modify physics state, so we need
        // this to be completed before we advance the simulation.
        HandleStagedEntities,
        // We want the post-control-update velocities as the basis for the
        // simulation step.
        UpdateControlledEntityMotion,
        // We need the up-to-date collidables.
        SyncVoxelObjectCollidables
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        let scene =  engine.scene().oread();
        let voxel_manager = scene.voxel_manager().oread();
        let intersection_manager = scene.intersection_manager().oread();
        let mut simulator = engine.simulator().owrite();
        simulator.advance_simulation(
            engine.task_timer(),
            voxel_manager.object_manager(),
            &intersection_manager,
        );
        Ok(())
    }
);

define_task!(
    /// Updates the [`ReferenceFrame`](impact_geometry::ReferenceFrame) and
    /// [`Motion`](impact_physics::quantities::Motion) components of entities
    /// with the [`DynamicRigidBodyID`](impact_physics::rigid_body::DynamicRigidBodyID)
    /// or [`KinematicRigidBodyID`](impact_physics::rigid_body::KinematicRigidBodyID)
    /// component to match the current state of the rigid body.
    [pub] SyncRigidBodyComponents,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing rigid body components", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let simulator = engine.simulator().oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
            impact_physics::systems::synchronize_rigid_body_components(&ecs_world, &rigid_body_manager);
            Ok(())
        })
    }
);

// =============================================================================
// INSTANCE BUFFER MANAGEMENT (for current frame)
// =============================================================================

define_task!(
    /// Clears any previously buffered instance features in the
    /// [`ModelInstanceManager`](crate::model::ModelInstanceManager).
    [pub] ClearModelInstanceBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Clearing model instance buffers", engine.task_timer(), {
            let scene = engine.scene().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            model_instance_manager.clear_buffer_contents();
            Ok(())
        })
    }
);

// =============================================================================
// SCENE GRAPH, TRANSFORMS AND CULLING (for current frame)
// =============================================================================

define_task!(
    /// Updates the model transform of each [`SceneGraph`](crate::scene::SceneGraph)
    /// node representing an entity that also has the
    /// [`ReferenceFrame`](impact_geometry::ReferenceFrame) component so that the
    /// translational, rotational and scaling parts match the origin offset,
    /// position, orientation and scaling. Also updates any flags for the node
    /// to match the entity's [`SceneEntityFlags`](crate::scene::SceneEntityFlags).
    /// In addition, the bounding spheres of nodes representing voxel objects are
    /// updated to match the current bounding spheres of the objects.
    [pub] SyncSceneGraphNodeProperties,
    depends_on = [
        // We want to include the changes due to entity creation and removal.
        HandleStagedEntities,
        // We also want to include the updated rigid body state for this frame.
        SyncRigidBodyComponents
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing scene graph node properties", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let voxel_manager = scene.voxel_manager().oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();

            impact_scene::systems::sync_scene_object_transforms_and_flags(&ecs_world, &mut scene_graph);

            impact_voxel::interaction::systems::sync_voxel_object_bounding_volumes(
                &ecs_world,
                voxel_manager.object_manager(),
                &mut intersection_manager.bounding_volume_manager,
            );
            Ok(())
        })
    }
);

define_task!(
    /// Updates the group-to-world transforms of all
    /// [`SceneGraph`](crate::scene::SceneGraph) group nodes.
    [pub] UpdateSceneGroupToWorldTransforms,
    depends_on = [
        // We depend on the updated group-to-parent transforms.
        SyncSceneGraphNodeProperties
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Updating scene object group-to-world transforms", engine.task_timer(), {
            let scene = engine.scene().oread();
            let mut scene_graph = scene.scene_graph().owrite();
            scene_graph.update_all_group_to_root_transforms();
            Ok(())
        })
    }
);

define_task!(
    /// Adds the world-space bounding volumes of the appropriate entities to the
    /// bounding volume hierarchy.
    [pub] AddBoundingVolumesToHierarchy,
    depends_on = [
        // We depend on the updated group-to-world transforms.
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Adding bounding volumes to hierarchy", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
            let scene_graph = scene.scene_graph().oread();

            intersection_manager.reset_bounding_volume_hierarchy();

            impact_scene::systems::add_bounding_volumes_to_hierarchy(
                &ecs_world,
                &mut intersection_manager,
                &scene_graph,
            );
            Ok(())
        })
    }
);

define_task!(
    /// Builds the bounding volume hierarchy for the added bounding volumes.
    [pub] BuildBoundingVolumeHierarchy,
    depends_on = [
        AddBoundingVolumesToHierarchy
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Building bounding volume hierarchy", engine.task_timer(), {
            let scene = engine.scene().oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
            intersection_manager.build_bounding_volume_hierarchy();
            Ok(())
        })
    }
);

define_task!(
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to update the view
    /// transform of the scene camera.
    [pub] SyncSceneCameraViewTransform,
    depends_on = [
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing scene camera view transform", engine.task_timer(), {
            let scene = engine.scene().oread();
            let mut camera_manager = scene.camera_manager().owrite();
            if let Some(camera) = camera_manager.active_camera_mut() {
                let scene_graph = scene.scene_graph().oread();
                scene_graph.sync_camera_view_transform(camera);
            }
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
        // We need the current view transform to compute model-to-camera
        // transforms.
        SyncSceneCameraViewTransform,
        // We need the BVH for view frustum culling.
        BuildBoundingVolumeHierarchy,
        // The buffers must have been cleared from the previous frame before we
        // write into them.
        ClearModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Buffering model instances for rendering", engine.task_timer(), {
            let current_frame_number = engine.game_loop_controller().oread().iteration() as u32;
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(camera) = camera_manager.active_camera() {
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let intersection_manager = scene.intersection_manager().oread();
                let scene_graph = scene.scene_graph().oread();

                scene_graph.buffer_model_instances_for_rendering(
                    &resource_manager.materials,
                    &mut model_instance_manager,
                    &intersection_manager,
                    camera,
                    current_frame_number,
                );
            }

            Ok(())
        })
    }
);

// =============================================================================
// LIGHT PROCESSING (for current frame)
// =============================================================================

define_task!(
    /// Updates the properties (position, direction, emission, extent and flags)
    /// of every light source in the [`LightManager`](crate::light::LightManager).
    [pub] SyncLights,
    depends_on = [
        // We need the current view transform for computing camera-space
        // positions and directions.
        SyncSceneCameraViewTransform,
        // For lights that have a parent group node in the scene graph, we need
        // the group-to-world transform to get the full light-to-camera
        // transform.
        UpdateSceneGroupToWorldTransforms,
        // Newly added or removed light entities should be included. (This is in
        // practice covered by the other dependencies).
        HandleStagedEntities
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing lights in storage", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
            let scene_graph = scene.scene_graph().oread();
            impact_scene::systems::sync_lights_in_storage(
                &ecs_world,
                &mut light_manager,
                &scene_graph,
                &view_transform,
            );
            Ok(())
        })
    }
);

// =============================================================================
// LIGHT CULLING AND SHADOW MAPPING (for current frame)
// =============================================================================

define_task!(
    /// Determines which model instances may cast a visible shadow for each
    /// omnidirectional light, bounds the light's cubemap projections to
    /// encompass these and buffer their model to cubemap face space transforms
    /// for shadow mapping.
    [pub] BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        // We need the up-to-date light state for this.
        SyncLights,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first.
        BufferModelInstancesForRendering,
        BuildBoundingVolumeHierarchy
        // Since gizmo models can't cast shadows, we luckily don't need this
        // dependency (which would create a cycle).
        // BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Bounding omnidirectional lights and buffering shadow casting model instances", engine.task_timer(), {
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(camera) = camera_manager.active_camera() {
                let mut light_manager = scene.light_manager().owrite();
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let intersection_manager = scene.intersection_manager().oread();
                let scene_graph = scene.scene_graph().oread();
                let shadow_mapping_enabled = engine.renderer().oread().shadow_mapping_config().enabled;

                scene_graph
                    .bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut light_manager,
                        &mut model_instance_manager,
                        &intersection_manager,
                        camera,
                        shadow_mapping_enabled,
                    );
            }
            Ok(())
        })
    }
);

define_task!(
    /// Determines which model instances may cast a visible shadow for each
    /// unidirectional light, bounds the light's orthographic projection to
    /// encompass these and buffer their model to light transforms for shadow
    /// mapping.
    [pub] BoundUnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        // We need to up-to-date light state for this.
        SyncLights,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first
        BufferModelInstancesForRendering,
        BuildBoundingVolumeHierarchy
        // Since gizmo models can't cast shadows, we luckily don't need this
        // dependency (which would create a cycle).
        // BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Bounding unidirectional lights and buffering shadow casting model instances", engine.task_timer(), {
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(camera) = camera_manager.active_camera() {
                let mut light_manager = scene.light_manager().owrite();
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let intersection_manager = scene.intersection_manager().oread();
                let scene_graph = scene.scene_graph().oread();
                let shadow_mapping_enabled = engine.renderer().oread().shadow_mapping_config().enabled;

                scene_graph
                    .bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut light_manager,
                        &mut model_instance_manager,
                        &intersection_manager,
                        camera,
                        shadow_mapping_enabled,
                    );
            }
            Ok(())
        })
    }
);

// =============================================================================
// GIZMO PROCESSING (for current frame)
// =============================================================================

define_task!(
    /// Updates the appropriate gizmo visibility flags for all applicable
    /// entities based on which gizmos have been newly configured to be
    /// globally visible or hidden.
    [pub] UpdateVisibilityFlagsForGizmos,
    depends_on = [ApplyEngineCommands],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Updating visibility flags for gizmos", engine.task_timer(), {
            let ecs_world = engine.ecs_world().oread();
            let mut gizmo_manager = engine.gizmo_manager().owrite();
            impact_gizmo::systems::update_visibility_flags_for_gizmos(&mut gizmo_manager, &ecs_world);
            Ok(())
        })
    }
);

define_task!(
    /// Finds entities for which gizmos should be displayed and writes their
    /// model-view transforms to the dedicated buffers for the gizmos.
    [pub] BufferTransformsForGizmos,
    depends_on = [
        // This is where we use the visibility flags.
        UpdateVisibilityFlagsForGizmos,
        // Certain gizmos need the current physics state.
        SyncRigidBodyComponents,
        // Certain gizmos need the current model-view transforms of their
        // associated model instances.
        BufferModelInstancesForRendering,
        // Certain gizmos need the current light state.
        SyncLights,
        // TODO: Certain gizmos need light properties that are modified when the
        // lights are bound to the scene. But we have to buffer the gizmo
        // transforms before buffering model-to-light transforms, since the
        // former have to come before the latter in the buffers. Ideally, the
        // bounding and buffering should be split into separate tasks, but since
        // gizmos are only a dev tool we're fine with those visualizations
        // lagging by one frame for now.
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Buffering transforms for gizmos", engine.task_timer(), {
            let current_frame_number = engine.game_loop_controller().oread().iteration() as u32;
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            let light_manager = scene.light_manager().oread();
            let voxel_manager = scene.voxel_manager().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let intersection_manager = scene.intersection_manager().oread();
            let scene_graph = scene.scene_graph().oread();
            let simulator = engine.simulator().oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
            let anchor_manager = simulator.anchor_manager().oread();
            let collision_world = simulator.collision_world().oread();
            let gizmo_manager = engine.gizmo_manager().oread();

            impact_gizmo::systems::buffer_transforms_for_gizmos(
                &mut model_instance_manager,
                &ecs_world,
                &camera_manager,
                &light_manager,
                voxel_manager.object_manager(),
                &intersection_manager,
                &scene_graph,
                &rigid_body_manager,
                &anchor_manager,
                &collision_world,
                &gizmo_manager,
                current_frame_number,
            );
            Ok(())
        })
    }
);

// =============================================================================
// GPU RESOURCE SYNCHRONIZATION (of updates from current frame, rendered next frame)
// =============================================================================

define_task!(
    /// Synchronizes GPU resources for textures.
    [pub] SyncTextureGPUResources,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing texture GPU resources", engine.task_timer(), {
            engine.sync_texture_gpu_resources()
        })
    }
);

define_task!(
    /// Synchronizes mesh GPU resources for triangle and line segment meshes.
    [pub] SyncMeshGPUResources,
    depends_on = [
        // The application may create or remove entities, which can affect mesh
        // resources. Same for staged entities.
        CallApp,
        HandleInputEvents,
        HandleStagedEntities
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing mesh GPU resources", engine.task_timer(), {
            engine.sync_mesh_gpu_resources()
        })
    }
);

define_task!(
    /// Synchronizes GPU resources for materials.
    [pub] SyncMaterialGPUResources,
    depends_on = [
        // The application may create or remove entities, which can affect
        // material resources. Same for staged entities.
        CallApp,
        HandleInputEvents,
        HandleStagedEntities,
        // Some materials need access to the current textures.
        SyncTextureGPUResources
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing material GPU resources", engine.task_timer(), {
            engine.sync_material_gpu_resources()
        })
    }
);

define_task!(
    /// Synchronizes miscellaneous GPU resources.
    [pub] SyncMiscGPUResources,
    depends_on = [
        // Both the skybox and voxel materials need access to the current
        // textures.
        SyncTextureGPUResources
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing miscellaneous GPU resources", engine.task_timer(), {
            engine.sync_misc_gpu_resources()
        })
    }
);

define_task!(
    /// Records and submits commands for synchronizing dynamic GPU resources
    /// (resources that benefit from a staging belt).
    [pub] SyncDynamicGPUResources,
    depends_on = [
        // Updating the camera uniform requires the current view transform.
        SyncSceneCameraViewTransform,
        // Light uniforms must be synced with current light state.
        SyncLights,
        // The current voxel meshes must be synced with various GPU buffers.
        UpdateVoxelObjectMeshes,
        // The current model instance feature buffers must be copied over to
        // their GPU-side counterparts.
        BufferModelInstancesForRendering,
        BufferTransformsForGizmos,
        // These task affect both the light uniforms and the instance feature
        // buffers.
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances,
        // We need to have the up-to-date rendering configuration at this point.
        ApplyRenderCommands
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Synchronizing dynamic GPU resources", engine.task_timer(), {
            engine.sync_dynamic_gpu_resources()
        })
    }
);

// =============================================================================
// VOXEL PROCESSING (for next frame)
// =============================================================================

define_task!(
    /// Applies each voxel absorber to the affected voxel objects.
    ///
    /// The changes will be visible when the next frame is rendered, not the
    /// current one.
    [pub] ApplyVoxelAbsorption,
    depends_on = [
        // Mesh modifications due to absorption this frame should be visible
        // next frame, so we must ensure that the mesh updates for this frame
        // are already completed before modifying the voxel objects.
        UpdateVoxelObjectMeshes,
        // Absorption will modify the objects' rigid bodies, but this should not
        // be visible until the next frame. Also, we want to operate on the
        // rigid body states for the current frame, which is after the
        // simulation step.
        SyncRigidBodyComponents,
        // For voxel absorbers that have a parent group node in the scene graph,
        // we need the group-to-world transform to get their full model-to-world
        // transform.
        UpdateSceneGroupToWorldTransforms,
        // Newly added or removed voxel object entities should be included.
        // (This is in practice covered by the other dependencies).
        HandleStagedEntities
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_task!("Applying voxel absorbers", engine.task_timer(), {
            let mut entity_id_manager = engine.entity_id_manager().olock();
            let mut entity_stager = engine.entity_stager().olock();
            let ecs_world = engine.ecs_world().oread();
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
            let intersection_manager = scene.intersection_manager().oread();
            let scene_graph = scene.scene_graph().oread();
            let simulator = engine.simulator().oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let force_generator_manager = simulator.force_generator_manager().oread();
            let collision_world = simulator.collision_world().oread();

            impact_voxel::interaction::systems::apply_absorption(
                engine.component_metadata_registry(),
                &mut entity_id_manager,
                &mut entity_stager,
                &ecs_world,
                &scene_graph,
                &mut voxel_manager,
                &resource_manager.voxel_types,
                &intersection_manager,
                &mut rigid_body_manager,
                &mut anchor_manager,
                &force_generator_manager,
                &collision_world,
            );

            Ok(())
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
    // APP CALLBACK
    task_scheduler.register_task(CallApp)?;
    task_scheduler.register_task(HandleInputEvents)?;

    // RENDERING (using synchronized GPU resources from the previous frame)
    task_scheduler.register_task(SyncRenderCommands)?;
    task_scheduler.register_task(RenderBeforeSurface)?;
    task_scheduler.register_task(RenderToSurface)?;
    task_scheduler.register_task(PerformPostRenderingUpdates)?;
    task_scheduler.register_task(SaveRequestedScreenshots)?;
    task_scheduler.register_task(PresentSurface)?;

    // COMMANDS (affecting the current frame)
    task_scheduler.register_task(ApplyEngineCommands)?;
    task_scheduler.register_task(ApplyRenderCommands)?;

    // ENTITY RULES (based on state from previous frame)
    task_scheduler.register_task(HandleDistanceTriggeredEntityRules)?;

    // USER INTERFACE
    task_scheduler.register_task(ProcessUserInterface)?;

    // STAGED ENTITIES (for current frame)
    task_scheduler.register_task(HandleStagedEntities)?;

    // VOXEL PROCESSING (for current frame)
    task_scheduler.register_task(SyncVoxelObjectModelTransforms)?;
    task_scheduler.register_task(SyncVoxelObjectCollidables)?;
    task_scheduler.register_task(UpdateVoxelObjectMeshes)?;

    // CONTROLLED ENTITIES (updates to state for current frame)
    task_scheduler.register_task(UpdateControlledEntityMotion)?;

    // PHYSICS SIMULATION (updates to state for current frame)
    task_scheduler.register_task(AdvanceSimulation)?;
    task_scheduler.register_task(SyncRigidBodyComponents)?;

    // INSTANCE BUFFER MANAGEMENT (for current frame)
    task_scheduler.register_task(ClearModelInstanceBuffers)?;

    // SCENE GRAPH, TRANSFORMS AND CULLING (for current frame)
    task_scheduler.register_task(SyncSceneGraphNodeProperties)?;
    task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
    task_scheduler.register_task(AddBoundingVolumesToHierarchy)?;
    task_scheduler.register_task(BuildBoundingVolumeHierarchy)?;
    task_scheduler.register_task(SyncSceneCameraViewTransform)?;
    task_scheduler.register_task(BufferModelInstancesForRendering)?;

    // LIGHT PROCESSING (for current frame)
    task_scheduler.register_task(SyncLights)?;

    // LIGHT CULLING AND SHADOW MAPPING (for current frame)
    task_scheduler.register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
    task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)?;

    // GIZMO PROCESSING (for current frame)
    task_scheduler.register_task(UpdateVisibilityFlagsForGizmos)?;
    task_scheduler.register_task(BufferTransformsForGizmos)?;

    // GPU RESOURCE SYNCHRONIZATION (of updates from current frame, rendered next frame)
    task_scheduler.register_task(SyncTextureGPUResources)?;
    task_scheduler.register_task(SyncMeshGPUResources)?;
    task_scheduler.register_task(SyncMaterialGPUResources)?;
    task_scheduler.register_task(SyncMiscGPUResources)?;
    task_scheduler.register_task(SyncDynamicGPUResources)?;

    // VOXEL PROCESSING (for next frame)
    task_scheduler.register_task(ApplyVoxelAbsorption)
}
