//! Setup of forces for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager};
use anyhow::{Result, anyhow};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ModelTransform;
use impact_id::EntityID;
use impact_math::hash::StringHash32;
use impact_mesh::TriangleMeshID;
use impact_physics::{
    force::{
        alignment_torque::HasAlignmentTorqueGenerator,
        constant_acceleration::{
            ConstantAccelerationGeneratorID, HasConstantAccelerationGenerator,
        },
        detailed_drag::{DetailedDragForceGeneratorID, HasDetailedDragForceGenerator},
        dynamic_gravity::DynamicGravity,
        local_force::{HasLocalForceGenerator, LocalForceGeneratorID},
        setup::{
            self, ConstantAcceleration, DetailedDragProperties, FixedDirectionAlignmentTorque,
            GravityAlignmentTorque, LocalForce,
        },
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicDynamicSpringForceProperties,
            DynamicKinematicSpringForceGeneratorID, DynamicKinematicSpringForceProperties,
            HasDynamicDynamicSpringForceGenerator, HasDynamicKinematicSpringForceGenerator,
        },
    },
    rigid_body::{DynamicRigidBodyID, HasDynamicRigidBody},
};
use parking_lot::RwLock;

pub fn setup_forces_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         acceleration: &ConstantAcceleration|
         -> Result<HasConstantAccelerationGenerator> {
            setup::setup_constant_acceleration(
                &mut force_generator_manager,
                entity_id,
                *acceleration,
            )?;
            Ok(HasConstantAccelerationGenerator)
        },
        [HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         local_force: &LocalForce,
         model_transform: Option<&ModelTransform>|
         -> Result<HasLocalForceGenerator> {
            setup::setup_local_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                entity_id,
                *local_force,
                model_transform,
            )?;
            Ok(HasLocalForceGenerator)
        },
        [HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         properties: &DynamicDynamicSpringForceProperties,
         model_transform: Option<&ModelTransform>|
         -> Result<HasDynamicDynamicSpringForceGenerator> {
            setup::setup_dynamic_dynamic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                entity_id,
                *properties,
                model_transform,
            )?;
            Ok(HasDynamicDynamicSpringForceGenerator)
        }
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         properties: &DynamicKinematicSpringForceProperties,
         model_transform: Option<&ModelTransform>|
         -> Result<HasDynamicKinematicSpringForceGenerator> {
            setup::setup_dynamic_kinematic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                entity_id,
                *properties,
                model_transform,
            )?;
            Ok(HasDynamicKinematicSpringForceGenerator)
        }
    )?;

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         drag_properties: &DetailedDragProperties,
         model_transform: &ModelTransform,
         mesh_id: &TriangleMeshID|
         -> Result<HasDetailedDragForceGenerator> {
            let triangle_mesh =
                resource_manager
                    .triangle_meshes
                    .get(*mesh_id)
                    .ok_or_else(|| {
                        anyhow!("Tried to setup detailed drag for missing mesh {}", mesh_id)
                    })?;

            setup::setup_detailed_drag_force(
                &mut force_generator_manager,
                *drag_properties,
                entity_id,
                model_transform,
                StringHash32::new(mesh_id.to_string()),
                triangle_mesh.triangle_vertex_positions(),
            )?;

            Ok(HasDetailedDragForceGenerator)
        },
        [HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID| {
            setup::setup_dynamic_gravity(&mut force_generator_manager, entity_id);
        },
        [DynamicGravity, HasDynamicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         torque: &FixedDirectionAlignmentTorque|
         -> Result<HasAlignmentTorqueGenerator> {
            setup::setup_fixed_direction_alignment_torque(
                &mut force_generator_manager,
                entity_id,
                *torque,
            )?;
            Ok(HasAlignmentTorqueGenerator)
        },
        [HasDynamicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         torque: &GravityAlignmentTorque|
         -> Result<HasAlignmentTorqueGenerator> {
            setup::setup_gravity_alignment_torque(
                &mut force_generator_manager,
                entity_id,
                *torque,
            )?;
            Ok(HasAlignmentTorqueGenerator)
        },
        [HasDynamicRigidBody]
    )?;

    Ok(())
}

pub fn remove_force_generators_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasConstantAccelerationGenerator>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let generator_id = ConstantAccelerationGeneratorID::from_entity_id(entity_id);
        force_generator_manager
            .constant_accelerations_mut()
            .remove_generator(generator_id);
    }
    if entity.has_component::<HasLocalForceGenerator>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let generator_id = LocalForceGeneratorID::from_entity_id(entity_id);
        force_generator_manager
            .local_forces_mut()
            .remove_generator(generator_id);
    }
    if entity.has_component::<HasDynamicDynamicSpringForceGenerator>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let generator_id = DynamicDynamicSpringForceGeneratorID::from_entity_id(entity_id);
        force_generator_manager
            .dynamic_dynamic_spring_forces_mut()
            .remove_generator(generator_id);
    }
    if entity.has_component::<HasDynamicKinematicSpringForceGenerator>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let generator_id = DynamicKinematicSpringForceGeneratorID::from_entity_id(entity_id);
        force_generator_manager
            .dynamic_kinematic_spring_forces_mut()
            .remove_generator(generator_id);
    }
    if entity.has_component::<HasDetailedDragForceGenerator>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let generator_id = DetailedDragForceGeneratorID::from_entity_id(entity_id);
        force_generator_manager
            .detailed_drag_forces_mut()
            .generators_mut()
            .remove_generator(generator_id);
    }
    if entity.has_component::<DynamicGravity>() && entity.has_component::<HasDynamicRigidBody>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
        force_generator_manager
            .dynamic_gravity_manager_mut()
            .remove_body(rigid_body_id);
    }
}
