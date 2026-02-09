//! Management of voxels for entities.

use crate::{
    lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager, scene::Scene,
};
use anyhow::{Context, Result, anyhow};
use impact_alloc::arena::ArenaPool;
use impact_ecs::{setup, world::PrototypeEntities};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_id::EntityID;
use impact_model::HasModel;
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID},
};
use impact_scene::{ParentEntity, SceneEntityFlags, setup::Uncullable};
use impact_voxel::{
    HasVoxelObject, VoxelObjectID,
    generation::{SDFVoxelGenerator, sdf::SDFGraph},
    interaction::absorption::{
        HasVoxelAbsorbingCapsule, HasVoxelAbsorbingSphere, VoxelAbsorbingCapsule,
        VoxelAbsorbingCapsuleID, VoxelAbsorbingSphere, VoxelAbsorbingSphereID,
    },
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
    entities: &mut PrototypeEntities,
) -> Result<()> {
    // Make sure entities that have manually created voxel object and physics
    // context get a model transform component with the center of mass offset
    setup!(
        {
            let scene = scene.oread();
            let voxel_manager = scene.voxel_manager().oread();
        },
        entities,
        |entity_id: EntityID, model_transform: Option<&ModelTransform>| -> ModelTransform {
            let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
            if let Some(physics_context) = voxel_manager
                .object_manager()
                .get_physics_context(voxel_object_id)
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
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         generated_voxel_object: &GeneratedVoxelObject,
         voxel_type: &SameVoxelType|
         -> Result<HasVoxelObject> {
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
                .build_in(&arena, generated_voxel_object.scale_factor, generated_voxel_object.seed)
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_box: &VoxelBox,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_sphere: &VoxelSphere,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_sphere_union: &VoxelSphereUnion,
         voxel_type: &SameVoxelType,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         generated_voxel_object: &GeneratedVoxelObject,
         voxel_types: &GradientNoiseVoxelTypes|
         -> Result<HasVoxelObject> {
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
                .build_in(&arena, generated_voxel_object.scale_factor, generated_voxel_object.seed)
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_box: &VoxelBox,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_sphere: &VoxelSphere,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_sphere_union: &VoxelSphereUnion,
         voxel_types: &GradientNoiseVoxelTypes,
         multifractal_noise_modification: Option<&MultifractalNoiseSDFModification>|
         -> Result<HasVoxelObject> {
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

            setup::setup_voxel_object(voxel_manager.object_manager_mut(), &generator, entity_id)?;

            Ok(HasVoxelObject)
        },
        ![HasVoxelObject]
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
            let simulator = simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         motion: Option<&Motion>|
         -> Result<(DynamicRigidBodyID, ModelTransform, ReferenceFrame, Motion)> {
            setup::setup_dynamic_rigid_body_for_voxel_object(
                &mut rigid_body_manager,
                voxel_manager.object_manager_mut(),
                &resource_manager.voxel_types,
                entity_id,
                model_transform,
                frame,
                motion,
            )
        },
        [HasVoxelObject, DynamicVoxels],
        ![DynamicRigidBodyID, KinematicRigidBodyID]
    )?;

    Ok(())
}

pub fn setup_voxel_interaction_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         absorbing_sphere: &VoxelAbsorbingSphere|
         -> Result<HasVoxelAbsorbingSphere> {
            let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity_id);
            voxel_manager
                .interaction_manager_mut()
                .absorption_manager_mut()
                .add_absorbing_sphere(absorber_id, *absorbing_sphere)?;
            Ok(HasVoxelAbsorbingSphere)
        },
        ![HasVoxelAbsorbingSphere]
    )?;

    setup!(
        {
            let scene = scene.oread();
            let mut voxel_manager = scene.voxel_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         absorbing_capsule: &VoxelAbsorbingCapsule|
         -> Result<HasVoxelAbsorbingCapsule> {
            let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity_id);
            voxel_manager
                .interaction_manager_mut()
                .absorption_manager_mut()
                .add_absorbing_capsule(absorber_id, *absorbing_capsule)?;
            Ok(HasVoxelAbsorbingCapsule)
        },
        ![HasVoxelAbsorbingSphere]
    )
}

pub fn setup_scene_graph_model_instance_nodes_for_new_voxel_object_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let voxel_manager = scene.voxel_manager().oread();
            let mut model_instance_manager = scene.model_instance_manager().owrite();
            let mut scene_graph = scene.scene_graph().owrite();
        },
        entities,
        |entity_id: EntityID,
         model_transform: Option<&ModelTransform>,
         frame: Option<&ReferenceFrame>,
         parent: Option<&ParentEntity>,
         flags: Option<&SceneEntityFlags>|
         -> Result<(HasModel, ModelTransform, SceneEntityFlags)> {
            let (model_transform, flags) = setup::create_model_instance_node_for_voxel_object(
                voxel_manager.object_manager(),
                &mut model_instance_manager,
                &mut scene_graph,
                entity_id,
                model_transform,
                frame,
                parent,
                flags,
                entities.has_component_type::<Uncullable>(),
            )?;
            Ok((HasModel, model_transform, flags))
        },
        [HasVoxelObject],
        ![HasModel]
    )
}

pub fn cleanup_voxel_object_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if entity.has_component::<HasVoxelObject>() {
        let scene = scene.oread();
        let mut voxel_manager = scene.voxel_manager().owrite();
        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
        voxel_manager
            .object_manager_mut()
            .remove_voxel_object(voxel_object_id);
    }
}

pub fn cleanup_voxel_interaction_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if entity.has_component::<HasVoxelAbsorbingSphere>() {
        let scene = scene.oread();
        let mut voxel_manager = scene.voxel_manager().owrite();
        let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity_id);
        voxel_manager
            .interaction_manager_mut()
            .absorption_manager_mut()
            .remove_absorbing_sphere(absorber_id);
    }
    if entity.has_component::<HasVoxelAbsorbingCapsule>() {
        let scene = scene.oread();
        let mut voxel_manager = scene.voxel_manager().owrite();
        let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity_id);
        voxel_manager
            .interaction_manager_mut()
            .absorption_manager_mut()
            .remove_absorbing_capsule(absorber_id);
    }
}
