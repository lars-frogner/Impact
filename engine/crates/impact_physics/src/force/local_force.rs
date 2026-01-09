//! Constant body-space force on part of a rigid body.

use crate::{
    anchor::{AnchorManager, DynamicRigidBodyAnchorID},
    force::ForceGeneratorRegistry,
    quantities::{ForceP, PositionP},
    rigid_body::RigidBodyManager,
};
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Manages all [`LocalForceGenerator`]s.
pub type LocalForceRegistry = ForceGeneratorRegistry<LocalForceGeneratorID, LocalForceGenerator>;

define_component_type! {
    /// Identifier for a [`LocalForceGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct LocalForceGeneratorID(u64);
}

/// Generator for a constant body-space force applied to a specific point on
/// a dynamic rigid body.
#[derive(Clone, Debug)]
pub struct LocalForceGenerator {
    /// The anchor point where the force is applied.
    pub anchor: DynamicRigidBodyAnchorID,
    /// The force vector in the body-fixed frame.
    pub force: ForceP,
}

define_setup_type! {
    target = LocalForceGeneratorID;
    /// A constant force vector and the point where it is applied, all in the
    /// body-fixed frame.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct LocalForce {
        /// The force vector in the body-fixed frame.
        pub force: ForceP,
        /// The point where the force is applied, in the body's model space.
        pub point: PositionP,
    }
}

impl From<u64> for LocalForceGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<LocalForceGeneratorID> for u64 {
    fn from(id: LocalForceGeneratorID) -> Self {
        id.0
    }
}

impl LocalForceGenerator {
    /// Applies the force to the appropriate dynamic rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, anchor_manager: &AnchorManager) {
        let Some(anchor) = anchor_manager.dynamic().get(self.anchor) else {
            return;
        };

        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(anchor.rigid_body_id)
        else {
            return;
        };

        let local_force = self.force.unpack();
        let local_anchor_point = anchor.point.unpack();

        let force = rigid_body.transform_vector_from_body_to_world_space(&local_force);
        let anchor_point = rigid_body.transform_point_from_body_to_world_space(&local_anchor_point);

        rigid_body.apply_force(&force, &anchor_point);
    }
}

#[roc]
impl LocalForce {
    #[roc(body = "{ force, point }")]
    pub fn new(force: ForceP, point: PositionP) -> Self {
        Self { force, point }
    }
}
