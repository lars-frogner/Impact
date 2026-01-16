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
use impact_math::{matrix::Matrix3C, point::Point3C};
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

define_setup_type! {
    target = DynamicRigidBodyID;
    /// The inertial properties of a dynamic rigid body.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicRigidBodyInertialProperties {
        // The mass of the rigid body.
        pub mass: f32,
        /// The center of mass of the rigid body.
        pub center_of_mass: Point3C,
        // The inertia tensor of the rigid body defined relative to the center
        // of mass.
        pub inertia_tensor: Matrix3C,
    }
}

#[roc]
impl DynamicRigidBodySubstance {
    #[roc(body = "{ mass_density }")]
    pub fn new(mass_density: f32) -> Self {
        Self { mass_density }
    }
}

#[roc]
impl DynamicRigidBodyInertialProperties {
    #[roc(body = "{ mass, center_of_mass, inertia_tensor }")]
    pub fn new(mass: f32, center_of_mass: Point3C, inertia_tensor: Matrix3C) -> Self {
        Self {
            mass,
            center_of_mass,
            inertia_tensor,
        }
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
        inertial_properties.inertia_tensor().compact(),
        frame.position,
        frame.orientation,
        motion.linear_velocity,
        motion.angular_velocity,
    );

    rigid_body_manager.add_dynamic_rigid_body(rigid_body)
}
