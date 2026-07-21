//! Contact (collision) constraints.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::{
    material::ContactResponseParameters,
    quantities::{self, Orientation, Position, PositionC, Velocity},
};
use impact_math::{
    point::Point3,
    quaternion::UnitQuaternion,
    random::splitmix,
    vector::{UnitVector3, UnitVector3C, Vector3},
};
use std::ops::{Add, Mul, Sub};
use tinyvec::TinyVec;

/// A set of contact points representing the region where two bodies are in
/// contact.
#[derive(Clone, Debug)]
pub struct ContactManifold {
    contacts: TinyVec<[ContactWithID; 32]>,
}

#[derive(Clone, Debug)]
pub struct ContactWithID {
    /// A globally unique identifier for the contact.
    pub id: ContactID,
    pub contact: Contact,
}

/// A point of contact between two bodies.
#[derive(Clone, Debug, Default)]
pub struct Contact {
    /// The geometrical information about the contact.
    pub geometry: ContactGeometry,
    /// The combined reponse parameters for the contact.
    pub response_params: ContactResponseParameters,
}

/// Identifier for a [`Contact`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContactID(u64);

/// Geometrical information about a point of contact between two bodies
/// A and B.
#[derive(Clone, Debug)]
pub struct ContactGeometry {
    /// The world space position of the point on body B that penetrates
    /// deepest into body A.
    pub position: Position,
    /// The world space surface normal of body B at [`Self::position`].
    pub surface_normal: UnitVector3,
    /// The distance between the deepest penetration points on A and B
    /// along [`Self::surface_normal`]. This is always non-negative when the
    /// bodies are in contact.
    pub penetration_depth: f32,
}

/// Derived information about a contact useful for solving the perpendicular
/// (bounce) and tangential (friction) contact constraints.
#[derive(Clone, Debug)]
pub struct PreparedContact {
    /// The point on body A that penetrates deepest into body B (along the
    /// surface normal from [`Self::local_position_on_b`]), expressed in the
    /// body frame of A.
    local_position_on_a: PositionC,
    /// The point on body B that penetrates deepest into body A, expressed in
    /// the body frame of B.
    local_position_on_b: PositionC,
    /// The world space surface normal of body B at
    /// [`Self::local_position_on_b`].
    normal: UnitVector3C,
    /// A world space tangent direction of the surface of body B at
    /// [`Self::local_position_on_b`].
    tangent: UnitVector3C,
    /// The world space tangent direction completing the right-handed
    /// coordinate system defined by [`Self::normal`] and
    /// [`Self::tangent`].
    bitangent: UnitVector3C,
    effective_mass_normal: f32,
    effective_mass_tangent: f32,
    effective_mass_bitangent: f32,
    friction_coef: f32,
    target_separating_velocity: f32,
}

/// Impulses along the three axes of a surface-aligned coordinate system for a
/// contact.
#[derive(Clone, Copy, Debug)]
pub struct ContactImpulses {
    normal: f32,
    tangent: f32,
    bitangent: f32,
}

impl ContactImpulses {
    /// The impulse magnitude along the contact normal.
    #[inline]
    pub fn normal(&self) -> f32 {
        self.normal
    }

    /// The impulse magnitude along the contact tangent.
    #[inline]
    pub fn tangent(&self) -> f32 {
        self.tangent
    }

    /// The impulse magnitude along the contact bitangent.
    #[inline]
    pub fn bitangent(&self) -> f32 {
        self.bitangent
    }
}

