//! Implementation of [`Collidable`](collision::Collidable) that includes voxel
//! geometry.

use crate::voxel::{VoxelObjectID, VoxelObjectManager};
use impact_physics::{
    collision::{
        self, CollidableDescriptor, CollidableID, CollidableOrder, CollidableWithId,
        collidable::{
            contact_id_from_collidable_ids_and_indices,
            plane::PlaneCollidable,
            sphere::{
                SphereCollidable, generate_sphere_plane_contact_manifold,
                generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    fph,
    material::ContactResponseParameters,
};
use nalgebra::{Isometry3, UnitVector3, Vector3};

pub type CollisionWorld = collision::CollisionWorld<Collidable>;

#[derive(Clone, Debug)]
pub enum Collidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    VoxelObject(VoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub enum LocalCollidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    VoxelObject {
        object_id: VoxelObjectID,
        response_params: ContactResponseParameters,
    },
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidable {
    object_id: VoxelObjectID,
    response_params: ContactResponseParameters,
    transform_to_object_space: Isometry3<fph>,
}

impl collision::Collidable for Collidable {
    type Local = LocalCollidable;
    type Context = VoxelObjectManager;

    fn from_descriptor(
        descriptor: &CollidableDescriptor<Self>,
        transform_to_world_space: &Isometry3<fph>,
    ) -> Self {
        match descriptor.local_collidable() {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(transform_to_world_space)),
            Self::Local::VoxelObject {
                object_id,
                response_params,
            } => Self::VoxelObject(VoxelObjectCollidable::new(
                *object_id,
                *response_params,
                *transform_to_world_space,
            )),
        }
    }

    fn generate_contact_manifold(
        voxel_object_manager: &VoxelObjectManager,
        collidable_a: &CollidableWithId<Self>,
        collidable_b: &CollidableWithId<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder {
        use Collidable::{Plane, Sphere, VoxelObject};

        match (collidable_a.collidable(), collidable_b.collidable()) {
            (VoxelObject(voxel_object_a), VoxelObject(voxel_object_b)) => {
                generate_voxel_object_voxel_object_contact_manifold(
                    voxel_object_manager,
                    voxel_object_a,
                    voxel_object_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), VoxelObject(voxel_object)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (VoxelObject(voxel_object), Sphere(sphere)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (VoxelObject(voxel_object), Plane(plane)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), VoxelObject(voxel_object)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
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

impl VoxelObjectCollidable {
    pub fn new(
        object_id: VoxelObjectID,
        response_params: ContactResponseParameters,
        transform_to_world_space: Isometry3<fph>,
    ) -> Self {
        Self {
            object_id,
            response_params,
            transform_to_object_space: transform_to_world_space.inverse(),
        }
    }

    pub fn object_id(&self) -> VoxelObjectID {
        self.object_id
    }

    pub fn transform_to_object_space(&self) -> &Isometry3<fph> {
        &self.transform_to_object_space
    }
}

fn generate_voxel_object_voxel_object_contact_manifold(
    _voxel_object_manager: &VoxelObjectManager,
    _voxel_object_a: &VoxelObjectCollidable,
    _voxel_object_b: &VoxelObjectCollidable,
    _voxel_object_a_collidable_id: CollidableID,
    _voxel_object_b_collidable_id: CollidableID,
    _contact_manifold: &mut ContactManifold,
) {
}

fn generate_sphere_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    sphere: &SphereCollidable,
    voxel_object: &VoxelObjectCollidable,
    sphere_collidable_id: CollidableID,
    voxel_object_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        object_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let response_params =
        ContactResponseParameters::combined(response_params, sphere.response_params());

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(*object_id) else {
        return;
    };
    let voxel_object = voxel_object.object();

    let voxel_radius = 0.5 * voxel_object.voxel_extent();

    let sphere = sphere.sphere();
    let sphere_in_object_space = sphere.translated_and_rotated(transform_to_object_space);

    let max_squared_center_distance =
        sphere.radius_squared() + voxel_radius.powi(2) + 2.0 * sphere.radius() * voxel_radius;
    let radius_sum = sphere.radius() + voxel_radius;

    voxel_object.for_each_surface_voxel_maybe_intersecting_sphere(
        &sphere_in_object_space,
        &mut |[i, j, k], _, _| {
            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);

            let center_displacement = sphere.center() - voxel_center;
            let squared_center_distance = center_displacement.norm_squared();

            if squared_center_distance > max_squared_center_distance {
                return;
            }

            let center_distance = squared_center_distance.sqrt();

            let surface_normal = if center_distance > 1e-8 {
                UnitVector3::new_unchecked(center_displacement.unscale(center_distance))
            } else {
                Vector3::z_axis()
            };

            let position = voxel_center + surface_normal.scale(voxel_radius);

            let penetration_depth = fph::max(0.0, radius_sum - center_distance);

            let id = contact_id_from_collidable_ids_and_indices(
                sphere_collidable_id,
                voxel_object_collidable_id,
                [i, j, k],
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry: ContactGeometry {
                        position,
                        surface_normal,
                        penetration_depth,
                    },
                    response_params,
                },
            });
        },
    );
}

fn generate_voxel_object_plane_contact_manifold(
    _voxel_object_manager: &VoxelObjectManager,
    _voxel_object: &VoxelObjectCollidable,
    _plane: &PlaneCollidable,
    _voxel_object_collidable_id: CollidableID,
    _plane_collidable_id: CollidableID,
    _contact_manifold: &mut ContactManifold,
) {
}
