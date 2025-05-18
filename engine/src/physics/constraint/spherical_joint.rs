//! Spherical (ball and socket) joints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::fph;
use impact_ecs::world::{EntityID, World as ECSWorld};
use nalgebra::Vector3;

#[derive(Clone, Debug)]
pub struct SphericalJoint {
    pub body_a_entity_id: EntityID,
    pub body_b_entity_id: EntityID,
    pub offset_in_body_a: Vector3<fph>,
    pub offset_in_body_b: Vector3<fph>,
}

#[derive(Clone, Debug)]
pub struct PreparedSphericalJoint {
    _attachment_point_displacement: Vector3<fph>,
}

impl TwoBodyConstraint for SphericalJoint {
    type Prepared = PreparedSphericalJoint;

    fn prepare(
        &self,
        _ecs_world: &ECSWorld,
        _body_a_entity_id: EntityID,
        _body_b_entity_id: EntityID,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Prepared {
        let body_a_attachment_point = body_a.position + body_a.orientation * self.offset_in_body_a;
        let body_b_attachment_point = body_b.position + body_b.orientation * self.offset_in_body_b;

        let attachment_point_displacement = body_a_attachment_point - body_b_attachment_point;

        PreparedSphericalJoint {
            _attachment_point_displacement: attachment_point_displacement,
        }
    }
}

impl PreparedTwoBodyConstraint for PreparedSphericalJoint {
    type Impulses = fph;

    fn can_use_warm_impulses_from(&self, _other: &Self) -> bool {
        true
    }

    fn compute_impulses(&self, _body_a: &ConstrainedBody, _body_b: &ConstrainedBody) -> fph {
        0.0
    }

    fn clamp_impulses(&self, impulse: fph) -> fph {
        impulse
    }

    fn apply_impulses_to_body_pair(
        &self,
        _body_a: &mut ConstrainedBody,
        _body_b: &mut ConstrainedBody,
        _impulse: fph,
    ) {
    }

    fn apply_positional_correction_to_body_pair(
        &self,
        _body_a: &mut ConstrainedBody,
        _body_b: &mut ConstrainedBody,
        _correction_factor: fph,
    ) {
    }
}
