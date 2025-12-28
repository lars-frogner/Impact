//! Spherical (ball and socket) joints.

use super::{AnchoredTwoBodyConstraint, ConstrainedBody, PreparedTwoBodyConstraint};
use crate::anchor::{TypedRigidBodyAnchorID, TypedRigidBodyAnchorRef};
use impact_math::vector::Vector3;

#[derive(Clone, Debug)]
pub struct SphericalJoint {
    pub anchor_a: TypedRigidBodyAnchorID,
    pub anchor_b: TypedRigidBodyAnchorID,
}

#[derive(Clone, Debug)]
pub struct ResolvedSphericalJoint<'a> {
    pub anchor_a: TypedRigidBodyAnchorRef<'a>,
    pub anchor_b: TypedRigidBodyAnchorRef<'a>,
}

#[derive(Clone, Debug)]
pub struct PreparedSphericalJoint {
    _attachment_point_displacement: Vector3,
}

impl AnchoredTwoBodyConstraint for SphericalJoint {
    type Prepared = PreparedSphericalJoint;

    fn anchors(&self) -> (&TypedRigidBodyAnchorID, &TypedRigidBodyAnchorID) {
        (&self.anchor_a, &self.anchor_b)
    }

    fn prepare<'a>(
        &self,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
        anchor_a: TypedRigidBodyAnchorRef<'a>,
        anchor_b: TypedRigidBodyAnchorRef<'a>,
    ) -> Self::Prepared {
        let body_a_position = body_a.position.aligned();
        let body_b_position = body_b.position.aligned();
        let body_a_orientation = body_a.orientation.aligned();
        let body_b_orientation = body_b.orientation.aligned();
        let anchor_a_point = anchor_a.point().aligned();
        let anchor_b_point = anchor_b.point().aligned();

        let body_a_attachment_point =
            body_a_position + body_a_orientation.rotate_vector(anchor_a_point.as_vector());

        let body_b_attachment_point =
            body_b_position + body_b_orientation.rotate_vector(anchor_b_point.as_vector());

        let attachment_point_displacement = body_a_attachment_point - body_b_attachment_point;

        PreparedSphericalJoint {
            _attachment_point_displacement: attachment_point_displacement.unaligned(),
        }
    }
}

impl PreparedTwoBodyConstraint for PreparedSphericalJoint {
    type Impulses = f32;

    fn can_use_warm_impulses_from(&self, _other: &Self) -> bool {
        true
    }

    fn compute_impulses(&self, _body_a: &ConstrainedBody, _body_b: &ConstrainedBody) -> f32 {
        0.0
    }

    fn clamp_impulses(&self, impulse: f32) -> f32 {
        impulse
    }

    fn apply_impulses_to_body_pair(
        &self,
        _body_a: &mut ConstrainedBody,
        _body_b: &mut ConstrainedBody,
        _impulse: f32,
    ) {
    }

    fn apply_positional_correction_to_body_pair(
        &self,
        _body_a: &mut ConstrainedBody,
        _body_b: &mut ConstrainedBody,
        _correction_factor: f32,
    ) {
    }
}
