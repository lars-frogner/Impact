//! Setup of forces for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager};
use anyhow::{Result, anyhow};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ModelTransform;
use impact_math::hash::StringHash32;
use impact_mesh::TriangleMeshID;
use impact_physics::{
    force::{
        alignment_torque::AlignmentTorqueGeneratorID,
        constant_acceleration::ConstantAccelerationGeneratorID,
        detailed_drag::DetailedDragForceGeneratorID,
        dynamic_gravity::DynamicGravity,
        local_force::LocalForceGeneratorID,
        setup::{
            self, ConstantAcceleration, DetailedDragProperties, FixedDirectionAlignmentTorque,
            GravityAlignmentTorque, LocalForce,
        },
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicDynamicSpringForceProperties,
            DynamicKinematicSpringForceGeneratorID, DynamicKinematicSpringForceProperties,
        },
    },
    rigid_body::DynamicRigidBodyID,
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
        |rigid_body_id: &DynamicRigidBodyID,
         acceleration: &ConstantAcceleration|
         -> ConstantAccelerationGeneratorID {
            setup::setup_constant_acceleration(
                &mut force_generator_manager,
                *rigid_body_id,
                *acceleration,
            )
        }
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |rigid_body_id: &DynamicRigidBodyID,
         local_force: &LocalForce,
         model_transform: Option<&ModelTransform>|
         -> LocalForceGeneratorID {
            setup::setup_local_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *rigid_body_id,
                *local_force,
                model_transform,
            )
        }
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |properties: &DynamicDynamicSpringForceProperties,
         model_transform: Option<&ModelTransform>|
         -> DynamicDynamicSpringForceGeneratorID {
            setup::setup_dynamic_dynamic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *properties,
                model_transform,
            )
        }
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |properties: &DynamicKinematicSpringForceProperties,
         model_transform: Option<&ModelTransform>|
         -> DynamicKinematicSpringForceGeneratorID {
            setup::setup_dynamic_kinematic_spring_force(
                &mut anchor_manager,
                &mut force_generator_manager,
                *properties,
                model_transform,
            )
        }
    );

    setup!(
        {
            let resource_manager = resource_manager.oread();
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |drag_properties: &DetailedDragProperties,
         rigid_body_id: &DynamicRigidBodyID,
         model_transform: &ModelTransform,
         mesh_id: &TriangleMeshID|
         -> Result<DetailedDragForceGeneratorID> {
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
                *rigid_body_id,
                model_transform,
                StringHash32::new(mesh_id.to_string()),
                triangle_mesh.triangle_vertex_positions(),
            )
        }
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |rigid_body_id: &DynamicRigidBodyID| {
            setup::setup_dynamic_gravity(&mut force_generator_manager, *rigid_body_id);
        },
        [DynamicGravity]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |torque: &FixedDirectionAlignmentTorque,
         rigid_body_id: &DynamicRigidBodyID|
         -> AlignmentTorqueGeneratorID {
            setup::setup_fixed_direction_alignment_torque(
                &mut force_generator_manager,
                *rigid_body_id,
                *torque,
            )
        }
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
        },
        entities,
        |torque: &GravityAlignmentTorque,
         rigid_body_id: &DynamicRigidBodyID|
         -> AlignmentTorqueGeneratorID {
            setup::setup_gravity_alignment_torque(
                &mut force_generator_manager,
                *rigid_body_id,
                *torque,
            )
        }
    );

    Ok(())
}

pub fn remove_force_generators_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity: &EntityEntry<'_>,
) {
    if let Some(generator_id) = entity.get_component::<ConstantAccelerationGeneratorID>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .constant_accelerations_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<LocalForceGeneratorID>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .local_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DynamicDynamicSpringForceGeneratorID>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .dynamic_dynamic_spring_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DynamicKinematicSpringForceGeneratorID>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .dynamic_kinematic_spring_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DetailedDragForceGeneratorID>() {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .detailed_drag_forces_mut()
            .generators_mut()
            .remove_generator(*generator_id.access());
    }
    if entity.has_component::<DynamicGravity>()
        && let Some(rigid_body_id) = entity.get_component::<DynamicRigidBodyID>()
    {
        let simulator = simulator.oread();
        let mut force_generator_manager = simulator.force_generator_manager().owrite();
        force_generator_manager
            .dynamic_gravity_manager_mut()
            .remove_body(*rigid_body_id.access());
    }
}
