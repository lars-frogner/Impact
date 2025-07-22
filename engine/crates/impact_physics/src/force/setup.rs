//! Force setup.

pub use crate::force::{
    constant_acceleration::ConstantAcceleration,
    detailed_drag::setup::{DetailedDragProperties, setup_detailed_drag_force},
    local_force::LocalForce,
    spring_force::{DynamicDynamicSpringForceGenerator, DynamicKinematicSpringForceGenerator},
};

use crate::{
    force::{
        ForceGeneratorManager,
        constant_acceleration::{ConstantAccelerationGenerator, ConstantAccelerationGeneratorID},
        local_force::{LocalForceGenerator, LocalForceGeneratorID},
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicKinematicSpringForceGeneratorID,
        },
    },
    rigid_body::DynamicRigidBodyID,
};

pub fn setup_constant_acceleration(
    force_generator_manager: &mut ForceGeneratorManager,
    rigid_body_id: DynamicRigidBodyID,
    acceleration: ConstantAcceleration,
) -> ConstantAccelerationGeneratorID {
    force_generator_manager
        .constant_accelerations_mut()
        .insert_generator(ConstantAccelerationGenerator {
            rigid_body_id,
            acceleration,
        })
}

pub fn setup_local_force(
    force_generator_manager: &mut ForceGeneratorManager,
    rigid_body_id: DynamicRigidBodyID,
    local_force: LocalForce,
) -> LocalForceGeneratorID {
    force_generator_manager
        .local_forces_mut()
        .insert_generator(LocalForceGenerator {
            rigid_body_id,
            local_force,
        })
}

pub fn setup_dynamic_dynamic_spring_force_generator(
    force_generator_manager: &mut ForceGeneratorManager,
    generator: DynamicDynamicSpringForceGenerator,
) -> DynamicDynamicSpringForceGeneratorID {
    force_generator_manager
        .dynamic_dynamic_spring_forces_mut()
        .insert_generator(generator)
}

pub fn setup_dynamic_kinematic_spring_force_generator(
    force_generator_manager: &mut ForceGeneratorManager,
    generator: DynamicKinematicSpringForceGenerator,
) -> DynamicKinematicSpringForceGeneratorID {
    force_generator_manager
        .dynamic_kinematic_spring_forces_mut()
        .insert_generator(generator)
}

#[cfg(feature = "ecs")]
pub fn remove_force_generators_for_entity(
    force_generator_manager: &parking_lot::RwLock<ForceGeneratorManager>,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    use crate::force::detailed_drag::DetailedDragForceGeneratorID;

    if let Some(generator_id) = entity.get_component::<ConstantAccelerationGeneratorID>() {
        force_generator_manager
            .write()
            .constant_accelerations_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<LocalForceGeneratorID>() {
        force_generator_manager
            .write()
            .local_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DynamicDynamicSpringForceGeneratorID>() {
        force_generator_manager
            .write()
            .dynamic_dynamic_spring_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DynamicKinematicSpringForceGeneratorID>() {
        force_generator_manager
            .write()
            .dynamic_kinematic_spring_forces_mut()
            .remove_generator(*generator_id.access());
    }
    if let Some(generator_id) = entity.get_component::<DetailedDragForceGeneratorID>() {
        force_generator_manager
            .write()
            .detailed_drag_forces_mut()
            .generators_mut()
            .remove_generator(*generator_id.access());
    }
}
