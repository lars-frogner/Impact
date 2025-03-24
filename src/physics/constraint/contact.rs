//! Contact (collision) constraints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::{
    fph,
    material::{ContactResponseParameters, components::UniformContactResponseComp},
    motion::{self, Orientation, Position, Velocity},
};
use impact_ecs::world::{Entity, World as ECSWorld};
use nalgebra::{UnitQuaternion, UnitVector3, Vector3, vector};
use num_traits::Zero;
use std::ops::{Add, Mul, Sub};
use tinyvec::TinyVec;

/// Identifier for a contact.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContactID(u64);

#[derive(Clone, Debug)]
pub struct ContactManifold {
    contacts: TinyVec<[Contact; 4]>,
}

#[derive(Clone, Debug)]
pub struct Contact {
    pub id: ContactID,
    pub geometry: ContactGeometry,
}

#[derive(Clone, Debug)]
pub struct ContactGeometry {
    /// The world space position of the point on body B that penetrates
    /// deepest into body A.
    pub position: Position,
    /// The world space surface normal of body B at [`Self::position`].
    pub surface_normal: UnitVector3<fph>,
    /// The distance between the deepest penetration points on A and B
    /// along [`Self::surface_normal`]. This is always non-negative when the
    /// bodies are in contact.
    pub penetration_depth: fph,
}

#[derive(Clone, Debug)]
pub struct PreparedContact {
    local_position_on_a: Position,
    local_position_on_b: Position,
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

impl ContactID {
    pub fn from_two_u32(a: u32, b: u32) -> Self {
        Self((u64::from(a) << 32) | u64::from(b))
    }
}

impl ContactManifold {
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

impl Default for ContactManifold {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            geometry: ContactGeometry::default(),
            id: ContactID(u64::MAX),
        }
    }
}

impl ContactGeometry {
    /// Returns world space position of the point on body A that penetrates
    /// deepest into body B along the surface normal from
    /// [`Self::position_on_b`].
    pub fn position_on_a(&self) -> Position {
        self.position - self.surface_normal.scale(self.penetration_depth)
    }

    /// Returns world space position of the point on body B that penetrates
    /// deepest into body A.
    pub fn position_on_b(&self) -> Position {
        self.position
    }

    fn determine_effective_response_parameters(
        ecs_world: &ECSWorld,
        body_a_entity: &Entity,
        body_b_entity: &Entity,
    ) -> ContactResponseParameters {
        let body_a_response_params =
            Self::obtain_contact_response_parameters_for_body(ecs_world, body_a_entity);

        let body_b_response_params =
            Self::obtain_contact_response_parameters_for_body(ecs_world, body_b_entity);

        ContactResponseParameters::combined(&body_a_response_params, &body_b_response_params)
    }

