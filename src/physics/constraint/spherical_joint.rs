//! Spherical (ball and socket) joints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::fph;
use impact_ecs::world::{Entity, World as ECSWorld};
use nalgebra::Vector3;

#[derive(Clone, Debug)]
pub struct SphericalJoint {
    pub body_a_entity: Entity,
    pub body_b_entity: Entity,
    pub offset_in_body_a: Vector3<fph>,
    pub offset_in_body_b: Vector3<fph>,
}

#[derive(Clone, Debug)]
pub struct PreparedSphericalJoint {
    attachment_point_displacement: Vector3<fph>,
}

impl TwoBodyConstraint for SphericalJoint {
    type Prepared = PreparedSphericalJoint;

    fn prepare(
        &self,
        _ecs_world: &ECSWorld,
        _body_a_entity: &Entity,
        _body_b_entity: &Entity,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Prepared {
        let body_a_attachment_point = body_a.position + body_a.orientation * self.offset_in_body_a;
        let body_b_attachment_point = body_b.position + body_b.orientation * self.offset_in_body_b;

        let attachment_point_displacement = body_a_attachment_point - body_b_attachment_point;

        PreparedSphericalJoint {
            attachment_point_displacement,
        }
    }
}

impl PreparedTwoBodyConstraint for PreparedSphericalJoint {
    type Impulses = fph;

    fn can_use_warm_impulses_from(&self, _other: &Self) -> bool {
        true
    }

    fn compute_impulses(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> fph {
        todo!()
    }

    fn clamp_impulses(&self, impulse: fph) -> fph {
        todo!()
    }

    fn apply_impulses_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        impulse: fph,
    ) {
        todo!()
    }

    fn apply_positional_correction_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        correction_factor: fph,
    ) {
        todo!()
    }
}
