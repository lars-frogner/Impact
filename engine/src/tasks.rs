//! Task definitions, arranged in dependency-consistent order.

use crate::{
    gizmo,
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
            let scene = engine.scene().read();
            scene.model_instance_manager().write().clear_buffer_contents();
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
            let ecs_world = engine.ecs_world().read();
            let scene = engine.scene().read();
            let mut scene_graph = scene.scene_graph().write();
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
            let scene = engine.scene().read();
            scene.scene_graph()
                .write()
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
            let scene = engine.scene().read();
            scene.scene_graph()
                .write()
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
            let scene = engine.scene().read();
            if let Some(scene_camera) = scene.scene_camera().write().as_mut() {
                scene.scene_graph()
                    .read()
                    .sync_camera_view_transform(scene_camera);
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
            let ecs_world = engine.ecs_world().read();
            let scene = engine.scene().read();
            let scene_graph = scene.scene_graph().read();
            let mut light_manager = scene.light_manager().write();
            impact_scene::systems::sync_lights_in_storage(
                &ecs_world,
                &scene_graph,
                scene.scene_camera().read().as_ref(),
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
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let scene = engine.scene().read();
            let scene_camera = scene.scene_camera().read();
            if let Some(scene_camera) = scene_camera.as_ref() {
                scene.scene_graph()
                    .read()
                    .buffer_model_instances_for_rendering(
                        &resource_manager.materials,
                        &mut scene.model_instance_manager().write(),
                        scene_camera,
                        renderer.current_frame_count(),
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
            engine.simulator()
                .write()
                .advance_simulation(
                    &engine.scene()
                        .read()
                        .voxel_object_manager()
                        .read()
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
            let ecs_world = engine.ecs_world().read();
            let simulator = engine.simulator().read();
            let rigid_body_manager = simulator.rigid_body_manager().read();
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
            let scene = engine.scene().read();
            let mut voxel_object_manager = scene.voxel_object_manager().write();
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
            let mut entity_stager = engine.entity_stager().lock();
            let ecs_world = engine.ecs_world().read();
            let resource_manager = engine.resource_manager().read();
            let simulator = engine.simulator().read();
            let mut rigid_body_manager = simulator.rigid_body_manager().write();
            let force_generator_manager = simulator.force_generator_manager().read();
            let collision_world = simulator.collision_world().read();
            let scene = engine.scene().read();
            let mut voxel_object_manager = scene.voxel_object_manager().write();
            let scene_graph = scene.scene_graph().read();

            impact_voxel::interaction::systems::apply_absorption(
                engine.component_metadata_registry(),
                &mut entity_stager,
                &ecs_world,
                &scene_graph,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                &mut rigid_body_manager,
                &force_generator_manager,
                &collision_world,
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
            let mut ecs_world = engine.ecs_world().write();
            let scene = engine.scene().read();
            let voxel_object_manager = scene.voxel_object_manager().read();

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
            let ecs_world = engine.ecs_world().read();
            let simulator = engine.simulator().read();
            let mut collision_world = simulator.collision_world().write();

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
            let mut gizmo_manager = engine.gizmo_manager().write();
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
            let ecs_world = engine.ecs_world().read();
            let current_frame_count = engine.renderer().read().current_frame_count();
            let gizmo_manager = engine.gizmo_manager().read();
            let simulator = engine.simulator().read();
            let rigid_body_manager = simulator.rigid_body_manager().read();
            let collision_world = simulator.collision_world().read();
            let scene = engine.scene().read();
            let mut model_instance_manager = scene.model_instance_manager().write();
            let voxel_object_manager = scene.voxel_object_manager().read();
            let scene_graph = scene.scene_graph().read();
            let light_manager = scene.light_manager().read();
            let scene_camera = scene.scene_camera().read();

            gizmo::systems::buffer_transforms_for_gizmos(
                &ecs_world,
                &rigid_body_manager,
                &mut model_instance_manager,
                &gizmo_manager,
                &collision_world,
                &voxel_object_manager,
                &scene_graph,
                &light_manager,
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
            let shadow_mapping_enabled = engine.renderer().read().shadow_mapping_config().enabled;
            let scene = engine.scene().read();
            let scene_camera = scene.scene_camera().read();
            if let Some(scene_camera) = scene_camera.as_ref() {
                scene.scene_graph()
                    .read()
                    .bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut scene.light_manager().write(),
                        &mut scene.model_instance_manager().write(),
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
            let shadow_mapping_enabled = engine.renderer().read().shadow_mapping_config().enabled;
            let scene = engine.scene().read();
            let scene_camera = scene.scene_camera().read();
            if let Some(scene_camera) = scene_camera.as_ref() {
                scene.scene_graph()
                    .read()
                    .bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
                        &mut scene.light_manager().write(),
                        &mut scene.model_instance_manager().write(),
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
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();

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
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();
            let render_resource_manager = &mut *render_resource_manager;

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
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();
            let render_resource_manager = &mut *render_resource_manager;

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
    /// Synchronizes camera and skybox GPU resources.
    [pub] SyncMinorResources,
    depends_on = [SyncSceneCameraViewTransform],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing camera and skybox GPU resources", engine, {
            let renderer = engine.renderer().read();
            let scene = engine.scene().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();
            let render_resource_manager = &mut *render_resource_manager;

            impact_scene::camera::sync_gpu_resources_for_scene_camera(
                scene.scene_camera().read().as_ref(),
                renderer.graphics_device(),
                renderer.bind_group_layout_registry(),
                &mut render_resource_manager.camera,
            );

            impact_scene::skybox::sync_gpu_resources_for_skybox(
                scene.skybox().read().as_ref(),
                renderer.graphics_device(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
                &mut render_resource_manager.skybox,
            )?;

            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes voxel object GPU resources.
    [pub] SyncVoxelObjectGPUResources,
    depends_on = [SyncVoxelObjectMeshes],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object GPU resources", engine, {
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();
            let render_resource_manager = &mut *render_resource_manager;
            let scene = engine.scene().read();
            let mut voxel_object_manager = scene.voxel_object_manager().write();

            resource_manager.voxel_types.sync_material_gpu_resources(
                renderer.graphics_device(),
                &render_resource_manager.textures,
                &render_resource_manager.samplers,
                renderer.bind_group_layout_registry(),
                &mut render_resource_manager.voxel_materials,
            )?;

            voxel_object_manager.sync_voxel_object_gpu_buffers(
                renderer.graphics_device(),
                renderer.bind_group_layout_registry(),
                &mut render_resource_manager.voxel_object_buffers,
            )?;

            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes light GPU resources.
    [pub] SyncLightGPUResources,
    depends_on = [
        SyncLightsInStorage,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing light GPU resources", engine, {
            let renderer = engine.renderer().read();
            let mut render_resource_manager = renderer.render_resource_manager().write();
            let scene = engine.scene().read();
            let light_manager = scene.light_manager().read();

            light_manager.sync_gpu_resources(
                renderer.graphics_device(),
                renderer.bind_group_layout_registry(),
                &mut render_resource_manager.lights,
                renderer.shadow_mapping_config(),
            );

            Ok(())
        })
    }
);

define_task!(
    /// Synchronizes model instance GPU buffers.
    [pub] SyncModelInstanceBuffers,
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
            "Synchronizing model instance GPU buffers",
            engine,
            {
                let renderer = engine.renderer().read();
                let mut render_resource_manager = renderer.render_resource_manager().write();
                let scene = engine.scene().read();
                let mut model_instance_manager = scene.model_instance_manager().write();

                model_instance_manager.sync_gpu_buffers(
                    renderer.graphics_device(),
                    &mut render_resource_manager.model_instance_buffers,
                );

                Ok(())
            }
        )
    }
);

// =============================================================================
// RENDER PIPELINE EXECUTION
// =============================================================================

define_task!(
    /// Ensures that all render commands required for rendering the entities
    /// are up to date with the current render resources.
    [pub] SyncRenderCommands,
    depends_on = [
        SyncMinorResources,
        SyncMeshGPUResources,
        SyncTextureGPUResources,
        SyncMaterialGPUResources,
        SyncVoxelObjectGPUResources,
        SyncLightGPUResources,
        SyncModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing render commands", engine, {
            let resource_manager = engine.resource_manager().read();
            let renderer = engine.renderer().read();
            let mut shader_manager = renderer.shader_manager().write();
            let render_resource_manager = renderer.render_resource_manager().read();
            let mut render_command_manager = renderer.render_command_manager().write();

            render_command_manager.sync_with_render_resources(
                renderer.graphics_device(),
                &mut shader_manager,
                &*resource_manager,
                &*render_resource_manager,
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
            let resource_manager = engine.resource_manager().read();
            let scene = engine.scene().read();
            engine.renderer().write().render_to_surface(
                &resource_manager,
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
    task_scheduler.register_task(SyncMinorResources)?;
    task_scheduler.register_task(SyncVoxelObjectGPUResources)?;
    task_scheduler.register_task(SyncLightGPUResources)?;
    task_scheduler.register_task(SyncModelInstanceBuffers)?;

    // Render Pipeline Execution
    task_scheduler.register_task(SyncRenderCommands)?;
    task_scheduler.register_task(Render)
}
