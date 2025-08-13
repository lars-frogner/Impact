//! Force setup and cleanup.

pub use crate::force::{
    constant_acceleration::ConstantAcceleration,
    detailed_drag::setup::{DetailedDragProperties, setup_detailed_drag_force},
    local_force::LocalForce,
    spring_force::{DynamicDynamicSpringForceGenerator, DynamicKinematicSpringForceGenerator},
};

use crate::{
    anchor::{AnchorManager, DynamicRigidBodyAnchor, KinematicRigidBodyAnchor},
    force::{
        ForceGeneratorManager,
        constant_acceleration::{ConstantAccelerationGenerator, ConstantAccelerationGeneratorID},
        local_force::{LocalForceGenerator, LocalForceGeneratorID},
        spring_force::{
            DynamicDynamicSpringForceGeneratorID, DynamicDynamicSpringForceProperties,
            DynamicKinematicSpringForceGeneratorID, DynamicKinematicSpringForceProperties,
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
    anchor_manager: &mut AnchorManager,
    force_generator_manager: &mut ForceGeneratorManager,
    rigid_body_id: DynamicRigidBodyID,
    local_force: LocalForce,
) -> LocalForceGeneratorID {
    let anchor = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id,
        point: local_force.point,
    });

    force_generator_manager
        .local_forces_mut()
        .insert_generator(LocalForceGenerator {
            anchor,
            force: local_force.force,
        })
}

pub fn setup_dynamic_dynamic_spring_force(
    anchor_manager: &mut AnchorManager,
    force_generator_manager: &mut ForceGeneratorManager,
    properties: DynamicDynamicSpringForceProperties,
) -> DynamicDynamicSpringForceGeneratorID {
    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: properties.attachment_point_1,
    });

    let anchor_2 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_2,
        point: properties.attachment_point_2,
    });

    force_generator_manager
        .dynamic_dynamic_spring_forces_mut()
        .insert_generator(DynamicDynamicSpringForceGenerator {
            anchor_1,
            anchor_2,
            spring: properties.spring,
        })
}

pub fn setup_dynamic_kinematic_spring_force(
    anchor_manager: &mut AnchorManager,
    force_generator_manager: &mut ForceGeneratorManager,
    properties: DynamicKinematicSpringForceProperties,
) -> DynamicKinematicSpringForceGeneratorID {
    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: properties.attachment_point_1,
    });

    let anchor_2 = anchor_manager
        .kinematic_mut()
        .insert(KinematicRigidBodyAnchor {
            rigid_body_id: properties.rigid_body_2,
            point: properties.attachment_point_2,
        });

    force_generator_manager
        .dynamic_kinematic_spring_forces_mut()
        .insert_generator(DynamicKinematicSpringForceGenerator {
            anchor_1,
            anchor_2,
            spring: properties.spring,
        })
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
