//! Implementation of [`Collidable`](collision::Collidable) that includes voxel
//! geometry.

pub mod setup;

#[cfg(feature = "ecs")]
pub mod systems;

use crate::{VoxelObjectID, VoxelObjectManager, VoxelSurfacePlacement, chunks::ChunkedVoxelObject};
use impact_geometry::{Plane, Sphere};
use impact_physics::{
    collision::{
        self, CollidableDescriptor, CollidableID, CollidableOrder, CollidableWithId,
        collidable::{
            contact_id_from_collidable_ids_and_indices,
            plane::PlaneCollidable,
            sphere::{
                SphereCollidable, determine_sphere_plane_contact_geometry,
                generate_sphere_plane_contact_manifold, generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    material::ContactResponseParameters,
};
use nalgebra::{Isometry3, Translation3, UnitQuaternion, UnitVector3, Vector3};

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
    VoxelObject(LocalVoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub struct LocalVoxelObjectCollidable {
    object_id: VoxelObjectID,
    response_params: ContactResponseParameters,
    origin_offset: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidable {
    object_id: VoxelObjectID,
    response_params: ContactResponseParameters,
    transform_to_object_space: Isometry3<f32>,
}

impl collision::Collidable for Collidable {
    type Local = LocalCollidable;
    type Context = VoxelObjectManager;

    fn from_descriptor(
        descriptor: &CollidableDescriptor<Self>,
        transform_to_world_space: &Isometry3<f32>,
    ) -> Self {
        match descriptor.local_collidable() {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(transform_to_world_space)),
            Self::Local::VoxelObject(voxel_object) => {
                Self::VoxelObject(VoxelObjectCollidable::new(
                    voxel_object.object_id,
                    voxel_object.response_params,
                    voxel_object.origin_offset,
                    *transform_to_world_space,
                ))
            }
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
                generate_mutual_voxel_object_contact_manifold(
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

impl LocalVoxelObjectCollidable {
    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

impl VoxelObjectCollidable {
    pub fn new(
        object_id: VoxelObjectID,
        response_params: ContactResponseParameters,
        origin_offset: Vector3<f32>,
        transform_to_world_space: Isometry3<f32>,
    ) -> Self {
        let transform_from_object_to_world_space = transform_to_world_space
            * Isometry3::from_parts(
                Translation3::from(-origin_offset.cast()),
                UnitQuaternion::identity(),
            );
        Self {
            object_id,
            response_params,
            transform_to_object_space: transform_from_object_to_world_space.inverse(),
        }
    }

    pub fn object_id(&self) -> VoxelObjectID {
        self.object_id
    }

    pub fn transform_to_object_space(&self) -> &Isometry3<f32> {
        &self.transform_to_object_space
    }
}

fn generate_mutual_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object_a: &VoxelObjectCollidable,
    voxel_object_b: &VoxelObjectCollidable,
    voxel_object_a_collidable_id: CollidableID,
    voxel_object_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        object_id: object_a_id,
        response_params: response_params_a,
        transform_to_object_space: transform_from_world_to_a,
    } = voxel_object_a;

    let VoxelObjectCollidable {
        object_id: object_b_id,
        response_params: response_params_b,
        transform_to_object_space: transform_from_world_to_b,
    } = voxel_object_b;

    let Some(object_a) = voxel_object_manager.get_voxel_object(*object_a_id) else {
        return;
    };
    let Some(object_b) = voxel_object_manager.get_voxel_object(*object_b_id) else {
        return;
    };

    let response_params = ContactResponseParameters::combined(response_params_a, response_params_b);

    for_each_mutual_voxel_object_contact(
        object_a.object(),
        object_b.object(),
        transform_from_world_to_a,
        transform_from_world_to_b,
        &mut |[i_a, j_a, k_a], [i_b, j_b, k_b], geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                voxel_object_a_collidable_id,
                voxel_object_b_collidable_id,
                [i_a, j_a, k_a, i_b, j_b, k_b],
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_mutual_voxel_object_contact(
    voxel_object_a: &ChunkedVoxelObject,
    voxel_object_b: &ChunkedVoxelObject,
    transform_from_world_to_a: &Isometry3<f32>,
    transform_from_world_to_b: &Isometry3<f32>,
    f: &mut impl FnMut([usize; 3], [usize; 3], ContactGeometry),
) {
    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverse();

    let Some((intersection_voxel_ranges_in_a, intersection_voxel_ranges_in_b)) =
        ChunkedVoxelObject::determine_voxel_ranges_encompassing_intersection(
            voxel_object_a,
            voxel_object_b,
            &transform_from_b_to_a,
        )
    else {
        return;
    };

    let voxel_radius_a = 0.5 * voxel_object_a.voxel_extent();
    let voxel_radius_b = 0.5 * voxel_object_b.voxel_extent();

    let max_center_distance = voxel_radius_a + voxel_radius_b;
    let max_squared_center_distance = max_center_distance.powi(2);

    voxel_object_a.for_each_surface_voxel_in_voxel_ranges(
        intersection_voxel_ranges_in_a,
        &mut |[i_a, j_a, k_a], _, placement_a| {
            let voxel_a_center_in_object_space =
                voxel_object_a.voxel_center_position_from_object_voxel_indices(i_a, j_a, k_a);

            let voxel_a_center =
                transform_from_world_to_a.inverse_transform_point(&voxel_a_center_in_object_space);

            voxel_object_b.for_each_surface_voxel_in_voxel_ranges(
                intersection_voxel_ranges_in_b.clone(),
                &mut |[i_b, j_b, k_b], _, placement_b| {
                    if !matches!(
                        (placement_a, placement_b),
                        // Corner voxels in A can be in contact with any surface voxel in B
                        (
                            VoxelSurfacePlacement::Corner,
                            VoxelSurfacePlacement::Corner
                                | VoxelSurfacePlacement::Edge
                                | VoxelSurfacePlacement::Face,
                        )
                        // Edge voxels in A can be in contact with corner or edge voxels in B
                        | (
                            VoxelSurfacePlacement::Edge,
                            VoxelSurfacePlacement::Corner | VoxelSurfacePlacement::Edge,
                        )
                        // Face voxels in A can be in contact with corner voxels in B
                        | (VoxelSurfacePlacement::Face, VoxelSurfacePlacement::Corner)
                    ) {
                        return;
                    }

                    let voxel_b_center_in_object_space = voxel_object_b
                        .voxel_center_position_from_object_voxel_indices(i_b, j_b, k_b);

                    let voxel_b_center = transform_from_world_to_b
                        .inverse_transform_point(&voxel_b_center_in_object_space);

                    let center_displacement = voxel_a_center - voxel_b_center;
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

                    let position = voxel_b_center + surface_normal.scale(voxel_radius_b);

                    let penetration_depth = f32::max(0.0, max_center_distance - center_distance);

                    let contact_geometry = ContactGeometry {
                        position,
                        surface_normal,
                        penetration_depth,
                    };

                    f([i_a, j_a, k_a], [i_b, j_b, k_b], contact_geometry);
                },
            );
        },
    );
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

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(*object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, sphere.response_params());

    for_each_sphere_voxel_object_contact(
        voxel_object.object(),
        transform_to_object_space,
        sphere.sphere(),
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                sphere_collidable_id,
                voxel_object_collidable_id,
                indices,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_sphere_voxel_object_contact(
    voxel_object: &ChunkedVoxelObject,
    transform_to_object_space: &Isometry3<f32>,
    sphere: &Sphere<f32>,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let voxel_radius = 0.5 * voxel_object.voxel_extent();

    let sphere_in_object_space = sphere.translated_and_rotated(transform_to_object_space);

    let max_center_distance = sphere.radius() + voxel_radius;
    let max_squared_center_distance = max_center_distance.powi(2);

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

            let penetration_depth = f32::max(0.0, max_center_distance - center_distance);

            let contact_geometry = ContactGeometry {
                position,
                surface_normal,
                penetration_depth,
            };

            f([i, j, k], contact_geometry);
        },
    );
}

fn generate_voxel_object_plane_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object: &VoxelObjectCollidable,
    plane: &PlaneCollidable,
    voxel_object_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        object_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(*object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, plane.response_params());

    for_each_voxel_object_plane_contact(
        voxel_object.object(),
        transform_to_object_space,
        plane.plane(),
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                plane_collidable_id,
                voxel_object_collidable_id,
                indices,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_voxel_object_plane_contact(
    voxel_object: &ChunkedVoxelObject,
    transform_to_object_space: &Isometry3<f32>,
    plane: &Plane<f32>,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let voxel_radius = 0.5 * voxel_object.voxel_extent();

    let plane_in_object_space = plane.translated_and_rotated(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
        &plane_in_object_space,
        &mut |[i, j, k], _, placement| {
            // In the case of a plane, we only need contacts for the corner
            // voxels
            if placement != VoxelSurfacePlacement::Corner {
                return;
            }

            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);

            if let Some(contact_geometry) = determine_sphere_plane_contact_geometry(
                &Sphere::new(voxel_center, voxel_radius),
                plane,
            ) {
                f([i, j, k], contact_geometry);
            }
        },
    );
}
