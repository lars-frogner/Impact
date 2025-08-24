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
    depends_on = [SyncSceneObjectTransformsAndFlags],
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
    /// Updates the bounding spheres of all [`SceneGraph`](crate::scene::SceneGraph) nodes.
    [pub] UpdateSceneObjectBoundingSpheres,
    depends_on = [SyncSceneObjectTransformsAndFlags],
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
        SyncSceneObjectTransformsAndFlags,
        UpdateSceneGroupToWorldTransforms
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing scene camera view transform", engine, {
            let scene = engine.scene().oread();
            let mut scene_camera = scene.scene_camera().owrite();
            if let Some(scene_camera) = scene_camera.as_mut() {
                let scene_graph = scene.scene_graph().oread();
                scene_graph.sync_camera_view_transform(scene_camera);
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
    /// of every light source in the [`LightManager`](crate::light::LightManager).
    [pub] SyncLightsInStorage,
    depends_on = [
        UpdateSceneGroupToWorldTransforms,
        SyncSceneCameraViewTransform
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing lights in storage", engine, {
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            let mut light_manager = scene.light_manager().owrite();
            let scene_graph = scene.scene_graph().oread();
            impact_scene::systems::sync_lights_in_storage(
                &ecs_world,
                &scene_graph,
                (**scene_camera).as_ref(),
                &mut light_manager,
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
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            if let Some(scene_camera) = (**scene_camera).as_ref() {
                let mut model_instance_manager = scene.model_instance_manager().owrite();
                let scene_graph = scene.scene_graph().oread();
                let current_frame_count = engine.renderer().oread().current_frame_count();

                scene_graph.buffer_model_instances_for_rendering(
                    &resource_manager.materials,
                    &mut model_instance_manager,
                    scene_camera,
                    current_frame_count,
                );
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
    depends_on = [AdvanceSimulation, ApplyVoxelAbsorption],
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
            let scene = engine.scene().oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            voxel_object_manager.sync_voxel_object_meshes();
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

define_task!(
    /// Updates the [`ModelTransform`](impact_geometry::ModelTransform) component
    /// of each voxel object to match its center of mass.
    [pub] SyncVoxelObjectModelTransforms,
    depends_on = [ApplyVoxelAbsorption],
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
            let ecs_world = engine.ecs_world().oread();
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            let light_manager = scene.light_manager().oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let scene_graph = scene.scene_graph().oread();
            let simulator = engine.simulator().oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
            let anchor_manager = simulator.anchor_manager().oread();
            let collision_world = simulator.collision_world().oread();
            let renderer = engine.renderer().oread();
            let current_frame_count = renderer.current_frame_count();
            let gizmo_manager = engine.gizmo_manager().oread();

            gizmo::systems::buffer_transforms_for_gizmos(
                &ecs_world,
                &rigid_body_manager,
                &anchor_manager,
                &mut model_instance_manager,
                &gizmo_manager,
                &collision_world,
                &voxel_object_manager,
                &scene_graph,
                &light_manager,
                (**scene_camera).as_ref(),
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
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            if let Some(scene_camera) = (**scene_camera).as_ref() {
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
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            if let Some(scene_camera) = (**scene_camera).as_ref() {
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
// GPU RESOURCE SYNCHRONIZATION
// =============================================================================

define_task!(
    /// Synchronizes mesh GPU resources for triangle and line segment meshes.
    [pub] SyncMeshGPUResources,
    depends_on = [],
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
    /// Synchronizes GPU resources for materials.
    [pub] SyncMaterialGPUResources,
    depends_on = [SyncTextureGPUResources],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing material GPU resources", engine, {
            let resource_manager = engine.resource_manager().oread();
            let renderer = engine.renderer().oread();
            let mut render_resource_manager = renderer.render_resource_manager().owrite();
            let render_resource_manager = &mut **render_resource_manager;

            impact_resource::gpu::sync_immutable_gpu_resources(
                engine.graphics_device(),
                &resource_manager.material_templates,
                &mut render_resource_manager.material_template_bind_group_layouts,
            )?;

            impact_resource::gpu::sync_immutable_gpu_resources(
                &(
                    engine.graphics_device(),
                    &render_resource_manager.textures,
                    &render_resource_manager.samplers,
                    &render_resource_manager.material_template_bind_group_layouts,
                ),
                &resource_manager.material_texture_groups,
                &mut render_resource_manager.material_texture_bind_groups,
            )?;

            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes miscellaneous GPU resources.
    [pub] SyncMiscGPUResources,
    depends_on = [],
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

// =============================================================================
// RENDER PIPELINE EXECUTION
// =============================================================================

define_task!(
    /// Executes the [`RenderingSystem::record_commands_and_render_surface`]
    /// method.
    [pub] RecordCommandsAndRender,
    depends_on = [
        SyncMeshGPUResources,
        SyncTextureGPUResources,
        SyncMaterialGPUResources,
        SyncMiscGPUResources,
        SyncSceneCameraViewTransform,
        SyncVoxelObjectMeshes,
        SyncLightsInStorage,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances,
        BufferModelInstancesForRendering,
        BufferTransformsForGizmos
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Recording commands and rendering surface", engine, {
            let resource_manager = engine.resource_manager().oread();
            let scene = engine.scene().oread();
            let scene_camera = scene.scene_camera().oread();
            let light_manager = scene.light_manager().oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut renderer = engine.renderer().owrite();

            let command_encoder = renderer.record_synchronization_commands(
                (**scene_camera).as_ref(),
                &light_manager,
                &mut voxel_object_manager,
                &mut model_instance_manager,
            );

            drop(voxel_object_manager);
            let model_instance_manager = model_instance_manager.downgrade();

            renderer.render_surface(
                command_encoder,
                &resource_manager,
                (**scene_camera).as_ref(),
                &light_manager,
                &model_instance_manager,
                ctx.user_interface(),
            )
        })
    }
);

define_task!(
    /// Captures and saves any screenshots or related textures requested through
    /// the [`ScreenCapturer`].
    [pub] SaveRequestedScreenshots,
    depends_on = [RecordCommandsAndRender],
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
    task_scheduler.register_task(SyncVoxelObjectCollidables)?;

    // Gizmo Processing
    task_scheduler.register_task(UpdateVisibilityFlagsForGizmos)?;
    task_scheduler.register_task(BufferTransformsForGizmos)?;

    // Shadow Mapping and Light Culling
    task_scheduler.register_task(BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances)?;
    task_scheduler.register_task(BoundUnidirectionalLightsAndBufferShadowCastingModelInstances)?;

    // GPU Resource Synchronization
    task_scheduler.register_task(SyncMeshGPUResources)?;
    task_scheduler.register_task(SyncTextureGPUResources)?;
    task_scheduler.register_task(SyncMaterialGPUResources)?;
    task_scheduler.register_task(SyncMiscGPUResources)?;

    // Render Pipeline Execution
    task_scheduler.register_task(RecordCommandsAndRender)?;
    task_scheduler.register_task(SaveRequestedScreenshots)
}
