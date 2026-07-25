//! Constant body-space force on part of a rigid body.

use crate::{
    anchor::{AnchorManager, DynamicRigidBodyAnchorID},
    force::ForceGeneratorRegistry,
    quantities::{ForceC, PositionC},
    rigid_body::RigidBodyManager,
};
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Manages all [`LocalForceGenerator`]s.
pub type LocalForceRegistry = ForceGeneratorRegistry<DynamicRigidBodyAnchorID, LocalForceGenerator>;

/// Generator for a constant body-space force applied to a specific point on
/// a dynamic rigid body.
#[derive(Clone, Debug)]
pub struct LocalForceGenerator {
    /// The force vector in the body-fixed frame.
    pub force: ForceC,
}

define_setup_type! {
    /// A constant force vector and the point where it is applied, all in the
    /// body-fixed frame.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct LocalForce {
        /// The force vector in the body-fixed frame.
        pub force: ForceC,
        /// The point where the force is applied, in the body's model space.
        pub point: PositionC,
    }
}

impl LocalForceGenerator {
    /// Applies the force to the appropriate dynamic rigid body.
    pub fn apply(
        &self,
        anchor_id: DynamicRigidBodyAnchorID,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) {
        let Some(anchor) = anchor_manager.dynamic().get(anchor_id) else {
            return;
        };

        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(anchor.rigid_body_id)
        else {
            return;
        };

        let local_force = self.force.aligned();
        let local_anchor_point = anchor.point.aligned();

        let force = rigid_body.transform_vector_from_body_to_world_space(&local_force);
        let anchor_point = rigid_body.transform_point_from_body_to_world_space(&local_anchor_point);

        rigid_body.apply_force(&force, &anchor_point);
    }
}

#[roc]
impl LocalForce {
    #[roc(body = "{ force, point }")]
    pub fn new(force: ForceC, point: PositionC) -> Self {
        Self { force, point }
    }
}
