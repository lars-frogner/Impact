//! Capsule-shaped collidable.

use crate::{
    collision::{
        CollidableID,
        collidable::{plane::PlaneCollidable, sphere::SphereCollidable},
    },
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    material::ContactResponseParameters,
};
use impact_geometry::{
    Capsule, CapsuleC, Plane, Sphere,
    line::{
        parameter_of_closest_point_on_line_segment_to_point,
        parameters_of_closest_points_on_line_segments,
    },
};
use impact_math::{transform::Isometry3, vector::UnitVector3};

#[derive(Clone, Debug)]
pub struct CapsuleCollidable {
    capsule: CapsuleC,
    response_params: ContactResponseParameters,
}

impl CapsuleCollidable {
    pub fn new(capsule: CapsuleC, response_params: ContactResponseParameters) -> Self {
        Self {
            capsule,
            response_params,
        }
    }

    pub fn capsule(&self) -> &CapsuleC {
        &self.capsule
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }

    pub fn transformed(&self, transform: &Isometry3) -> Self {
        let capsule = self.capsule.aligned();
        let transformed_capsule = capsule.iso_transformed(transform).compact();
        Self {
            capsule: transformed_capsule,
            response_params: self.response_params,
        }
    }

    pub fn with_response_params(&self, response_params: ContactResponseParameters) -> Self {
        Self {
            capsule: self.capsule,
            response_params,
        }
    }
}

pub fn generate_capsule_capsule_contact_manifold(
    capsule_a: &CapsuleCollidable,
    capsule_b: &CapsuleCollidable,
    capsule_a_collidable_id: CollidableID,
    capsule_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_capsule_capsule_contact_geometry(
        &capsule_a.capsule.aligned(),
        &capsule_b.capsule.aligned(),
    ) {
        let id =
            super::contact_id_from_collidable_ids(capsule_a_collidable_id, capsule_b_collidable_id);

        let response_params = ContactResponseParameters::combined(
            capsule_a.response_params(),
            capsule_b.response_params(),
        );

        contact_manifold.add_contact(ContactWithID {
            id,
            contact: Contact {
                geometry,
                response_params,
            },
        });
    }
}

pub fn generate_capsule_sphere_contact_manifold(
    capsule: &CapsuleCollidable,
    sphere: &SphereCollidable,
    capsule_collidable_id: CollidableID,
    sphere_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_capsule_sphere_contact_geometry(
        &capsule.capsule().aligned(),
        &sphere.sphere().aligned(),
    ) {
        let id = super::contact_id_from_collidable_ids(capsule_collidable_id, sphere_collidable_id);

        let response_params = ContactResponseParameters::combined(
            capsule.response_params(),
            sphere.response_params(),
        );

        contact_manifold.add_contact(ContactWithID {
            id,
            contact: Contact {
                geometry,
                response_params,
            },
        });
    }
}

pub fn generate_capsule_plane_contact_manifold(
    capsule: &CapsuleCollidable,
    plane: &PlaneCollidable,
    capsule_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_capsule_plane_contact_geometry(
        &capsule.capsule().aligned(),
        &plane.plane().aligned(),
    ) {
        let id = super::contact_id_from_collidable_ids(capsule_collidable_id, plane_collidable_id);

        let response_params =
            ContactResponseParameters::combined(capsule.response_params(), plane.response_params());

        contact_manifold.add_contact(ContactWithID {
            id,
            contact: Contact {
                geometry,
                response_params,
            },
        });
    }
}