impl ContactManifold {
    #[inline]
    pub fn new() -> Self {
        Self {
            contacts: TinyVec::new(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    #[inline]
    pub fn contacts(&self) -> &[ContactWithID] {
        self.contacts.as_slice()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.contacts.clear();
    }

    #[inline]
    pub fn add_contact(&mut self, contact: ContactWithID) {
        self.contacts.push(contact);
    }
}

impl Default for ContactManifold {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ContactWithID {
    #[inline]
    pub fn position(&self) -> &Position {
        &self.contact.geometry.position
    }

    #[inline]
    pub fn surface_normal(&self) -> &UnitVector3 {
        &self.contact.geometry.surface_normal
    }

    #[inline]
    pub fn penetration_depth(&self) -> f32 {
        self.contact.geometry.penetration_depth
    }
}

impl Default for ContactWithID {
    #[inline]
    fn default() -> Self {
        Self {
            id: ContactID(u64::MAX),
            contact: Contact::default(),
        }
    }
}

impl ContactID {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn from_two_u64(a: u64, b: u64) -> Self {
        Self(splitmix::random_u64_from_two_states(a, b))
    }

    #[inline]
    pub fn from_two_u64_and_n_indices<const N: usize>(a: u64, b: u64, indices: [usize; N]) -> Self {
        let mut id = splitmix::random_u64_from_two_states(a, b);
        for index in indices {
            // Mix in indices
            id = splitmix::random_u64_from_two_states(id, index as u64);
        }
        Self(id)
    }
}

impl ContactGeometry {
    /// Returns world space position of the point on body A that penetrates
    /// deepest into body B along the surface normal from
    /// [`Self::position_on_b`].
    #[inline]
    pub fn position_on_a(&self) -> Position {
        self.position - self.penetration_depth * self.surface_normal
    }

    /// Returns world space position of the point on body B that penetrates
    /// deepest into body A.
    #[inline]
    pub fn position_on_b(&self) -> Position {
        self.position
    }
}

impl Default for ContactGeometry {
    #[inline]
    fn default() -> Self {
        Self {
            position: Position::origin(),
            surface_normal: UnitVector3::unit_z(),
            penetration_depth: 0.0,
        }
    }
}

impl TwoBodyConstraint for Contact {
    type Prepared = PreparedContact;

    fn prepare(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> Self::Prepared {
        // For slower impacts than this, we always use a restitution coefficient
        // of zero so that resting contacts become less jittery
        const NORMAL_SPEED_FOR_BOUNCE: f32 = 0.4;

        const SQUARED_SLIP_SPEED_FOR_DYNAMIC_FRICTION: f32 = 1e-4;

        let body_a_position = body_a.position.aligned();
        let body_b_position = body_b.position.aligned();

        let local_position_on_a =
            body_a.transform_point_from_world_to_body_frame(&self.geometry.position_on_a());
        let local_position_on_b =
            body_b.transform_point_from_world_to_body_frame(&self.geometry.position_on_b());

        // World space displacements from the center of mass of each body to
        // the reference contact point (taken to be on body B)
        let disp_a = self.geometry.position - body_a_position;
        let disp_b = self.geometry.position - body_b_position;

        let normal = self.geometry.surface_normal;
        let (tangent_1, tangent_2) = construct_tangent_vectors(&normal);

        let compute_effective_mass =
            |direction| compute_effective_mass(body_a, body_b, &disp_a, &disp_b, direction);

        let effective_mass_normal = compute_effective_mass(&normal);
        let effective_mass_tangent_1 = compute_effective_mass(&tangent_1);
        let effective_mass_tangent_2 = compute_effective_mass(&tangent_2);

        let ContactResponseParameters {
            restitution_coef,
            static_friction_coef,
            dynamic_friction_coef,
        } = self.response_params;

        // World space velocity of the reference contact point when considered
        // fixed to body A and B respectively
        let velocity_a = compute_point_velocity(body_a, &disp_a);
        let velocity_b = compute_point_velocity(body_b, &disp_b);

        let relative_velocity = velocity_a - velocity_b;

        let separating_velocity = normal.dot(&relative_velocity);

        let target_separating_velocity = if separating_velocity.abs() >= NORMAL_SPEED_FOR_BOUNCE {
            -restitution_coef * separating_velocity
        } else {
            0.0
        };

        // We need the speed at which the surfaces of the two bodies are
        // slipping at the contact point to determine whether to apply static
        // or dynamic friction. Note that the body velocities have not yet been
        // advanced based on non-constraint forces for this frame, which is
        // crucial for correctly identifying the kind of friction to use.
        let slip_speed_squared =
            relative_velocity.dot(&tangent_1).powi(2) + relative_velocity.dot(&tangent_2).powi(2);

        let friction_coef = if slip_speed_squared >= SQUARED_SLIP_SPEED_FOR_DYNAMIC_FRICTION {
            dynamic_friction_coef
        } else {
            static_friction_coef
        };

        PreparedContact {
            local_position_on_a: local_position_on_a.compact(),
            local_position_on_b: local_position_on_b.compact(),
            normal: normal.compact(),
            tangent: tangent_1.compact(),
            bitangent: tangent_2.compact(),
            effective_mass_normal,
            effective_mass_tangent: effective_mass_tangent_1,
            effective_mass_bitangent: effective_mass_tangent_2,
            friction_coef,
            target_separating_velocity,
        }
    }
}

impl PreparedTwoBodyConstraint for PreparedContact {
    type Impulses = ContactImpulses;

    fn can_use_warm_impulses_from(&self, other: &Self) -> bool {
        // `max_deviation_angle = acos(1 - threshold)`
        const THRESHOLD: f32 = 1e-2;

        let normal_matches = self.normal.dot(&other.normal) > 1.0 - THRESHOLD;

        // We also need to check one of the tangent directions in case a
        // small deviation in the normal has caused the tangents to flip
        let tangent_matches = self.tangent.dot(&other.tangent) > 1.0 - THRESHOLD;

        normal_matches && tangent_matches
    }

    fn compute_impulses(
        &self,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> ContactImpulses {
        let normal = self.normal.aligned();
        let tangent = self.tangent.aligned();
        let bitangent = self.bitangent.aligned();
        let local_position_on_b = self.local_position_on_b.aligned();
        let body_a_position = body_a.position.aligned();
        let body_b_position = body_b.position.aligned();

        let position_on_b = body_b.transform_point_from_body_to_world_frame(&local_position_on_b);

        // These could have been cached from `ContactGeometry::prepare`, but
        // probably not worth the extra space as they are cheap to recompute
        let disp_a = position_on_b - body_a_position;
        let disp_b = position_on_b - body_b_position;

        // At this point, the body velocities have been advanced based on
        // non-constraint forces and may also have been updated with
        // constraint impulses
        let velocity_a = compute_point_velocity(body_a, &disp_a);
        let velocity_b = compute_point_velocity(body_b, &disp_b);

        let relative_velocity = velocity_a - velocity_b;

        let separating_velocity = normal.dot(&relative_velocity);

        let normal_impulse =
            -self.effective_mass_normal * (separating_velocity - self.target_separating_velocity);

        let tangent_impulse = -self.effective_mass_tangent * tangent.dot(&relative_velocity);
        let bitangent_impulse = -self.effective_mass_bitangent * bitangent.dot(&relative_velocity);

        ContactImpulses {
            normal: normal_impulse,
            tangent: tangent_impulse,
            bitangent: bitangent_impulse,
        }
    }

    fn clamp_impulses(&self, impulses: ContactImpulses) -> ContactImpulses {
        // This ensures that the total normal impulse can only push the bodies apart
        let clamped_normal_impulse = f32::max(0.0, impulses.normal);

        // The impulse version of Coulomb's friction law determines the maximum
        // frictional impulse
        let max_tangent_impulse_magnitude = self.friction_coef * clamped_normal_impulse;

        let tangent_impulse_magnitude =
            f32::sqrt(impulses.tangent.powi(2) + impulses.bitangent.powi(2));

        // The tangential impulse must be scaled down if it exceeds the maximum
        let tangent_impulse_scaling = if tangent_impulse_magnitude > max_tangent_impulse_magnitude {
            max_tangent_impulse_magnitude / tangent_impulse_magnitude
        } else {
            1.0
        };

        let clamped_tangent_impulse = impulses.tangent * tangent_impulse_scaling;
        let clamped_bitangent_impulse = impulses.bitangent * tangent_impulse_scaling;

        ContactImpulses {
            normal: clamped_normal_impulse,
            tangent: clamped_tangent_impulse,
            bitangent: clamped_bitangent_impulse,
        }
    }

    fn apply_impulses_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        impulses: ContactImpulses,
    ) {
        let normal = self.normal.aligned();
        let tangent = self.tangent.aligned();
        let bitangent = self.bitangent.aligned();
        let local_position_on_b = self.local_position_on_b.aligned();
        let body_a_position = body_a.position.aligned();
        let body_b_position = body_b.position.aligned();
        let body_a_inverse_inertia_tensor = body_a.inverse_inertia_tensor.aligned();
        let body_b_inverse_inertia_tensor = body_b.inverse_inertia_tensor.aligned();

        let momentum_change =
            impulses.normal * normal + impulses.tangent * tangent + impulses.bitangent * bitangent;

        // TODO: maybe this should be cached from `compute_impulses`
        let position_on_b = body_b.transform_point_from_body_to_world_frame(&local_position_on_b);

        let disp_a = position_on_b - body_a_position;
        let disp_b = position_on_b - body_b_position;

        let mut body_a_velocity = body_a.velocity.aligned();
        let mut body_b_velocity = body_b.velocity.aligned();
        let mut body_a_angular_velocity = body_a.angular_velocity.aligned();
        let mut body_b_angular_velocity = body_b.angular_velocity.aligned();

        body_a_velocity += body_a.inverse_mass * momentum_change;
        body_b_velocity -= body_b.inverse_mass * momentum_change;

        body_a_angular_velocity += body_a_inverse_inertia_tensor * disp_a.cross(&momentum_change);
        body_b_angular_velocity -= body_b_inverse_inertia_tensor * disp_b.cross(&momentum_change);

        body_a.velocity = body_a_velocity.compact();
        body_b.velocity = body_b_velocity.compact();
        body_a.angular_velocity = body_a_angular_velocity.compact();
        body_b.angular_velocity = body_b_angular_velocity.compact();
    }

    fn apply_positional_correction_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        correction_factor: f32,
    ) {
        let normal = self.normal.aligned();
        let local_position_on_a = self.local_position_on_a.aligned();
        let local_position_on_b = self.local_position_on_b.aligned();
        let body_a_position = body_a.position.aligned();
        let body_b_position = body_b.position.aligned();
        let body_a_inverse_inertia_tensor = body_a.inverse_inertia_tensor.aligned();
        let body_b_inverse_inertia_tensor = body_b.inverse_inertia_tensor.aligned();

        // We are now correcting body positions and orientations iteratively.
        // In principle, we should rerun collision detection to obtain the new
        // contact geometry. Since that's not feasible in practice, we instead
        // assume that the contact point on each body stays fixed on the body
        // and that the surface normal does not change. We can then compute the
        // world space positions of the points based on the current body
        // positions and orientations and combine with the contact normal to
        // estimate the current penetration depth. TODO: it's probably more
        // accurate to fix the normal in body B space instead of world space.

        let position_on_a = body_a.transform_point_from_body_to_world_frame(&local_position_on_a);
        let position_on_b = body_b.transform_point_from_body_to_world_frame(&local_position_on_b);

        let penetration_depth = normal.dot(&(position_on_b - position_on_a));

        // We don't want to touch the bodies if they are no longer penetrating
        if penetration_depth <= 0.0 {
            return;
        }

        let disp_a = position_on_b - body_a_position;
        let disp_b = position_on_b - body_b_position;

        let effective_mass = compute_effective_mass(body_a, body_b, &disp_a, &disp_b, &normal);

        // We are trying to compute the impulse that would yield a change in
        // linear and angular velocity that over one time step would move the
        // bodies so as to correct a certain fraction of the interpenetration.
        // Instead of modifying the body velocites, which would add unphysical
        // kinetic energy to the system, we compute the deltas in linear and
        // angular velocity caused by the impulse and use those to update the
        // positions and orientations directly. We don't need the time step
        // duration as this gets cancelled out in the equations.

        let pseudo_impulse = effective_mass * correction_factor * penetration_depth;

        let pseudo_momentum_change = pseudo_impulse * normal;

        let pseudo_velocity_a = body_a.inverse_mass * pseudo_momentum_change;
        let pseudo_angular_velocity_a =
            body_a_inverse_inertia_tensor * disp_a.cross(&pseudo_momentum_change);

        let pseudo_velocity_b = -body_b.inverse_mass * pseudo_momentum_change;
        let pseudo_angular_velocity_b =
            -body_b_inverse_inertia_tensor * disp_b.cross(&pseudo_momentum_change);

        let mut body_a_position = body_a.position.aligned();
        let mut body_b_position = body_b.position.aligned();
        let mut body_a_orientation = body_a.orientation.aligned();
        let mut body_b_orientation = body_b.orientation.aligned();

        body_a_position += pseudo_velocity_a;
        body_b_position += pseudo_velocity_b;

        body_a_orientation =
            pseudo_advanced_orientation(&body_a_orientation, &pseudo_angular_velocity_a);
        body_b_orientation =
            pseudo_advanced_orientation(&body_b_orientation, &pseudo_angular_velocity_b);

        body_a.position = body_a_position.compact();
        body_b.position = body_b_position.compact();
        body_a.orientation = body_a_orientation.compact();
        body_b.orientation = body_b_orientation.compact();
    }
}

impl Default for ContactImpulses {
    fn default() -> Self {
        Self {
            normal: 0.0,
            tangent: 0.0,
            bitangent: 0.0,
        }
    }
}

impl Add for ContactImpulses {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            normal: self.normal + rhs.normal,
            tangent: self.tangent + rhs.tangent,
            bitangent: self.bitangent + rhs.bitangent,
        }
    }
}

impl Sub for ContactImpulses {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            normal: self.normal - rhs.normal,
            tangent: self.tangent - rhs.tangent,
            bitangent: self.bitangent - rhs.bitangent,
        }
    }
}

