//! Basic implementation of [`CollidableGeometry`](collision::CollidableGeometry).

use crate::physics::{
    collision::{
        self, Collidable, CollidableOrder,
        geometry::{
            plane::PlaneCollidableGeometry,
            sphere::{
                SphereCollidableGeometry, generate_sphere_plane_contact_manifold,
                generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::ContactManifold,
    fph,
};
use impact_geometry::{Plane, Sphere};
use nalgebra::Similarity3;

pub type CollisionWorld = collision::CollisionWorld<CollidableGeometry>;

#[derive(Clone, Debug)]
pub enum CollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
}

#[derive(Clone, Debug)]
pub enum LocalBasicCollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
}

impl CollidableGeometry {
    pub fn local_sphere(sphere: Sphere<fph>) -> LocalBasicCollidableGeometry {
        LocalBasicCollidableGeometry::Sphere(SphereCollidableGeometry::new(sphere))
    }

    pub fn local_plane(plane: Plane<fph>) -> LocalBasicCollidableGeometry {
        LocalBasicCollidableGeometry::Plane(PlaneCollidableGeometry::new(plane))
    }
}

impl collision::CollidableGeometry for CollidableGeometry {
    type Local = LocalBasicCollidableGeometry;
    type Context = ();

    fn from_local(geometry: &Self::Local, transform_to_world_space: Similarity3<fph>) -> Self {
        match geometry {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(&transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(&transform_to_world_space)),
        }
    }

    fn generate_contact_manifold(
        _context: &(),
        collidable_a: &Collidable<Self>,
        collidable_b: &Collidable<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder {
        use CollidableGeometry::{Plane, Sphere};

        match (&collidable_a.geometry, &collidable_b.geometry) {
            (Sphere(sphere_a), Sphere(sphere_b)) => {
                generate_sphere_sphere_contact_manifold(
                    sphere_a,
                    sphere_b,
                    collidable_a.id,
                    collidable_b.id,
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), Plane(plane)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_a.id,
                    collidable_b.id,
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), Sphere(sphere)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_b.id,
                    collidable_a.id,
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Plane(_), Plane(_)) => {
                // Not useful
                CollidableOrder::Original
            }
        }
    }
}
