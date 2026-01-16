//! Management of voxels for entities.

use crate::{
    lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager, scene::Scene,
};
use anyhow::{Context, Result, anyhow};
use impact_alloc::arena::ArenaPool;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID},
};
use impact_scene::{
    SceneEntityFlags, SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle,
    setup::Uncullable,
};
use impact_voxel::{
    VoxelObjectID,
    generation::{SDFVoxelGenerator, sdf::SDFGraph},
    setup::{
        self, DynamicVoxels, GeneratedVoxelObject, GradientNoiseVoxelTypes,
        MultifractalNoiseSDFModification, SameVoxelType, VoxelBox, VoxelSphere, VoxelSphereUnion,
    },
};
use parking_lot::RwLock;

pub fn setup_voxel_objects_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    simulator: &RwLock<PhysicsSimulator>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    // Make sure entities that have manually created voxel object and physics
    // context get a model transform component with the center of mass offset
    setup!(
        {
            let scene = scene.oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();
        },
        components,
        |voxel_object_id: &VoxelObjectID,
         model_transform: Option<&ModelTransform>|
         -> ModelTransform {
            if let Some(physics_context) =
                voxel_object_manager.get_physics_context(*voxel_object_id)
            {
                let center_of_mass = physics_context
                    .inertial_property_manager
                    .derive_center_of_mass();

                ModelTransform::with_offset(center_of_mass.compact())
            } else {
                model_transform.copied().unwrap_or_default()
            }
        }
    );

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |generated_voxel_object: &GeneratedVoxelObject,
         voxel_type: &SameVoxelType|
         -> Result<VoxelObjectID> {
            let generator_id = generated_voxel_object.generator_id;

            let generator = resource_manager
                .voxel_generators
                .get(generator_id)
                .ok_or_else(|| {
                    anyhow!("Tried to setup voxel object using missing generator {generator_id}")
                })?;

            let arena = ArenaPool::get_arena();

            let graph = generator
                .sdf_graph
                .build_in(&arena, generated_voxel_object.seed())
                .with_context(|| {
                    format!("Failed to compile meta SDF graph into atomic graph for voxel generator {generator_id}")
                })?;

            let sdf_generator = graph.build_in(&arena).with_context(|| {
                format!("Failed to build SDF generator from atomic graph for voxel generator {generator_id}")
            })?;

            let voxel_type_generator = voxel_type
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                generated_voxel_object.voxel_extent,
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_box: &VoxelBox,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_box.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_type
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_box.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_sphere: &VoxelSphere,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_sphere.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_type
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_sphere.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnion,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_sphere_union.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_type
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_sphere_union.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |generated_voxel_object: &GeneratedVoxelObject,
         voxel_types: &GradientNoiseVoxelTypes|
         -> Result<VoxelObjectID> {
            let generator_id = generated_voxel_object.generator_id;

            let generator = resource_manager
                .voxel_generators
                .get(generator_id)
                .ok_or_else(|| {
                    anyhow!("Tried to setup voxel object using missing generator {generator_id}")
                })?;

            let arena = ArenaPool::get_arena();

            let graph = generator
                .sdf_graph
                .build_in(&arena, generated_voxel_object.seed())
                .with_context(|| {
                    format!("Failed to compile meta SDF graph into atomic graph for voxel generator {generator_id}")
                })?;

            let sdf_generator = graph.build_in(&arena).with_context(|| {
                format!("Failed to build SDF generator from atomic graph for voxel generator {generator_id}")
            })?;

            let voxel_type_generator = voxel_types
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                generated_voxel_object.voxel_extent,
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_box: &VoxelBox,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_box.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_types
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_box.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_sphere: &VoxelSphere,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_sphere.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_types
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_sphere.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        },
        components,
        |voxel_sphere_union: &VoxelSphereUnion,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<VoxelObjectID> {
            let arena = ArenaPool::get_arena();

            let mut graph = SDFGraph::new_in(&arena);
            let node_id = voxel_sphere_union.add(&mut graph);
            setup::apply_modifications(&mut graph, node_id, multifractal_noise_modification);

            let sdf_generator = graph.build_in(&arena)?;

            let voxel_type_generator = voxel_types
                .create_generator(&resource_manager.voxel_types)?
                .into();

            let generator = SDFVoxelGenerator::new(
                voxel_sphere_union.voxel_extent(),
                sdf_generator,
                voxel_type_generator,
            );

            Ok(setup::setup_voxel_object(
                &mut voxel_object_manager,
                &generator,
            ))
        },
        ![VoxelObjectID]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_object_manager = scene.voxel_object_manager().owrite();
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        components,
        |voxel_object_id: &VoxelObjectID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> Result<(DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion)> {
            setup::setup_dynamic_rigid_body_for_voxel_object(
                &mut rigid_body_manager,
                &mut voxel_object_manager,
                &resource_manager.voxel_types,
                *voxel_object_id,
                model_transform,
                frame,
                motion,
            )
        },
        [DynamicVoxels],
        ![DynamicRigidBodyID, KinematicRigidBodyID]
    )?;

    Ok(())
}

pub fn setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let voxel_object_manager = scene.voxel_object_manager().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        components,
        |voxel_object_id: &VoxelObjectID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         parent: Option<&SceneGraphParentNodeHandle>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(
            SceneGraphModelInstanceNodeHandle,
            ModelTransform,
            SceneEntityFlags
        )> {
            setup::create_model_instance_node_for_voxel_object(
                &voxel_object_manager,
                &mut model_instance_manager,
                &mut scene_graph,
                voxel_object_id,
                model_transform,
                frame,
                parent,
                flags,
                components.has_component_type::<Uncullable>(),
            )
        },
        ![SceneGraphModelInstanceNodeHandle]
    )
}

pub fn cleanup_voxel_object_for_removed_entity(
    scene: &RwLock<Scene>,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if let Some(voxel_object_id) = entity.get_component::<VoxelObjectID>() {
        let scene = scene.oread();
        let mut voxel_object_manager = scene.voxel_object_manager().owrite();
        voxel_object_manager.remove_voxel_object(*voxel_object_id.access());
    }
}
