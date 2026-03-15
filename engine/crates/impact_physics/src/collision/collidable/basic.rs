//! Basic implementation of [`Collidable`](collision::Collidable).

use crate::{
    collision::{
        self, CollidableDescriptor, CollidableOrder, CollidableWithId,
        collidable::{
            capsule::{
                CapsuleCollidable, generate_capsule_capsule_contact_manifold,
                generate_capsule_plane_contact_manifold, generate_capsule_sphere_contact_manifold,
            },
            plane::PlaneCollidable,
            sphere::{
                SphereCollidable, generate_sphere_plane_contact_manifold,
                generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::ContactManifold,
};
use impact_math::transform::Isometry3;

pub type CollisionWorld = collision::CollisionWorld<Collidable>;

#[derive(Clone, Debug)]
pub enum Collidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
}

#[derive(Clone, Debug)]
pub enum LocalCollidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
}

impl collision::Collidable for Collidable {
    type Local = LocalCollidable;
    type Context = ();

    fn from_descriptor(
        descriptor: &CollidableDescriptor<Self>,
        transform_to_world_space: &Isometry3,
    ) -> Self {
        match descriptor.local_collidable() {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(transform_to_world_space)),
            Self::Local::Capsule(capsule) => {
                Self::Capsule(capsule.transformed(transform_to_world_space))
            }
        }
    }

    fn generate_contact_manifold(
        _context: &(),
        collidable_a: &CollidableWithId<Self>,
        collidable_b: &CollidableWithId<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder {
        use Collidable::{Capsule, Plane, Sphere};

        match (collidable_a.collidable(), collidable_b.collidable()) {
            (Capsule(capsule_a), Capsule(capsule_b)) => {
                generate_capsule_capsule_contact_manifold(
                    capsule_a,
                    capsule_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Capsule(capsule), Sphere(sphere)) => {
                generate_capsule_sphere_contact_manifold(
                    capsule,
                    sphere,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), Capsule(capsule)) => {
                generate_capsule_sphere_contact_manifold(
                    capsule,
                    sphere,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Capsule(capsule), Plane(plane)) => {
                generate_capsule_plane_contact_manifold(
                    capsule,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), Capsule(capsule)) => {
                generate_capsule_plane_contact_manifold(
                    capsule,
                    plane,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Sphere(sphere_a), Sphere(sphere_b)) => {
                generate_sphere_sphere_contact_manifold(
                    sphere_a,
                    sphere_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), Plane(plane)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), Sphere(sphere)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_b.id(),
                    collidable_a.id(),
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
