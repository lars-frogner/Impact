//! Spherical collidable.

use crate::{
    collision::{CollidableID, collidable::plane::PlaneCollidable},
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    fph,
    material::ContactResponseParameters,
};
use impact_geometry::{Plane, Sphere};
use nalgebra::{Isometry3, UnitVector3, Vector3};

#[derive(Clone, Debug)]
pub struct SphereCollidable {
    sphere: Sphere<fph>,
    response_params: ContactResponseParameters,
}

impl SphereCollidable {
    pub fn new(sphere: Sphere<fph>, response_params: ContactResponseParameters) -> Self {
        Self {
            sphere,
            response_params,
        }
    }

    pub fn sphere(&self) -> &Sphere<fph> {
        &self.sphere
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }

    pub fn transformed(&self, transform: &Isometry3<fph>) -> Self {
        Self {
            sphere: self.sphere.translated_and_rotated(transform),
            response_params: self.response_params,
        }
    }

    pub fn with_response_params(&self, response_params: ContactResponseParameters) -> Self {
        Self {
            sphere: self.sphere,
            response_params,
        }
    }
}

pub fn generate_sphere_sphere_contact_manifold(
    sphere_a: &SphereCollidable,
    sphere_b: &SphereCollidable,
    sphere_a_collidable_id: CollidableID,
    sphere_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) =
        determine_sphere_sphere_contact_geometry(&sphere_a.sphere, &sphere_b.sphere)
    {
        let id =
            super::contact_id_from_collidable_ids(sphere_a_collidable_id, sphere_b_collidable_id);

        let response_params = ContactResponseParameters::combined(
            sphere_a.response_params(),
            sphere_b.response_params(),
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

pub fn generate_sphere_plane_contact_manifold(
    sphere: &SphereCollidable,
    plane: &PlaneCollidable,
    sphere_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_sphere_plane_contact_geometry(sphere.sphere(), plane.plane())
    {
        let id = super::contact_id_from_collidable_ids(sphere_collidable_id, plane_collidable_id);

        let response_params =
            ContactResponseParameters::combined(sphere.response_params(), plane.response_params());

        contact_manifold.add_contact(ContactWithID {
            id,
            contact: Contact {
                geometry,
                response_params,
            },
        });
    }
}

pub fn determine_sphere_sphere_contact_geometry(
    sphere_a: &Sphere<fph>,
    sphere_b: &Sphere<fph>,
) -> Option<ContactGeometry> {
    let center_displacement = sphere_a.center() - sphere_b.center();
    let squared_center_distance = center_displacement.norm_squared();
    let max_center_distance = sphere_a.radius() + sphere_b.radius();

    if squared_center_distance > max_center_distance.powi(2) {
        return None;
    }

    let center_distance = squared_center_distance.sqrt();

    let surface_normal = if center_distance > 1e-8 {
        UnitVector3::new_unchecked(center_displacement.unscale(center_distance))
    } else {
        Vector3::z_axis()
    };

    let position = sphere_b.center() + surface_normal.scale(sphere_b.radius());

    let penetration_depth = fph::max(
        0.0,
        (sphere_a.radius() + sphere_b.radius()) - center_distance,
    );

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}

pub fn determine_sphere_plane_contact_geometry(
    sphere: &Sphere<fph>,
    plane: &Plane<fph>,
) -> Option<ContactGeometry> {
    let signed_distance = plane.compute_signed_distance(sphere.center());
    let penetration_depth = sphere.radius() - signed_distance;

    if penetration_depth < 0.0 {
        return None;
    }

    let surface_normal = *plane.unit_normal();
    let position = sphere.center() - surface_normal.scale(signed_distance);

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}
