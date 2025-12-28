//! Management of rigid bodies for entities.

use crate::{
    inertia::InertialProperties,
    quantities::Motion,
    rigid_body::{
        DynamicRigidBody, DynamicRigidBodyID, KinematicRigidBody, KinematicRigidBodyID,
        RigidBodyManager,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_geometry::ReferenceFrame;
use roc_integration::roc;

define_setup_type! {
    target = DynamicRigidBodyID;
    /// The properties of the substance making up a dynamic rigid body.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicRigidBodySubstance {
        // The mass density of the body's substance.
        pub mass_density: f32,
    }
}

pub fn setup_kinematic_rigid_body(
    rigid_body_manager: &mut RigidBodyManager,
    frame: ReferenceFrame,
    motion: Motion,
) -> KinematicRigidBodyID {
    let rigid_body = KinematicRigidBody::new(
        frame.position,
        frame.orientation,
        motion.linear_velocity,
        motion.angular_velocity,
    );

    rigid_body_manager.add_kinematic_rigid_body(rigid_body)
}

pub fn setup_dynamic_rigid_body(
    rigid_body_manager: &mut RigidBodyManager,
    inertial_properties: InertialProperties,
    frame: ReferenceFrame,
    motion: Motion,
) -> DynamicRigidBodyID {
    let rigid_body = DynamicRigidBody::new(
        inertial_properties.mass(),
        inertial_properties.inertia_tensor().unaligned(),
        frame.position,
        frame.orientation,
        motion.linear_velocity,
        motion.angular_velocity,
    );

    rigid_body_manager.add_dynamic_rigid_body(rigid_body)
}