impl Mul<f32> for ContactImpulses {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            normal: self.normal * rhs,
            tangent: self.tangent * rhs,
            bitangent: self.bitangent * rhs,
        }
    }
}

/// Analyzes the contact manifold for a collision between two objects and
/// determines whether the contacts will keep the objects interlocked rather
/// than pushing them apart.
///
/// The objects are considered interlocked when the contact penetration vectors
/// are strongly misaligned, meaning that the positional corrections would fight
/// to push the objects apart along opposing directions.
pub fn objects_in_contact_are_interlocked(contact_manifold: &ContactManifold) -> bool {
    const ALIGNMENT_THRESHOLD: f32 = 0.1;

    let mut abs_penetration_sum = 0.0;
    let mut penetration_vector_sum = Vector3::zeros();

    for contact in contact_manifold.contacts() {
        let penetration_depth = contact.contact.geometry.penetration_depth;
        if penetration_depth <= 0.0 {
            continue;
        }
        abs_penetration_sum += penetration_depth;
        penetration_vector_sum += penetration_depth * contact.contact.geometry.surface_normal;
    }

    if abs_penetration_sum < 1e-6 {
        // Effectively no penetration to correct
        return false;
    }

    // 1.0 when all corrections would be aligned, 0.0 when they would cancel
    let correction_alignment_factor =
        penetration_vector_sum.norm_squared() / abs_penetration_sum.powi(2);

    correction_alignment_factor < ALIGNMENT_THRESHOLD
}

