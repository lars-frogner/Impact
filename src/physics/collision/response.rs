//! Collision response.

use super::Contact;
use crate::physics::fph;
use impact_ecs::world::World as ECSWorld;
use nalgebra::{Matrix3, Point3, Vector3};

fn a(ecs_world: &ECSWorld) {}

#[derive(Clone, Debug)]
struct CollidingBodyState<'a> {
    /// Inverse of the body's mass.
    inverse_mass: fph,
    /// Inverse of the body's inertia tensor (in world space).
    inverse_inertia_tensor: &'a Matrix3<fph>,
    /// Position of the body's center of mass (in world space).
    position: &'a Point3<fph>,
    /// Linear velocity of the body' center of mass (in world space).
    velocity: &'a Vector3<fph>,
    /// Angular velocity of the body about its center of mass (in world space).
    angular_velocity: &'a Vector3<fph>,
}

#[derive(Clone, Debug)]
struct CollisionResponse {
    /// Change in the linear momentum of a body.
    linear_momentum_change: Vector3<fph>,
    /// Change in the angular momentum of a body about its center of mass.
    angular_momentum_change: Vector3<fph>,
}

/// Computes the responses of two rigid bodies due to a single collisional
/// contact between them.
///
/// See Eberly (2004) Sect. 5.2.2 for derivation.
fn compute_body_responses_to_single_collisional_contact(
    body_a: &CollidingBodyState<'_>,
    body_b: &CollidingBodyState<'_>,
    contact: &Contact,
    restitution_coef: fph,
) -> (CollisionResponse, CollisionResponse) {
    let &CollidingBodyState {
        inverse_mass: m_inv_a,
        inverse_inertia_tensor: j_inv_a,
        position: p_a,
        velocity: v_a,
        angular_velocity: w_a,
    } = body_a;

    let &CollidingBodyState {
        inverse_mass: m_inv_b,
        inverse_inertia_tensor: j_inv_b,
        position: p_b,
        velocity: v_b,
        angular_velocity: w_b,
    } = body_b;

    let n = contact.surface_normal();

    let r_a = contact.position() - p_a;
    let r_b = contact.position() - p_b;

    let r_a_cross_n = r_a.cross(n);
    let r_b_cross_n = r_b.cross(n);

    let scalar_impulse = -(1.0 + restitution_coef)
        * (n.dot(&(v_a - v_b)) + w_a.dot(&r_a_cross_n) - w_b.dot(&r_b_cross_n))
        / (m_inv_a
            + m_inv_b
            + r_a_cross_n.dot(&(j_inv_a * r_a_cross_n))
            + r_b_cross_n.dot(&(j_inv_b * r_b_cross_n)));

    let impulse = n.scale(scalar_impulse);

    let response_a = CollisionResponse::from_impulse_at_displacement_from_com(impulse, &r_a);
    let response_b = CollisionResponse::from_impulse_at_displacement_from_com(-impulse, &r_b);

    (response_a, response_b)
}

/// Computes the response of a rigid body due to a single collisional contact
/// with a static object (no motion and infinite mass).
///
/// See Eberly (2004) Sect. 5.2.2 for derivation.
fn compute_body_response_to_single_collisional_contact_with_static_object(
    body: &CollidingBodyState<'_>,
    contact: &Contact,
    restitution_coef: fph,
) -> CollisionResponse {
    let &CollidingBodyState {
        inverse_mass: m_inv,
        inverse_inertia_tensor: j_inv,
        position: p,
        velocity: v,
        angular_velocity: w,
    } = body;

    let n = contact.surface_normal();
    let r = contact.position() - p;
    let r_cross_n = r.cross(n);

    let scalar_impulse = -(1.0 + restitution_coef) * (n.dot(v) + w.dot(&r_cross_n))
        / (m_inv + r_cross_n.dot(&(j_inv * r_cross_n)));

    let impulse = n.scale(scalar_impulse);

    CollisionResponse::from_impulse_at_displacement_from_com(impulse, &r)
}

impl CollisionResponse {
    fn from_impulse_at_displacement_from_com(
        impulse: Vector3<fph>,
        displacement: &Vector3<fph>,
    ) -> Self {
        let angular_momentum_change = displacement.cross(&impulse);
        Self {
            linear_momentum_change: impulse,
            angular_momentum_change,
        }
    }
}
