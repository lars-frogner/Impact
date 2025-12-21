//! Constant acceleration.

use crate::{
    force::ForceGeneratorRegistry,
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use impact_math::vector::Vector3;
use roc_integration::roc;

/// Manages all [`ConstantAccelerationGenerator`]s.
pub type ConstantAccelerationRegistry =
    ForceGeneratorRegistry<ConstantAccelerationGeneratorID, ConstantAccelerationGenerator>;

define_component_type! {
    /// Identifier for a [`ConstantAccelerationGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct ConstantAccelerationGeneratorID(u64);
}

/// Generator for a constant world-space acceleration of the center of mass
/// of a dynamic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ConstantAccelerationGenerator {
    /// The dynamic rigid body experiencing the acceleration.
    pub rigid_body_id: DynamicRigidBodyID,
    /// The acceleration of the body's center of mass in world space.
    pub acceleration: ConstantAcceleration,
    padding: f32,
}

define_setup_type! {
    target = ConstantAccelerationGeneratorID;
    /// A constant acceleration vector.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ConstantAcceleration(Vector3);
}

impl From<u64> for ConstantAccelerationGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl ConstantAccelerationGenerator {
    pub fn new(rigid_body_id: DynamicRigidBodyID, acceleration: ConstantAcceleration) -> Self {
        Self {
            rigid_body_id,
            acceleration,
            padding: 0.0,
        }
    }

    /// Applies the acceleration to the appropriate dynamic rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager) {
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };
        self.acceleration.apply(rigid_body);
    }
}

#[roc]
impl ConstantAcceleration {
    /// The downward acceleration at the surface of Earth [m/s^2].
    #[roc(expr = "9.81")]
    pub const EARTH_DOWNWARD_ACCELERATION: f32 = 9.81;

    #[roc(body = "(acceleration,)")]
    pub fn new(acceleration: Vector3) -> Self {
        Self(acceleration)
    }

    /// Constant acceleration in the negative y-direction.
    #[roc(body = "new((0.0, -acceleration, 0.0))")]
    pub fn downward(acceleration: f32) -> Self {
        Self::new(Vector3::new(0.0, -acceleration, 0.0))
    }

    /// The downward gravitational acceleration at the surface of Earth.
    #[roc(body = "downward(earth_downward_acceleration)")]
    pub fn earth() -> Self {
        Self::downward(Self::EARTH_DOWNWARD_ACCELERATION)
    }

    /// Applies the acceleration to the given dynamic rigid body.
    pub fn apply(&self, rigid_body: &mut DynamicRigidBody) {
        let force = self.0 * rigid_body.mass();
        rigid_body.apply_force_at_center_of_mass(&force);
    }
}
