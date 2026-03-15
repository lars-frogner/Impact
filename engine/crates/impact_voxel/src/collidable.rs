//! Implementation of [`Collidable`](collision::Collidable) that includes voxel
//! geometry.

pub mod setup;

#[cfg(feature = "ecs")]
pub mod systems;

use crate::{
    Voxel, VoxelObjectID, VoxelObjectManager, VoxelSignedDistance, VoxelSurfacePlacement,
    chunks::{CHUNK_SIZE, ChunkedVoxelObject, sdf},
};
use impact_geometry::{Capsule, Plane, Sphere};
use impact_id::EntityID;
use impact_math::{
    transform::{Isometry3, Isometry3C},
    vector::{UnitVector3, Vector3, Vector3C},
};
use impact_physics::{
    collision::{
        self, CollidableDescriptor, CollidableID, CollidableOrder, CollidableWithId,
        collidable::{
            capsule::{
                CapsuleCollidable, determine_capsule_sphere_contact_geometry,
                generate_capsule_capsule_contact_manifold, generate_capsule_plane_contact_manifold,
                generate_capsule_sphere_contact_manifold,
            },
            contact_id_from_collidable_ids_and_indices,
            plane::PlaneCollidable,
            sphere::{
                SphereCollidable, determine_sphere_plane_contact_geometry,
                determine_sphere_sphere_contact_geometry, generate_sphere_plane_contact_manifold,
                generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    material::ContactResponseParameters,
};

pub type CollisionWorld = collision::CollisionWorld<Collidable>;

#[derive(Clone, Debug)]
pub enum Collidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
    VoxelObject(VoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub enum LocalCollidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
    VoxelObject(LocalVoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub struct LocalVoxelObjectCollidable {
    entity_id: EntityID,
    response_params: ContactResponseParameters,
    origin_offset: Vector3C,
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidable {
    entity_id: EntityID,
    response_params: ContactResponseParameters,
    transform_to_object_space: Isometry3C,
}

impl collision::Collidable for Collidable {
    type Local = LocalCollidable;
    type Context = VoxelObjectManager;

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
            Self::Local::VoxelObject(voxel_object) => {
                Self::VoxelObject(VoxelObjectCollidable::new(
                    voxel_object.entity_id,
                    voxel_object.response_params,
                    voxel_object.origin_offset.aligned(),
                    transform_to_world_space,
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
        use Collidable::{Capsule, Plane, Sphere, VoxelObject};

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
            (Capsule(capsule), VoxelObject(voxel_object)) => {
                generate_capsule_voxel_object_contact_manifold(
                    voxel_object_manager,
                    capsule,
                    voxel_object,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (VoxelObject(voxel_object), Capsule(capsule)) => {
                generate_capsule_voxel_object_contact_manifold(
                    voxel_object_manager,
                    capsule,
                    voxel_object,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
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

impl LocalVoxelObjectCollidable {
    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

impl VoxelObjectCollidable {
    pub fn new(
        entity_id: EntityID,
        response_params: ContactResponseParameters,
        origin_offset: Vector3,
        transform_to_world_space: &Isometry3,
    ) -> Self {
        let transform_from_object_to_world_space =
            transform_to_world_space.applied_to_translation(&(-origin_offset));

        let transform_to_object_space = transform_from_object_to_world_space.inverted();

        Self {
            entity_id,
            response_params,
            transform_to_object_space: transform_to_object_space.compact(),
        }
    }

    pub fn entity_id(&self) -> EntityID {
        self.entity_id
    }

    pub fn transform_to_object_space(&self) -> &Isometry3C {
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
        entity_id: entity_a_id,
        response_params: response_params_a,
        transform_to_object_space: transform_from_world_to_a,
    } = voxel_object_a;

    let VoxelObjectCollidable {
        entity_id: entity_b_id,
        response_params: response_params_b,
        transform_to_object_space: transform_from_world_to_b,
    } = voxel_object_b;

    let object_a_id = VoxelObjectID::from_entity_id(*entity_a_id);
    let Some(object_a) = voxel_object_manager.get_voxel_object(object_a_id) else {
        return;
    };

    let object_b_id = VoxelObjectID::from_entity_id(*entity_b_id);
    let Some(object_b) = voxel_object_manager.get_voxel_object(object_b_id) else {
        return;
    };

    let transform_from_world_to_a = transform_from_world_to_a.aligned();
    let transform_from_world_to_b = transform_from_world_to_b.aligned();

    let response_params = ContactResponseParameters::combined(response_params_a, response_params_b);

    for_each_mutual_voxel_object_contact(
        object_a.object(),
        object_b.object(),
        &transform_from_world_to_a,
        &transform_from_world_to_b,
        &mut |indices_for_id, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                voxel_object_a_collidable_id,
                voxel_object_b_collidable_id,
                indices_for_id,
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

pub fn for_each_mutual_voxel_object_contact<'a>(
    voxel_object_a: &'a ChunkedVoxelObject,
    voxel_object_b: &'a ChunkedVoxelObject,
    transform_from_world_to_a: &'a Isometry3,
    transform_from_world_to_b: &'a Isometry3,
    f: &mut impl FnMut([usize; 7], ContactGeometry),
) {
    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverted();

    let Some((intersection_voxel_ranges_in_a, intersection_voxel_ranges_in_b)) =
        ChunkedVoxelObject::determine_voxel_ranges_encompassing_intersection(
            voxel_object_a,
            voxel_object_b,
            &transform_from_b_to_a,
        )
    else {
        return;
    };

    let voxel_object_a_intersection_size = intersection_voxel_ranges_in_a
        .iter()
        .map(|r| r.len())
        .product::<usize>();
    let voxel_object_b_intersection_size = intersection_voxel_ranges_in_b
        .iter()
        .map(|r| r.len())
        .product::<usize>();

    if voxel_object_a_intersection_size <= voxel_object_b_intersection_size {
        let grid_dimensions_for_b = voxel_object_b
            .chunk_counts()
            .map(|count| count * CHUNK_SIZE);

        voxel_object_a.for_each_surface_voxel_in_voxel_ranges(
            intersection_voxel_ranges_in_a,
            &mut |[i_a, j_a, k_a], voxel_a, _| {
                let voxel_a_center_in_a =
                    voxel_object_a.voxel_center_position_from_object_voxel_indices(i_a, j_a, k_a);

                let voxel_a_center =
                    transform_from_world_to_a.inverse_transform_point(&voxel_a_center_in_a);

                let voxel_a_radius = compute_voxel_radius(voxel_a, voxel_object_a.voxel_extent());

                let voxel_a_sphere = Sphere::new(voxel_a_center, voxel_a_radius);

                let Some(([sdf_i, sdf_j, sdf_k], signed_distance, surface_normal)) =
                    determine_sdf_value_and_normal_at_sphere_center_if_intersecting(
                        voxel_object_b,
                        &grid_dimensions_for_b,
                        transform_from_world_to_b,
                        &voxel_a_sphere,
                    )
                else {
                    return;
                };

                let position = voxel_a_center - signed_distance * surface_normal;

                let penetration_depth = f32::max(0.0, voxel_a_radius - signed_distance);

                f(
                    [0, i_a, j_a, k_a, sdf_i, sdf_j, sdf_k],
                    ContactGeometry {
                        position,
                        surface_normal,
                        penetration_depth,
                    },
                );
            },
        );
    } else {
        let grid_dimensions_for_a = voxel_object_a
            .chunk_counts()
            .map(|count| count * CHUNK_SIZE);

        voxel_object_b.for_each_surface_voxel_in_voxel_ranges(
            intersection_voxel_ranges_in_b,
            &mut |[i_b, j_b, k_b], voxel_b, _| {
                let voxel_b_center_in_b =
                    voxel_object_b.voxel_center_position_from_object_voxel_indices(i_b, j_b, k_b);

                let voxel_b_center =
                    transform_from_world_to_b.inverse_transform_point(&voxel_b_center_in_b);

                let voxel_b_radius = compute_voxel_radius(voxel_b, voxel_object_b.voxel_extent());

                let voxel_b_sphere = Sphere::new(voxel_b_center, voxel_b_radius);

                let Some(([sdf_i, sdf_j, sdf_k], signed_distance, sdf_surface_normal)) =
                    determine_sdf_value_and_normal_at_sphere_center_if_intersecting(
                        voxel_object_a,
                        &grid_dimensions_for_a,
                        transform_from_world_to_a,
                        &voxel_b_sphere,
                    )
                else {
                    return;
                };

                let surface_normal = -sdf_surface_normal;

                let position = voxel_b_center + voxel_b_radius * surface_normal;

                let penetration_depth = f32::max(0.0, voxel_b_radius - signed_distance);

                f(
                    [1, i_b, j_b, k_b, sdf_i, sdf_j, sdf_k],
                    ContactGeometry {
                        position,
                        surface_normal,
                        penetration_depth,
                    },
                );
            },
        );
    }
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
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, sphere.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let sphere = sphere.sphere().aligned();

    for_each_sphere_voxel_object_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &sphere,
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
    transform_to_object_space: &Isometry3,
    sphere: &Sphere,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let sphere_in_object_space = sphere.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_sphere(
        &sphere_in_object_space,
        &mut |[i, j, k], voxel, _| {
            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            let voxel_sphere = Sphere::new(voxel_center, voxel_radius);

            let Some(contact_geometry) =
                determine_sphere_sphere_contact_geometry(sphere, &voxel_sphere)
            else {
                return;
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
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, plane.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let plane = plane.plane().aligned();

    for_each_voxel_object_plane_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &plane,
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
    transform_to_object_space: &Isometry3,
    plane: &Plane,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let plane_in_object_space = plane.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
        &plane_in_object_space,
        &mut |[i, j, k], voxel, placement| {
            // In the case of a plane, we only need contacts for the corner
            // voxels
            if placement != VoxelSurfacePlacement::Corner {
                return;
            }

            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            if let Some(contact_geometry) = determine_sphere_plane_contact_geometry(
                &Sphere::new(voxel_center, voxel_radius),
                plane,
            ) {
                f([i, j, k], contact_geometry);
            }
        },
    );
}

fn generate_capsule_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    capsule: &CapsuleCollidable,
    voxel_object: &VoxelObjectCollidable,
    capsule_collidable_id: CollidableID,
    voxel_object_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, capsule.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let capsule = capsule.capsule().aligned();

    for_each_capsule_voxel_object_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &capsule,
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                capsule_collidable_id,
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

pub fn for_each_capsule_voxel_object_contact(
    voxel_object: &ChunkedVoxelObject,
    transform_to_object_space: &Isometry3,
    capsule: &Capsule,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let capsule_in_object_space = capsule.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_capsule(
        &capsule_in_object_space,
        &mut |[i, j, k], voxel, _| {
            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            let voxel_sphere = Sphere::new(voxel_center, voxel_radius);

            let Some(contact_geometry) =
                determine_capsule_sphere_contact_geometry(capsule, &voxel_sphere)
            else {
                return;
            };

            f([i, j, k], contact_geometry);
        },
    );
}

#[inline]
fn determine_sdf_value_and_normal_at_sphere_center_if_intersecting(
    sdf_object: &ChunkedVoxelObject,
    sdf_grid_dimensions: &[usize; 3],
    transform_from_world_to_sdf: &Isometry3,
    sphere_in_world_space: &Sphere,
) -> Option<([usize; 3], f32, UnitVector3)> {
    let sphere_in_sdf_space = sphere_in_world_space
        .iso_transformed(transform_from_world_to_sdf)
        .scaled(sdf_object.inverse_voxel_extent());

    let shifted_sphere_offset_in_sdf = sphere_in_sdf_space.center() - Vector3::same(0.5);

    let lower_indices = shifted_sphere_offset_in_sdf.as_vector().component_floor();
    let fractional_offset = shifted_sphere_offset_in_sdf.as_vector() - lower_indices;

    if lower_indices.has_negative_component() {
        // Avoid sampling outside the lower bounds of the SDF grid
        return None;
    }

    let [sdf_i, sdf_j, sdf_k] = <[f32; 3]>::from(lower_indices).map(|idx| idx as usize);

    if sdf_i + 1 >= sdf_grid_dimensions[0]
        || sdf_j + 1 >= sdf_grid_dimensions[1]
        || sdf_k + 1 >= sdf_grid_dimensions[2]
    {
        // Avoid sampling outside the upper bounds of the SDF grid
        return None;
    }

    let sample_dist = |i, j, k| sdf_object.voxel(i, j, k).signed_distance().to_f32();

    let dists = [
        sample_dist(sdf_i, sdf_j, sdf_k),
        sample_dist(sdf_i, sdf_j, sdf_k + 1),
        sample_dist(sdf_i, sdf_j + 1, sdf_k),
        sample_dist(sdf_i, sdf_j + 1, sdf_k + 1),
        sample_dist(sdf_i + 1, sdf_j, sdf_k),
        sample_dist(sdf_i + 1, sdf_j, sdf_k + 1),
        sample_dist(sdf_i + 1, sdf_j + 1, sdf_k),
        sample_dist(sdf_i + 1, sdf_j + 1, sdf_k + 1),
    ];

    let signed_distance = sdf::evaluate_sdf_from_corner_samples(&dists, &fractional_offset);

    if signed_distance > sphere_in_sdf_space.radius() {
        // The sphere is fully on the outside of the surface
        return None;
    }

    if (signed_distance - VoxelSignedDistance::MIN_F32).abs() < 1e-3 {
        // The sphere is deep enough in the SDF interior that the local signed
        // distance is capped, so the gradient will be zero. We don't have
        // enough information to proceed.
        return None;
    }

    let sdf_gradient = sdf::compute_sdf_gradient_from_corner_samples(&dists, &fractional_offset);

    let normal_vector = UnitVector3::normalized_from_if_above(sdf_gradient, 1e-8)?;

    if signed_distance.is_sign_positive() {
        // If there sphere center is outside the surface, we should verify that
        // the sphere indeed crosses the surface. If it doesn't, the signed
        // distance we sampled was stale and we should reject the collision.

        let deepest_sphere_point =
            sphere_in_sdf_space.center() - sphere_in_sdf_space.radius() * normal_vector;

        let sphere_crosses_surface = sdf_object
            .get_voxel_at_grid_coords_if_occupied(deepest_sphere_point.as_vector())
            .is_some();

        if !sphere_crosses_surface {
            return None;
        }
    }

    let world_signed_distance = signed_distance * sdf_object.voxel_extent();

    let world_normal_vector = transform_from_world_to_sdf
        .rotation()
        .inverse()
        .rotate_unit_vector(&normal_vector);

    Some((
        [sdf_i, sdf_j, sdf_k],
        world_signed_distance,
        world_normal_vector,
    ))
}

#[inline]
fn compute_voxel_radius(voxel: &Voxel, voxel_extent: f32) -> f32 {
    -voxel.signed_distance().to_f32() * voxel_extent
}
