//! Spherical collidable geometry.

use crate::physics::{
    collision::{CollidableID, geometry::plane::PlaneCollidableGeometry},
    constraint::contact::{Contact, ContactGeometry, ContactManifold},
    fph,
};
use impact_geometry::{Plane, Sphere};
use nalgebra::{Similarity3, UnitVector3, Vector3};

#[derive(Clone, Debug)]
pub struct SphereCollidableGeometry {
    sphere: Sphere<fph>,
}

impl SphereCollidableGeometry {
    pub fn new(sphere: Sphere<fph>) -> Self {
        Self { sphere }
    }

    pub fn sphere(&self) -> &Sphere<fph> {
        &self.sphere
    }

    pub fn transformed(&self, transform: &Similarity3<fph>) -> Self {
        Self {
            sphere: self.sphere.transformed(transform),
        }
    }
}

pub fn generate_sphere_sphere_contact_manifold(
    sphere_a: &SphereCollidableGeometry,
    sphere_b: &SphereCollidableGeometry,
    sphere_a_collidable_id: CollidableID,
    sphere_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_sphere_sphere_contact(&sphere_a.sphere, &sphere_b.sphere) {
        let id =
            super::contact_id_from_collidable_ids(sphere_a_collidable_id, sphere_b_collidable_id);
        contact_manifold.add_contact(Contact { id, geometry });
    }
}

pub fn generate_sphere_plane_contact_manifold(
    sphere: &SphereCollidableGeometry,
    plane: &PlaneCollidableGeometry,
    sphere_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_sphere_plane_contact(sphere.sphere(), plane.plane()) {
        let id = super::contact_id_from_collidable_ids(sphere_collidable_id, plane_collidable_id);
        contact_manifold.add_contact(Contact { id, geometry });
    }
}

pub fn determine_sphere_sphere_contact(
    sphere_a: &Sphere<fph>,
    sphere_b: &Sphere<fph>,
) -> Option<ContactGeometry> {
    let center_displacement = sphere_a.center() - sphere_b.center();
    let squared_center_distance = center_displacement.norm_squared();

    if squared_center_distance
        > sphere_a.radius_squared()
            + sphere_b.radius_squared()
            + 2.0 * sphere_a.radius() * sphere_b.radius()
    {
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

pub fn determine_sphere_plane_contact(
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
