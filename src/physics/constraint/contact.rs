//! Contact (collision) constraints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::{fph, motion::Position};
use nalgebra::{UnitVector3, Vector3};
use tinyvec::TinyVec;

#[derive(Clone, Debug)]
pub struct ContactSet {
    contacts: TinyVec<[Contact; 4]>,
}

#[derive(Clone, Debug)]
pub struct Contact {
    pub position: Position,
    pub surface_normal: UnitVector3<fph>,
    pub penetration_depth: fph,
}

#[derive(Clone, Debug)]
pub struct PreparedContact {
    restitution_coef: fph,
    n: UnitVector3<fph>,
    r_a_cross_n: Vector3<fph>,
    r_b_cross_n: Vector3<fph>,
    effective_mass: fph,
}

impl ContactSet {
    pub fn new() -> Self {
        Self {
            contacts: TinyVec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    pub fn contacts(&self) -> &[Contact] {
        self.contacts.as_slice()
    }

    pub fn clear(&mut self) {
        self.contacts.clear();
    }

    pub fn add_contact(&mut self, contact: Contact) {
        self.contacts.push(contact);
    }
}

impl Default for ContactSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            position: Position::origin(),
            surface_normal: Vector3::z_axis(),
            penetration_depth: 0.0,
        }
    }
}

impl TwoBodyConstraint for Contact {
    type Prepared = PreparedContact;

    fn prepare(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> Self::Prepared {
        let n = self.surface_normal;

        let r_a = self.position - body_a.position;
        let r_b = self.position - body_b.position;

        let r_a_cross_n = r_a.cross(&n);
        let r_b_cross_n = r_b.cross(&n);

        let effective_mass = 1.0
            / (body_a.inverse_mass
                + body_b.inverse_mass
                + r_a_cross_n.dot(&(body_a.inverse_inertia_tensor * r_a_cross_n))
                + r_b_cross_n.dot(&(body_b.inverse_inertia_tensor * r_b_cross_n)));

        PreparedContact {
            restitution_coef: 0.7,
            n,
            r_a_cross_n,
            r_b_cross_n,
            effective_mass,
        }
    }
}

impl PreparedTwoBodyConstraint for PreparedContact {
    fn compute_scalar_impulse(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> fph {
        let v_a = body_a.velocity;
        let v_b = body_b.velocity;
        let w_a = body_a.angular_velocity;
        let w_b = body_b.angular_velocity;

        let separating_velocity =
            self.n.dot(&(v_a - v_b)) + w_a.dot(&self.r_a_cross_n) - w_b.dot(&self.r_b_cross_n);

        -self.effective_mass * (1.0 + self.restitution_coef) * separating_velocity.min(0.0)
    }

    fn clamp_scalar_impulse(&self, scalar_impulse: fph) -> fph {
        fph::max(0.0, scalar_impulse)
    }

    fn apply_scalar_impulse_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        scalar_impulse: fph,
    ) {
        body_a.velocity += self.n.scale(scalar_impulse * body_a.inverse_mass);
        body_a.angular_velocity +=
            body_a.inverse_inertia_tensor * self.r_a_cross_n.scale(scalar_impulse);

        body_b.velocity += self.n.scale(-scalar_impulse * body_b.inverse_mass);
        body_b.angular_velocity +=
            body_b.inverse_inertia_tensor * self.r_b_cross_n.scale(-scalar_impulse);
    }
}

impl PreparedContact {}
