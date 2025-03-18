//! Contact (collision) constraints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::{
    fph,
    material::{ContactResponseParameters, components::UniformContactResponseComp},
    motion::Position,
};
use impact_ecs::world::{Entity, World as ECSWorld};
use nalgebra::{UnitVector3, Vector3, vector};
use num_traits::Zero;
use std::ops::{Add, Sub};
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
    disp_a: Vector3<fph>,
    disp_b: Vector3<fph>,
    normal: UnitVector3<fph>,
    tangent_1: UnitVector3<fph>,
    tangent_2: UnitVector3<fph>,
    effective_mass_normal: fph,
    effective_mass_tangent_1: fph,
    effective_mass_tangent_2: fph,
    restitution_coef: fph,
    friction_coef: fph,
}

#[derive(Clone, Copy, Debug)]
pub struct ContactImpulses {
    normal: fph,
    tangent_1: fph,
    tangent_2: fph,
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

    fn prepare(
        &self,
        ecs_world: &ECSWorld,
        body_a_entity: &Entity,
        body_b_entity: &Entity,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Prepared {
        let disp_a = self.position - body_a.position;
        let disp_b = self.position - body_b.position;

        let normal = self.surface_normal;
        let (tangent_1, tangent_2) = construct_tangent_vectors(&normal);

        let compute_effective_mass = |direction: &UnitVector3<fph>| {
            let disp_a_cross_dir = disp_a.cross(direction);
            let disp_b_cross_dir = disp_b.cross(direction);

            1.0 / (body_a.inverse_mass
                + body_b.inverse_mass
                + disp_a_cross_dir.dot(&(body_a.inverse_inertia_tensor * disp_a_cross_dir))
                + disp_b_cross_dir.dot(&(body_b.inverse_inertia_tensor * disp_b_cross_dir)))
        };

        let effective_mass_normal = compute_effective_mass(&normal);
        let effective_mass_tangent_1 = compute_effective_mass(&tangent_1);
        let effective_mass_tangent_2 = compute_effective_mass(&tangent_2);

        let ContactResponseParameters {
            restitution_coef,
            static_friction_coef,
            dynamic_friction_coef,
        } = self.determine_effective_response_parameters(ecs_world, body_a_entity, body_b_entity);

        let velocity_a = body_a.velocity + body_a.angular_velocity.cross(&disp_a);
        let velocity_b = body_b.velocity + body_b.angular_velocity.cross(&disp_b);
        let relative_velocity = velocity_a - velocity_b;
        let slip_speed_squared =
            relative_velocity.dot(&tangent_1).powi(2) + relative_velocity.dot(&tangent_2).powi(2);

        let friction_coef = if slip_speed_squared < 1e-4 {
            static_friction_coef
        } else {
            dynamic_friction_coef
        };

        PreparedContact {
            disp_a,
            disp_b,
            normal,
            tangent_1,
            tangent_2,
            effective_mass_normal,
            effective_mass_tangent_1,
            effective_mass_tangent_2,
            restitution_coef,
            friction_coef,
        }
    }
}

impl Contact {
    fn determine_effective_response_parameters(
        &self,
        ecs_world: &ECSWorld,
        body_a_entity: &Entity,
        body_b_entity: &Entity,
    ) -> ContactResponseParameters {
        let body_a_response_params =
            self.obtain_contact_response_parameters_for_body(ecs_world, body_a_entity);

        let body_b_response_params =
            self.obtain_contact_response_parameters_for_body(ecs_world, body_b_entity);

        ContactResponseParameters::combined(&body_a_response_params, &body_b_response_params)
    }

    fn obtain_contact_response_parameters_for_body(
        &self,
        ecs_world: &ECSWorld,
        body_entity: &Entity,
    ) -> ContactResponseParameters {
        let entry = ecs_world.entity(body_entity);

        if let Some(params) = entry.get_component::<UniformContactResponseComp>() {
            return params.access().0;
        }

        ContactResponseParameters::default()
    }
}

impl PreparedTwoBodyConstraint for PreparedContact {
    type Impulses = ContactImpulses;