pub fn determine_capsule_capsule_contact_geometry(
    capsule_a: &Capsule,
    capsule_b: &Capsule,
) -> Option<ContactGeometry> {
    const EPSILON: f32 = 1e-8;

    let (segment_a_param, segment_b_param) = parameters_of_closest_points_on_line_segments(
        capsule_a.segment_start(),
        capsule_a.segment_vector(),
        capsule_b.segment_start(),
        capsule_b.segment_vector(),
    );

    let closest_point_on_a_segment =
        capsule_a.segment_start() + segment_a_param * capsule_a.segment_vector();
    let closest_point_on_b_segment =
        capsule_b.segment_start() + segment_b_param * capsule_b.segment_vector();

    let segment_displacement = closest_point_on_a_segment - closest_point_on_b_segment;
    let squared_segment_distance = segment_displacement.norm_squared();

    let max_segment_distance = capsule_a.radius() + capsule_b.radius();

    if squared_segment_distance > max_segment_distance.powi(2) {
        return None;
    }

    let segment_distance = squared_segment_distance.sqrt();

    let (surface_normal, penetration_depth) = if segment_distance > EPSILON {
        let surface_normal = UnitVector3::unchecked_from(segment_displacement / segment_distance);

        let penetration_depth = f32::max(0.0, max_segment_distance - segment_distance);

        (surface_normal, penetration_depth)
    } else {
        // The two line segments are intersecting

        // We pick any vector normal to B's segment
        let normal_vector_to_segment = capsule_b.segment_vector().any_orthogonal_vector();

        // Normalize and designate as surface normal
        let surface_normal =
            UnitVector3::normalized_from_if_above(normal_vector_to_segment, EPSILON)
                .unwrap_or_else(UnitVector3::unit_z);

        // Determine how far capsule A would have to be shifted against the
        // normal in order to no longer intersect capsule B
        let segment_a_dot_normal = capsule_a.segment_vector().dot(&surface_normal);

        let shift_to_clear_segment = if segment_a_dot_normal.is_sign_positive() {
            (1.0 - segment_a_param) * segment_a_dot_normal
        } else {
            -segment_a_param * segment_a_dot_normal
        };

        let penetration_depth = f32::max(0.0, max_segment_distance + shift_to_clear_segment);

        (surface_normal, penetration_depth)
    };

    let position = closest_point_on_b_segment + capsule_b.radius() * surface_normal;

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}

pub fn determine_capsule_sphere_contact_geometry(
    capsule: &Capsule,
    sphere: &Sphere,
) -> Option<ContactGeometry> {
    const EPSILON: f32 = 1e-8;

    let segment_param = parameter_of_closest_point_on_line_segment_to_point(
        capsule.segment_start(),
        capsule.segment_vector(),
        sphere.center(),
    );

    let closest_point_on_segment =
        capsule.segment_start() + segment_param * capsule.segment_vector();

    let segment_displacement = sphere.center() - closest_point_on_segment;
    let squared_segment_distance = segment_displacement.norm_squared();

    let max_segment_distance = sphere.radius() + capsule.radius();

    if squared_segment_distance > max_segment_distance.powi(2) {
        return None;
    }

    let segment_distance = squared_segment_distance.sqrt();

    let (capsule_surface_normal, penetration_depth) = if segment_distance > EPSILON {
        let capsule_surface_normal =
            UnitVector3::unchecked_from(segment_displacement / segment_distance);

        let penetration_depth = f32::max(0.0, max_segment_distance - segment_distance);

        (capsule_surface_normal, penetration_depth)
    } else {
        // The sphere center lies on the capsule line segment

        // We pick any vector normal to the capsule's segment
        let normal_vector_to_segment = capsule.segment_vector().any_orthogonal_vector();

        // Normalize and designate as surface normal
        let capsule_surface_normal =
            UnitVector3::normalized_from_if_above(normal_vector_to_segment, EPSILON)
                .unwrap_or_else(UnitVector3::unit_z);

        let penetration_depth = f32::max(0.0, max_segment_distance);

        (capsule_surface_normal, penetration_depth)
    };

    let surface_normal = -capsule_surface_normal;

    let position = sphere.center() + sphere.radius() * surface_normal;

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}

pub fn determine_capsule_plane_contact_geometry(
    capsule: &Capsule,
    plane: &Plane,
) -> Option<ContactGeometry> {
    let segment_start = *capsule.segment_start();
    let segment_end = capsule.segment_end();

    let segment_start_signed_dist = plane.compute_signed_distance(&segment_start);
    let segment_end_signed_dist = plane.compute_signed_distance(&segment_end);

    let (closest_segment_point, lowest_segment_signed_dist) =
        if segment_start_signed_dist <= segment_end_signed_dist {
            (segment_start, segment_start_signed_dist)
        } else {
            (segment_end, segment_end_signed_dist)
        };

    let penetration_depth = capsule.radius() - lowest_segment_signed_dist;

    if penetration_depth < 0.0 {
        return None;
    }

    let surface_normal = *plane.unit_normal();
    let position = closest_segment_point - lowest_segment_signed_dist * surface_normal;

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}