/// Creates a synthetic contact designed to separate two interlocked objects by
/// substituting their contact manifold for it.
///
/// The method looks at the distribution of contact points and estimates the
/// axis along which they are least separated. It uses this to create a contact
/// that, when used for positional correction, pushes the objects apart along
/// that axis enough to remove the separation.
///
/// No contact will be returned if the contact manifold is empty or effectively
/// a single point.
pub fn create_separating_contact_for_interlocked_objects(
    body_a: &ConstrainedBody,
    body_b: &ConstrainedBody,
    contact_manifold: &ContactManifold,
) -> Option<ContactWithID> {
    let contacts = contact_manifold.contacts();

    if contacts.is_empty() {
        return None;
    }

    // Find the axis with the maximum separation of contacts
    let major_displacement = find_max_displacement_vector(contacts, |point| *point);
    let major_axis = UnitVector3::normalized_from_if_above(major_displacement, 1e-6)?;

    // Project the contact positions onto the plane perpendicular to the major
    // axis and find the axis with the maximum separation inside that plane.
    // This gives the axis with the maximum separation perpendicular to the
    // major axis.
    let middle_displacement = find_max_displacement_vector(contacts, |point| {
        point - point.as_vector().dot(&major_axis) * major_axis
    });
    let Some(middle_axis) = UnitVector3::normalized_from_if_above(middle_displacement, 1e-6) else {
        // If the contacts all lie along the major axis, fall back to using the
        // major axis as the separating axis
        return create_contact_separating_along_axis(body_a, body_b, contacts, major_axis);
    };

    // Calculate the axis perpendicular to the major and middle axis and use
    // that as the separating axis
    let minor_axis = UnitVector3::normalized_from(major_axis.cross(&middle_axis));
    create_contact_separating_along_axis(body_a, body_b, contacts, minor_axis).or_else(|| {
        // If the contacts all lie in the plane perpendicular to the minor axis,
        // fall back to using the middle axis as the separating axis
        create_contact_separating_along_axis(body_a, body_b, contacts, middle_axis)
    })
}