    fn compute_impulses(
        &self,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> ContactImpulses {
        let velocity_a = body_a.velocity + body_a.angular_velocity.cross(&self.disp_a);
        let velocity_b = body_b.velocity + body_b.angular_velocity.cross(&self.disp_b);

        let relative_velocity = velocity_a - velocity_b;

        let separating_velocity = self.normal.dot(&relative_velocity);

        let normal_impulse = -self.effective_mass_normal
            * (1.0 + self.restitution_coef)
            * separating_velocity.min(0.0);

        let tangent_1_impulse =
            -self.effective_mass_tangent_1 * self.tangent_1.dot(&relative_velocity);
        let tangent_2_impulse =
            -self.effective_mass_tangent_2 * self.tangent_2.dot(&relative_velocity);

        ContactImpulses {
            normal: normal_impulse,
            tangent_1: tangent_1_impulse,
            tangent_2: tangent_2_impulse,
        }
    }

    fn clamp_impulses(&self, impulses: ContactImpulses) -> ContactImpulses {
        let clamped_normal_impulse = fph::max(0.0, impulses.normal);

        let max_tangent_impulse_magnitude = self.friction_coef * clamped_normal_impulse;
        let tangent_impulse_magnitude =
            fph::sqrt(impulses.tangent_1.powi(2) + impulses.tangent_2.powi(2));

        let tangent_impulse_scaling = if tangent_impulse_magnitude > max_tangent_impulse_magnitude {
            max_tangent_impulse_magnitude / tangent_impulse_magnitude
        } else {
            1.0
        };

        let clamped_tangent_1_impulse = impulses.tangent_1 * tangent_impulse_scaling;
        let clamped_tangent_2_impulse = impulses.tangent_2 * tangent_impulse_scaling;

        ContactImpulses {
            normal: clamped_normal_impulse,
            tangent_1: clamped_tangent_1_impulse,
            tangent_2: clamped_tangent_2_impulse,
        }
    }

    fn apply_impulses_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        impulses: ContactImpulses,
    ) {
        let momentum_change = self.normal.scale(impulses.normal)
            + self.tangent_1.scale(impulses.tangent_1)
            + self.tangent_2.scale(impulses.tangent_2);

        body_a.velocity += body_a.inverse_mass * momentum_change;
        body_a.angular_velocity +=
            body_a.inverse_inertia_tensor * self.disp_a.cross(&momentum_change);

        body_b.velocity -= body_b.inverse_mass * momentum_change;
        body_b.angular_velocity -=
            body_b.inverse_inertia_tensor * self.disp_b.cross(&momentum_change);
    }
}

impl Zero for ContactImpulses {
    fn zero() -> Self {
        Self {
            normal: 0.0,
            tangent_1: 0.0,
            tangent_2: 0.0,
        }
    }

    fn is_zero(&self) -> bool {
        self.normal == 0.0 && self.tangent_1 == 0.0 && self.tangent_2 == 0.0
    }
}

impl Add for ContactImpulses {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            normal: self.normal + rhs.normal,
            tangent_1: self.tangent_1 + rhs.tangent_1,
            tangent_2: self.tangent_2 + rhs.tangent_2,
        }
    }
}

impl Sub for ContactImpulses {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            normal: self.normal - rhs.normal,
            tangent_1: self.tangent_1 - rhs.tangent_1,
            tangent_2: self.tangent_2 - rhs.tangent_2,
        }
    }
}

fn construct_tangent_vectors(
    surface_normal: &UnitVector3<fph>,
) -> (UnitVector3<fph>, UnitVector3<fph>) {
    const INV_SQRT_THREE: fph = 0.57735;

    let tangent_1 = UnitVector3::new_normalize(if surface_normal.x.abs() < INV_SQRT_THREE {
        // Since the normal is relatively close to lying in the yz-plane, we
        // project it onto the yz plane, rotate it 90 degrees within the plane
        // and use that as the (unnormalized) first tangent. This vector will
        // be sufficiently different from the normal to avoid numerical issues.
        vector![0.0, surface_normal.z, -surface_normal.y]
    } else {
        // If the normal lies far from the yz-plane, projecting it onto the
        // yz-plane could lead to degeneracy, so we project it onto the xy-
        // plane instead to construct the first tangent.
        vector![surface_normal.y, -surface_normal.x, 0.0]
    });

    let tangent_2 = UnitVector3::new_unchecked(surface_normal.cross(&tangent_1));

    (tangent_1, tangent_2)
}
