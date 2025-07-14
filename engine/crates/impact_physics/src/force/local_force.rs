//! Constant body-space force on part of a rigid body.

use crate::{
    force::ForceGeneratorRegistry,
    quantities::{Force, Position},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Manages all [`LocalForceGenerator`]s.
pub type LocalForceRegistry = ForceGeneratorRegistry<LocalForceGeneratorID, LocalForceGenerator>;

define_component_type! {
    /// Identifier for a [`LocalForceGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct LocalForceGeneratorID(u64);
}

/// Generator for a constant body-space force applied to a specific point on
/// a dynamic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct LocalForceGenerator {
    /// The dynamic rigid body experiencing the force.
    pub rigid_body_id: DynamicRigidBodyID,
    /// The force and its point of application, all in the body's local
    /// reference frame.
    pub local_force: LocalForce,
}

define_setup_type! {
    target = LocalForceGeneratorID;
    /// A constant force vector and the point where it is applied, all in the body's
    /// local reference frame.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct LocalForce {
        /// The force vector in the body's local reference frame.
        pub force: Force,
        /// The point where the force is applied, in the body's local reference
        /// frame.
        pub point: Position,
    }
}

impl From<u64> for LocalForceGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl LocalForceGenerator {
    /// Applies the force to the appropriate dynamic rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager) {
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };
        self.local_force.apply(rigid_body);
    }
}

#[roc]
impl LocalForce {
    pub fn new(force: Force, point: Position) -> Self {
        Self { force, point }
    }

    /// Applies the force to the given dynamic rigid body.
    pub fn apply(&self, rigid_body: &mut DynamicRigidBody) {
        let force = rigid_body.transform_vector_from_body_to_world_space(&self.force);
        let point = rigid_body.transform_point_from_body_to_world_space(&self.point);
        rigid_body.apply_force(&force, &point);
    }
}