fn find_max_displacement_vector(
    contacts: &[ContactWithID],
    map_point: impl Fn(&Point3) -> Point3,
) -> Vector3 {
    let mut max_squared_dist = f32::NEG_INFINITY;
    let mut best_pair = [usize::MAX; 2];

    for (i, contact_i) in contacts.iter().enumerate().take(contacts.len() - 1) {
        for (j, contact_j) in contacts.iter().enumerate().skip(i + 1) {
            let squared_dist = Point3::squared_distance_between(
                &map_point(contact_i.position()),
                &map_point(contact_j.position()),
            );
            if squared_dist > max_squared_dist {
                max_squared_dist = squared_dist;
                best_pair = [i, j];
            }
        }
    }

    let [i, j] = best_pair;
    map_point(contacts[j].position()) - map_point(contacts[i].position())
}

fn create_contact_separating_along_axis(
    body_a: &ConstrainedBody,
    body_b: &ConstrainedBody,
    contacts: &[ContactWithID],
    mut separating_axis: UnitVector3,
) -> Option<ContactWithID> {
    // The axis will be the "surface normal" of body B, which corresponds to the
    // direction we will push body A. We orient the axis so that we push the COM
    // of A away from the COM of B.
    if separating_axis.dot(&(body_a.position - body_b.position).aligned()) < 0.0 {
        separating_axis = -separating_axis;
    }

    // Determine the contacts with the minimum and maximum displacement along
    // the separating axis
    let mut min_displacement = f32::INFINITY;
    let mut max_displacement = f32::NEG_INFINITY;
    let mut most_separated_pair = [usize::MAX; 2];

    for (i, contact) in contacts.iter().enumerate() {
        let displacement_along_axis = contact.position().as_vector().dot(&separating_axis);

        if displacement_along_axis < min_displacement {
            min_displacement = displacement_along_axis;
            most_separated_pair[0] = i;
        }
        if displacement_along_axis > max_displacement {
            max_displacement = displacement_along_axis;
            most_separated_pair[1] = i;
        }
    }

    let [i, j] = most_separated_pair;
    if i == j {
        // The contacts all lie in the plane perpendicular to the axis, so we do
        // not attempt to proceed with separation along this axis
        return None;
    }

    // The separation required to move the lower contact to the upper contact
    let required_separation = max_displacement - min_displacement;

    let contact_i = &contacts[i];
    let contact_j = &contacts[j];

    let separating_contact = Contact {
        geometry: ContactGeometry {
            position: *contact_i.position(),
            surface_normal: separating_axis,
            penetration_depth: required_separation,
        },
        // The purpose of this contact is positional correction. To minimize
        // side effects from velocity correction, we use response parameters
        // that kill the relative velocity.
        response_params: ContactResponseParameters::new(0.0, f32::INFINITY, f32::INFINITY),
    };

    // The choice of ID is a bit arbitrary. The main point is that it does not
    // alias existing IDs.
    let id = ContactID::from_two_u64(contact_i.id.as_u64(), contact_j.id.as_u64());

    Some(ContactWithID {
        id,
        contact: separating_contact,
    })
}

