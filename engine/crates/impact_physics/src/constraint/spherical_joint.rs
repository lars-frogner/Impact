//! Spherical (ball and socket) joints.

use super::{AnchoredTwoBodyConstraint, ConstrainedBody, PreparedTwoBodyConstraint};
use crate::{
    anchor::{TypedRigidBodyAnchorID, TypedRigidBodyAnchorRef},
    fph,
};
use nalgebra::Vector3;

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
    _attachment_point_displacement: Vector3<fph>,
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
        let body_a_attachment_point =
            body_a.position + body_a.orientation * anchor_a.point().coords;
        let body_b_attachment_point =
            body_b.position + body_b.orientation * anchor_b.point().coords;

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