    fn obtain_contact_response_parameters_for_body(
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

impl Default for ContactGeometry {
    fn default() -> Self {
        Self {
            position: Position::origin(),
            surface_normal: Vector3::z_axis(),
            penetration_depth: 0.0,
        }
    }
}

impl TwoBodyConstraint for ContactGeometry {
    type Prepared = PreparedContact;

    fn prepare(
        &self,
        ecs_world: &ECSWorld,
        body_a_entity: &Entity,
        body_b_entity: &Entity,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Prepared {
        let local_position_on_a =
            body_a.transform_point_from_world_to_body_frame(&self.position_on_a());
        let local_position_on_b =
            body_b.transform_point_from_world_to_body_frame(&self.position_on_b());

        let disp_a = self.position - body_a.position;
        let disp_b = self.position - body_b.position;

        let normal = self.surface_normal;
        let (tangent_1, tangent_2) = construct_tangent_vectors(&normal);

        let compute_effective_mass = |direction: &UnitVector3<fph>| {
            compute_effective_mass(body_a, body_b, &disp_a, &disp_b, direction)
        };

        let effective_mass_normal = compute_effective_mass(&normal);
        let effective_mass_tangent_1 = compute_effective_mass(&tangent_1);
        let effective_mass_tangent_2 = compute_effective_mass(&tangent_2);

        let ContactResponseParameters {
            restitution_coef,
            static_friction_coef,
            dynamic_friction_coef,
        } = Self::determine_effective_response_parameters(ecs_world, body_a_entity, body_b_entity);

        let velocity_a = compute_point_velocity(body_a, &disp_a);
        let velocity_b = compute_point_velocity(body_b, &disp_b);
        let relative_velocity = velocity_a - velocity_b;
        let slip_speed_squared =
            relative_velocity.dot(&tangent_1).powi(2) + relative_velocity.dot(&tangent_2).powi(2);

        let friction_coef = if slip_speed_squared < 1e-4 {
            static_friction_coef
        } else {
            dynamic_friction_coef
        };

        PreparedContact {
            local_position_on_a,
            local_position_on_b,
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

impl PreparedTwoBodyConstraint for PreparedContact {
    type Impulses = ContactImpulses;

    fn can_use_warm_impulses_from(&self, other: &Self) -> bool {
        // `max_deviation_angle = acos(1 - threshold)`
        const THRESHOLD: fph = 1e-2;

        let normal_matches = self.normal.dot(&other.normal) > 1.0 - THRESHOLD;

        // We also need to check one of the tangent directions in case a
        // small deviation in the normal has caused the tangents to flip
        let tangent_matches = self.tangent_1.dot(&other.tangent_1) > 1.0 - THRESHOLD;

        normal_matches && tangent_matches
    }

    fn compute_impulses(
        &self,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> ContactImpulses {
        let position_on_b =
            body_b.transform_point_from_body_to_world_frame(&self.local_position_on_b);

        let disp_a = position_on_b - body_a.position;
        let disp_b = position_on_b - body_b.position;

        let velocity_a = compute_point_velocity(body_a, &disp_a);
        let velocity_b = compute_point_velocity(body_b, &disp_b);

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

        // TODO: maybe this should be cached
        let position_on_b =
            body_b.transform_point_from_body_to_world_frame(&self.local_position_on_b);

        let disp_a = position_on_b - body_a.position;
        let disp_b = position_on_b - body_b.position;

        body_a.velocity += body_a.inverse_mass * momentum_change;
        body_a.angular_velocity += body_a.inverse_inertia_tensor * disp_a.cross(&momentum_change);

        body_b.velocity -= body_b.inverse_mass * momentum_change;
        body_b.angular_velocity -= body_b.inverse_inertia_tensor * disp_b.cross(&momentum_change);
    }

    fn apply_positional_correction_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        correction_factor: fph,
    ) {
        let position_on_a =
            body_a.transform_point_from_body_to_world_frame(&self.local_position_on_a);
        let position_on_b =
            body_b.transform_point_from_body_to_world_frame(&self.local_position_on_b);

        let penetration_depth = self.normal.dot(&(position_on_b - position_on_a));

        if penetration_depth <= 0.0 {
            return;
        }

        let disp_a = position_on_b - body_a.position;
        let disp_b = position_on_b - body_b.position;

        let effective_mass = compute_effective_mass(body_a, body_b, &disp_a, &disp_b, &self.normal);

        let pseudo_impulse = effective_mass * correction_factor * penetration_depth;

        let pseudo_momentum_change = self.normal.scale(pseudo_impulse);

        let pseudo_velocity_a = body_a.inverse_mass * pseudo_momentum_change;
        let pseudo_angular_velocity_a =
            body_a.inverse_inertia_tensor * disp_a.cross(&pseudo_momentum_change);

        let pseudo_velocity_b = -body_b.inverse_mass * pseudo_momentum_change;
        let pseudo_angular_velocity_b =
            -body_b.inverse_inertia_tensor * disp_b.cross(&pseudo_momentum_change);

        body_a.position += pseudo_velocity_a;
        pseudo_advance_orientation(&mut body_a.orientation, &pseudo_angular_velocity_a);

        body_b.position += pseudo_velocity_b;
        pseudo_advance_orientation(&mut body_b.orientation, &pseudo_angular_velocity_b);
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

impl Mul<fph> for ContactImpulses {
    type Output = Self;

    fn mul(self, rhs: fph) -> Self::Output {
        Self {
            normal: self.normal * rhs,
            tangent_1: self.tangent_1 * rhs,
            tangent_2: self.tangent_2 * rhs,
        }
    }
}

fn compute_point_velocity(body: &ConstrainedBody, disp: &Vector3<fph>) -> Velocity {
    body.velocity + body.angular_velocity.cross(disp)
}

fn compute_effective_mass(
    body_a: &ConstrainedBody,
    body_b: &ConstrainedBody,
    disp_a: &Vector3<fph>,
    disp_b: &Vector3<fph>,
    direction: &UnitVector3<fph>,
) -> fph {
    let disp_a_cross_dir = disp_a.cross(direction);
    let disp_b_cross_dir = disp_b.cross(direction);

    1.0 / (body_a.inverse_mass
        + body_b.inverse_mass
        + disp_a_cross_dir.dot(&(body_a.inverse_inertia_tensor * disp_a_cross_dir))
        + disp_b_cross_dir.dot(&(body_b.inverse_inertia_tensor * disp_b_cross_dir)))
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

fn pseudo_advance_orientation(
    orientation: &mut Orientation,
    pseudo_angular_velocity: &Vector3<fph>,
) {
    *orientation = UnitQuaternion::new_normalize(
        orientation.as_ref()
            + motion::compute_orientation_derivative(orientation, pseudo_angular_velocity),
    );
}
