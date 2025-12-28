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
    let mut point = local_force.point.unpack();

    if let Some(transform) = model_transform {
        // Transform point to body-fixed frame
        point = transform.transform_point_from_model_space_to_entity_space(&point);
    }

    let anchor = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id,
        point: point.pack(),
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
    let mut point_1 = properties.attachment_point_1.unpack();
    let mut point_2 = properties.attachment_point_2.unpack();

    if let Some(transform) = model_transform {
        // Transform points to body-fixed frame
        point_1 = transform.transform_point_from_model_space_to_entity_space(&point_1);
        point_2 = transform.transform_point_from_model_space_to_entity_space(&point_2);
    }

    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: point_1.pack(),
    });

    let anchor_2 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_2,
        point: point_2.pack(),
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
    let mut point_1 = properties.attachment_point_1.unpack();
    let mut point_2 = properties.attachment_point_2.unpack();

    if let Some(transform) = model_transform {
        // Transform points to body-fixed frame
        point_1 = transform.transform_point_from_model_space_to_entity_space(&point_1);
        point_2 = transform.transform_point_from_model_space_to_entity_space(&point_2);
    }

    let anchor_1 = anchor_manager.dynamic_mut().insert(DynamicRigidBodyAnchor {
        rigid_body_id: properties.rigid_body_1,
        point: point_1.pack(),
    });

    let anchor_2 = anchor_manager
        .kinematic_mut()
        .insert(KinematicRigidBodyAnchor {
            rigid_body_id: properties.rigid_body_2,
            point: point_2.pack(),
        });

    force_generator_manager
        .dynamic_kinematic_spring_forces_mut()
        .insert_generator(DynamicKinematicSpringForceGenerator {
            anchor_1,
            anchor_2,
            spring: properties.spring,
        })
}