#[inline]
fn compute_point_velocity(body: &ConstrainedBody, disp: &Vector3) -> Velocity {
    body.velocity.aligned() + body.angular_velocity.aligned().cross(disp)
}

#[inline]
fn compute_effective_mass(
    body_a: &ConstrainedBody,
    body_b: &ConstrainedBody,
    disp_a: &Vector3,
    disp_b: &Vector3,
    direction: &UnitVector3,
) -> f32 {
    let body_a_inverse_inertia_tensor = body_a.inverse_inertia_tensor.aligned();
    let body_b_inverse_inertia_tensor = body_b.inverse_inertia_tensor.aligned();

    let disp_a_cross_dir = disp_a.cross(direction);
    let disp_b_cross_dir = disp_b.cross(direction);

    let effective_mass = 1.0
        / (body_a.inverse_mass
            + body_b.inverse_mass
            + disp_a_cross_dir.dot(&(body_a_inverse_inertia_tensor * disp_a_cross_dir))
            + disp_b_cross_dir.dot(&(body_b_inverse_inertia_tensor * disp_b_cross_dir)));

    debug_assert!(effective_mass.is_finite());

    effective_mass
}

#[inline]
fn construct_tangent_vectors(surface_normal: &UnitVector3) -> (UnitVector3, UnitVector3) {
    const INV_SQRT_THREE: f32 = 0.57735;

    let tangent_1 = UnitVector3::normalized_from(if surface_normal.x().abs() < INV_SQRT_THREE {
        // Since the normal is relatively close to lying in the yz-plane, we
        // project it onto the yz plane, rotate it 90 degrees within the plane
        // and use that as the (unnormalized) first tangent. This vector will
        // be sufficiently different from the normal to avoid numerical issues.
        Vector3::new(0.0, surface_normal.z(), -surface_normal.y())
    } else {
        // If the normal lies far from the yz-plane, projecting it onto the
        // yz-plane could lead to degeneracy, so we project it onto the xy-
        // plane instead to construct the first tangent.
        Vector3::new(surface_normal.y(), -surface_normal.x(), 0.0)
    });

    let tangent_2 = UnitVector3::unchecked_from(surface_normal.cross(&tangent_1));

    (tangent_1, tangent_2)
}

#[inline]
fn pseudo_advanced_orientation(
    orientation: &Orientation,
    pseudo_angular_velocity: &Vector3,
) -> Orientation {
    UnitQuaternion::normalized_from(
        orientation.as_quaternion()
            + quantities::compute_orientation_derivative(orientation, pseudo_angular_velocity),
    )
}
