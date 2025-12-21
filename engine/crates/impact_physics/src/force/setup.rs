//! Force setup and cleanup.

use impact_geometry::ModelTransform;

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
        .insert_generator(ConstantAccelerationGenerator::new(
            rigid_body_id,
            acceleration,
        ))
}

pub fn setup_local_force(
    anchor_manager: &mut AnchorManager,
    force_generator_manager: &mut ForceGeneratorManager,
    rigid_body_id: DynamicRigidBodyID,
    local_force: LocalForce,
    model_transform: Option<&ModelTransform>,
) -> LocalForceGeneratorID {
    // Transform point to body-fixed frame
    let point = model_transform.map_or(local_force.point, |transform| {
        transform.transform_point_from_model_space_to_entity_space(&local_force.point)
    });

    let anchor = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id,
        point,
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
    model_transform: Option<&ModelTransform>,
) -> DynamicDynamicSpringForceGeneratorID {
    // Transform points to body-fixed frame
    let point_1 = model_transform.map_or(properties.attachment_point_1, |transform| {
        transform.transform_point_from_model_space_to_entity_space(&properties.attachment_point_1)
    });
    let point_2 = model_transform.map_or(properties.attachment_point_2, |transform| {
        transform.transform_point_from_model_space_to_entity_space(&properties.attachment_point_2)
    });

    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: point_1,
    });

    let anchor_2 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_2,
        point: point_2,
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
    model_transform: Option<&ModelTransform>,
) -> DynamicKinematicSpringForceGeneratorID {
    // Transform points to body-fixed frame
    let point_1 = model_transform.map_or(properties.attachment_point_1, |transform| {
        transform.transform_point_from_model_space_to_entity_space(&properties.attachment_point_1)
    });
    let point_2 = model_transform.map_or(properties.attachment_point_2, |transform| {
        transform.transform_point_from_model_space_to_entity_space(&properties.attachment_point_2)
    });

    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: point_1,
    });

    let anchor_2 = anchor_manager
        .kinematic_mut()
        .insert(KinematicRigidBodyAnchor {
            rigid_body_id: properties.rigid_body_2,
            point: point_2,
        });

    force_generator_manager
        .dynamic_kinematic_spring_forces_mut()
        .insert_generator(DynamicKinematicSpringForceGenerator {
            anchor_1,
            anchor_2,
            spring: properties.spring,
        })
}
