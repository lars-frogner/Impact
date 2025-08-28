//! Task definitions, arranged in dependency-consistent order.

use crate::{
    alloc::TaskArenas,
    gizmo,
    lock_order::{OrderedMutex, OrderedRwLock},
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
// APP CALLBACK (for the current frame)
// =============================================================================

define_task!(
    /// Invokes all applicable app callbacks.
    [pub] CallApp,
    depends_on = [],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Calling app", engine, {
            let frame_number = engine.game_loop_controller().oread().iteration();
            engine.app().on_new_frame(engine, frame_number)
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
        instrument_engine_task!("Synchronizing render commands", engine, {
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
        instrument_engine_task!("Rendering before surface", engine, {
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

        let (surface_texture_view, surface_texture) = instrument_engine_task!("Obtaining surface", engine, {
            renderer.obtain_surface()
        })?;

        instrument_engine_task!("Rendering to surface", engine, {
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
        instrument_engine_task!("Performing post-rendering updates", engine, {
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
        instrument_engine_task!("Saving requested screenshots", engine, {
            TaskArenas::with(|arena| {
                engine.save_requested_screenshots(arena)
            })
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
        instrument_engine_task!("Presenting surface", engine, {
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
    depends_on = [CallApp],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Executing enqueued engine commands", engine, {
            engine.execute_enqueued_scene_commands()?;
            engine.execute_enqueued_controller_commands()?;
            engine.execute_enqueued_physics_commands()?;
            engine.execute_enqueued_control_commands()?;
            engine.execute_enqueued_instrumentation_commands()?;
            engine.execute_enqueued_game_loop_commands()?;
            engine.execute_enqueued_gizmo_commands()?;
            engine.execute_enqueued_system_commands()?;
            Ok(())
        })
    }
);

define_task!(
    /// Executes all the current rendering and capturing commands in the queue.
    [pub] ApplyRenderCommands,
    depends_on = [
        CallApp,
        // We must wait for the rendering of the previous frame to be completed
        // before we touch the rendering configuration or request a frame
        // capture.
        SaveRequestedScreenshots
    ],
    execute_on = [UserInterfaceTag, PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Executing enqueued rendering commands", engine, {
            engine.execute_enqueued_rendering_commands()?;
            engine.execute_enqueued_capture_commands()
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
        instrument_engine_task!("Processing user interface", engine, {
            ctx.user_interface().process(engine)
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
        ApplyEngineCommands
    ],
    execute_on = [PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Handling staged entities", engine, {
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
        instrument_engine_task!("Synchronizing voxel object model transforms", engine, {
            let mut ecs_world = engine.ecs_world().owrite();
            let scene = engine.scene().oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();

            impact_voxel::interaction::systems::sync_voxel_object_model_transforms(
                &mut ecs_world,
                &voxel_object_manager,
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
        instrument_engine_task!("Synchronizing voxel object collidables", engine, {
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
        instrument_engine_task!("Updating voxel object meshes", engine, {
            let scene = engine.scene().oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            voxel_object_manager.sync_voxel_object_meshes();
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
        instrument_engine_task!("Updating controlled entities", engine, {
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
        instrument_engine_task!("Advancing simulation", engine, {
            let scene =  engine.scene().oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();
            let mut simulator = engine.simulator().owrite();
            simulator.advance_simulation(&voxel_object_manager);
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
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing rigid body components", engine, {
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
        instrument_engine_task!("Clearing model instance buffers", engine, {
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
    [pub] SyncSceneObjectTransformsAndFlags,
    depends_on = [
        // We want to include the changes due to entity creation and removal.
        HandleStagedEntities,
        // We also want to include the updated rigid body state for this frame.
        SyncRigidBodyComponents
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing scene graph node transforms and flags", engine, {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let mut scene_graph = scene.scene_graph().owrite();
            impact_scene::systems::sync_scene_object_transforms_and_flags(&ecs_world, &mut scene_graph);
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
        SyncSceneObjectTransformsAndFlags
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating scene object group-to-world transforms", engine, {
            let scene = engine.scene().oread();
            let mut scene_graph = scene.scene_graph().owrite();
            TaskArenas::with(|arena| {
                scene_graph.update_all_group_to_root_transforms(arena);
            });
            Ok(())
        })
    }
);

define_task!(
    /// Updates the bounding spheres of all
    /// [`SceneGraph`](crate::scene::SceneGraph) nodes.
    [pub] UpdateSceneObjectBoundingSpheres,
    depends_on = [
        // This is non strictly a dependency (we only use the group-to-parent
        // transforms), but there is no point in trying to do both at the same
        // time, since both need write access to the scene graph.
        UpdateSceneGroupToWorldTransforms
        // This is the actual dependency.
        // SyncSceneObjectTransformsAndFlags
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating scene object bounding spheres", engine, {
            let scene = engine.scene().oread();
            let mut scene_graph = scene.scene_graph().owrite();
            TaskArenas::with(|arena| {
                scene_graph.update_all_bounding_spheres(arena);
            });
            Ok(())
        })
    }
);

define_task!(
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to update the view
    /// transform of the scene camera.
    [pub] SyncSceneCameraViewTransform,
    depends_on = [
        // This is non strictly a dependency (we don't use the bounding
        // spheres), but there is no point in trying to do both at the same
        // time, since updating bounding spheres requires exclusive accesss to
        // the scene graph.
        UpdateSceneObjectBoundingSpheres
        // This is the actual dependency.
        // UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing scene camera view transform", engine, {
            let scene = engine.scene().oread();
            let mut camera_manager = scene.camera_manager().owrite();
            if let Some(scene_camera) = camera_manager.active_camera_mut() {
                let scene_graph = scene.scene_graph().oread();
                scene_graph.sync_camera_view_transform(scene_camera);
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
        // We need the bounding spheres for view frustum culling.
        UpdateSceneObjectBoundingSpheres,
        // The buffers must have been cleared from the previous frame before we
        // write into them.
        ClearModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Buffering model instances for rendering", engine, {
            let current_frame_number = engine.game_loop_controller().oread().iteration() as u32;
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(scene_camera) = camera_manager.active_camera() {
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let scene_graph = scene.scene_graph().oread();

                scene_graph.buffer_model_instances_for_rendering(
                    &resource_manager.materials,
                    &mut model_instance_manager,
                    scene_camera,
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
        instrument_engine_task!("Synchronizing lights in storage", engine, {
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
    /// Uses the [`SceneGraph`](crate::scene::SceneGraph) to determine which model
    /// instances may cast a visible shadows for each omnidirectional light,
    /// bounds the light's cubemap projections to encompass these and buffer
    /// their model to cubemap face space transforms for shadow mapping.
    [pub] BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
    depends_on = [
        // We need to up-to-date light state for this.
        SyncLights,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first.
        BufferModelInstancesForRendering
        // Since gizmo models can't cast shadows, we luckily don't need this
        // dependency (which would create a cycle).
        // BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Bounding omnidirectional lights and buffering shadow casting model instances", engine, {
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(scene_camera) = camera_manager.active_camera() {
                let mut light_manager = scene.light_manager().owrite();
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let scene_graph = scene.scene_graph().oread();
                let shadow_mapping_enabled = engine.renderer().oread().shadow_mapping_config().enabled;

                scene_graph
                    .bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut light_manager,
                        &mut model_instance_manager,
                        scene_camera,
                        shadow_mapping_enabled,
                    );
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
        // We need to up-to-date light state for this.
        SyncLights,
        // The current task begins new ranges in the instance feature buffers,
        // so all tasks writing to the initial range have to be completed first
        BufferModelInstancesForRendering
        // Since gizmo models can't cast shadows, we luckily don't need this
        // dependency (which would create a cycle).
        // BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Bounding unidirectional lights and buffering shadow casting model instances", engine, {
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            if let Some(scene_camera) = camera_manager.active_camera() {
                let mut light_manager = scene.light_manager().owrite();
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let scene_graph = scene.scene_graph().oread();
                let shadow_mapping_enabled = engine.renderer().oread().shadow_mapping_config().enabled;

                scene_graph
                    .bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut light_manager,
                        &mut model_instance_manager,
                        scene_camera,
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
        instrument_engine_task!("Updating visibility flags for gizmos", engine, {
            let ecs_world = engine.ecs_world().oread();
            let mut gizmo_manager = engine.gizmo_manager().owrite();
            gizmo::systems::update_visibility_flags_for_gizmos(&mut gizmo_manager, &ecs_world);
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
        instrument_engine_task!("Buffering transforms for gizmos", engine, {
            let current_frame_number = engine.game_loop_controller().oread().iteration() as u32;
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            let light_manager = scene.light_manager().oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let scene_graph = scene.scene_graph().oread();
            let simulator = engine.simulator().oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
            let anchor_manager = simulator.anchor_manager().oread();
            let collision_world = simulator.collision_world().oread();
            let gizmo_manager = engine.gizmo_manager().oread();

            gizmo::systems::buffer_transforms_for_gizmos(
                &mut model_instance_manager,
                &ecs_world,
                &camera_manager,
                &light_manager,
                &voxel_object_manager,
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
        instrument_engine_task!("Synchronizing texture GPU resources", engine, {
            let resource_manager = engine.resource_manager().oread();
            let renderer = engine.renderer().oread();
            let mut render_resource_manager = renderer.render_resource_manager().owrite();
            let render_resource_manager = &mut **render_resource_manager;

            impact_resource::gpu::sync_immutable_gpu_resources(
                &(
                    engine.graphics_device(),
                    renderer.mipmapper_generator().as_ref(),
                ),
                &resource_manager.textures,
                &mut render_resource_manager.textures,
            )?;

            impact_resource::gpu::sync_immutable_gpu_resources(
                engine.graphics_device(),
                &resource_manager.samplers,
                &mut render_resource_manager.samplers,
            )?;

            impact_resource::gpu::sync_immutable_gpu_resources(
                &(
                    engine.graphics_device(),
                    renderer.bind_group_layout_registry(),
                    &render_resource_manager.textures,
                    &render_resource_manager.samplers,
                ),
                &resource_manager.lookup_tables,
                &mut render_resource_manager.lookup_table_bind_groups,
            )?;

            Ok(())
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
        HandleStagedEntities
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing mesh GPU resources", engine, {
            let resource_manager = engine.resource_manager().oread();
            let renderer = engine.renderer().oread();
            let mut render_resource_manager = renderer.render_resource_manager().owrite();

            impact_resource::gpu::sync_mutable_gpu_resources(
                engine.graphics_device(),
                &resource_manager.triangle_meshes,
                &mut render_resource_manager.triangle_meshes,
            )?;

            impact_resource::gpu::sync_mutable_gpu_resources(
                engine.graphics_device(),
                &resource_manager.line_segment_meshes,
                &mut render_resource_manager.line_segment_meshes,
            )?;

            Ok(())
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
        HandleStagedEntities,
        // Some materials need access to the current textures.
        SyncTextureGPUResources
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing material GPU resources", engine, {
            let resource_manager = engine.resource_manager().oread();
            let renderer = engine.renderer().oread();
            let mut render_resource_manager = renderer.render_resource_manager().owrite();
            let render_resource_manager = &mut **render_resource_manager;

            impact_resource::gpu::sync_immutable_gpu_resources(
                &(),
                &resource_manager.materials,
                &mut render_resource_manager.materials,
            )?;

            impact_resource::gpu::sync_immutable_gpu_resources(
                engine.graphics_device(),
                &resource_manager.material_templates,
                &mut render_resource_manager.material_templates,
            )?;

            impact_resource::gpu::sync_immutable_gpu_resources(
                &(
                    engine.graphics_device(),
                    &render_resource_manager.textures,
                    &render_resource_manager.samplers,
                    &render_resource_manager.material_templates,
                ),
                &resource_manager.material_texture_groups,
                &mut render_resource_manager.material_texture_groups,
            )?;

            Ok(())
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
        instrument_engine_task!("Synchronizing miscellaneous GPU resources", engine, {
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let skybox = scene.skybox().oread();
            let renderer = engine.renderer().oread();
            let mut render_resource_manager = renderer.render_resource_manager().owrite();
            let render_resource_manager = &mut **render_resource_manager;

            impact_scene::skybox::sync_gpu_resources_for_skybox(
                skybox.as_ref(),
                renderer.graphics_device(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
                &mut render_resource_manager.skybox,
            )?;

            resource_manager.voxel_types.sync_material_gpu_resources(
                renderer.graphics_device(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
                renderer.bind_group_layout_registry(),
                &mut render_resource_manager.voxel_materials,
            )?;

            Ok(())
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
        instrument_engine_task!("Synchronizing dynamic GPU resources", engine, {
            let scene = engine.scene().oread();
            let camera_manager = scene.camera_manager().oread();
            let light_manager = scene.light_manager().oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut renderer = engine.renderer().owrite();

            renderer.sync_dynamic_gpu_resources(
                &camera_manager,
                &light_manager,
                &mut voxel_object_manager,
                &mut model_instance_manager,
            );
            Ok(())
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
        instrument_engine_task!("Applying voxel absorbers", engine, {
            let mut entity_stager = engine.entity_stager().olock();
            let ecs_world = engine.ecs_world().oread();
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            let scene_graph = scene.scene_graph().oread();
            let simulator = engine.simulator().oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let force_generator_manager = simulator.force_generator_manager().oread();
            let collision_world = simulator.collision_world().oread();

            TaskArenas::with(|arena| {
                impact_voxel::interaction::systems::apply_absorption(
                    arena,
                    engine.component_metadata_registry(),
                    &mut entity_stager,
                    &ecs_world,
                    &scene_graph,
                    &mut voxel_object_manager,
                    &resource_manager.voxel_types,
                    &mut rigid_body_manager,
                    &mut anchor_manager,
                    &force_generator_manager,
                    &collision_world,
                    simulator.time_step_duration(),
                );
            });

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
    task_scheduler.register_task(SyncSceneObjectTransformsAndFlags)?;
    task_scheduler.register_task(UpdateSceneGroupToWorldTransforms)?;
    task_scheduler.register_task(UpdateSceneObjectBoundingSpheres)?;
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
