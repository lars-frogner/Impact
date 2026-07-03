//! Implementation of [`Collidable`](collision::Collidable) that includes voxel
//! geometry.

pub mod setup;

#[cfg(feature = "ecs")]
pub mod systems;

use crate::{
    Voxel, VoxelObjectID, VoxelObjectManager, VoxelSignedDistance, VoxelSurfacePlacement,
    chunks::{
        self, CHUNK_SIZE, ChunkedVoxelObject, VoxelChunk, chunk_range_encompassing_voxel_range, sdf,
    },
    mesh::MeshedChunkedVoxelObject,
};
use impact_geometry::{Capsule, Plane, Sphere};
use impact_id::EntityID;
use impact_math::{
    consts::f32::SQRT_3,
    point::Point3C,
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
        object_a,
        object_b,
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
    voxel_object_a: &'a MeshedChunkedVoxelObject,
    voxel_object_b: &'a MeshedChunkedVoxelObject,
    transform_from_world_to_a: &'a Isometry3,
    transform_from_world_to_b: &'a Isometry3,
    f: &mut impl FnMut([usize; 4], ContactGeometry),
) {
    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverted();

    let Some((intersection_voxel_ranges_in_a, intersection_voxel_ranges_in_b)) =
        ChunkedVoxelObject::determine_voxel_ranges_encompassing_intersection(
            voxel_object_a.object(),
            voxel_object_b.object(),
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
            .object()
            .chunk_counts()
            .map(|count| count * CHUNK_SIZE);

        let intersection_chunk_ranges_in_a = intersection_voxel_ranges_in_a
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        for chunk_i in intersection_chunk_ranges_in_a[0].clone() {
            for chunk_j in intersection_chunk_ranges_in_a[1].clone() {
                for chunk_k in intersection_chunk_ranges_in_a[2].clone() {
                    let Some(vertex_positions) = voxel_object_a
                        .mesh()
                        .vertex_positions_for_chunk_at_indices([chunk_i, chunk_j, chunk_k])
                    else {
                        continue;
                    };
                    for vertex_position in vertex_positions {
                        let vertex_position = Point3C::from(vertex_position.0);

                        let point_in_a = vertex_position.aligned();

                        let point = transform_from_world_to_a.inverse_transform_point(&point_in_a);

                        let norm_point_in_b = transform_from_world_to_b.transform_point(&point)
                            * voxel_object_b.object().inverse_voxel_extent();

                        let Some((signed_distance_in_b, normal_vector_in_b)) =
                            determine_sdf_value_and_normal_at_point_if_intersecting(
                                voxel_object_b.object(),
                                &grid_dimensions_for_b,
                                &norm_point_in_b.compact(),
                            )
                        else {
                            continue;
                        };

                        let surface_normal = transform_from_world_to_b
                            .rotation()
                            .inverse()
                            .rotate_unit_vector(&normal_vector_in_b);

                        let penetration_depth =
                            -signed_distance_in_b * voxel_object_b.object().voxel_extent();

                        let norm_point_in_a =
                            point_in_a * voxel_object_a.object().inverse_voxel_extent();

                        let [i_a, j_a, k_a] =
                            <[f32; 3]>::from(norm_point_in_a.as_vector().component_floor())
                                .map(|idx| idx as usize);

                        f(
                            [0, i_a, j_a, k_a],
                            ContactGeometry {
                                position: point,
                                surface_normal,
                                penetration_depth,
                            },
                        );
                    }
                }
            }
        }
    } else {
        let grid_dimensions_for_a = voxel_object_a
            .object()
            .chunk_counts()
            .map(|count| count * CHUNK_SIZE);

        let intersection_chunk_ranges_in_b = intersection_voxel_ranges_in_b
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        for chunk_i in intersection_chunk_ranges_in_b[0].clone() {
            for chunk_j in intersection_chunk_ranges_in_b[1].clone() {
                for chunk_k in intersection_chunk_ranges_in_b[2].clone() {
                    let Some(vertex_positions) = voxel_object_b
                        .mesh()
                        .vertex_positions_for_chunk_at_indices([chunk_i, chunk_j, chunk_k])
                    else {
                        continue;
                    };
                    for vertex_position in vertex_positions {
                        let vertex_position = Point3C::from(vertex_position.0);

                        let point_in_b = vertex_position.aligned();

                        let point = transform_from_world_to_b.inverse_transform_point(&point_in_b);

                        let norm_point_in_a = transform_from_world_to_a.transform_point(&point)
                            * voxel_object_a.object().inverse_voxel_extent();

                        let Some((signed_distance_in_a, normal_vector_in_a)) =
                            determine_sdf_value_and_normal_at_point_if_intersecting(
                                voxel_object_a.object(),
                                &grid_dimensions_for_a,
                                &norm_point_in_a.compact(),
                            )
                        else {
                            continue;
                        };

                        let normal_vector = transform_from_world_to_a
                            .rotation()
                            .inverse()
                            .rotate_unit_vector(&normal_vector_in_a);

                        let surface_normal = -normal_vector;

                        let penetration_depth =
                            -signed_distance_in_a * voxel_object_a.object().voxel_extent();

                        let norm_point_in_b =
                            point_in_b * voxel_object_b.object().inverse_voxel_extent();

                        let [i_b, j_b, k_b] =
                            <[f32; 3]>::from(norm_point_in_b.as_vector().component_floor())
                                .map(|idx| idx as usize);

                        f(
                            [0, i_b, j_b, k_b],
                            ContactGeometry {
                                position: point,
                                surface_normal,
                                penetration_depth,
                            },
                        );
                    }
                }
            }
        }
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
fn determine_sdf_value_and_normal_at_point_if_intersecting(
    object: &ChunkedVoxelObject,
    grid_dimensions: &[usize; 3],
    norm_point: &Point3C,
) -> Option<(f32, UnitVector3)> {
    const HALF_VOXEL_DIAGONAL: f32 = 0.5 * SQRT_3;

    let shifted_point = norm_point - Vector3C::same(0.5);
    let lower_indices_f32 = shifted_point.as_vector().component_floor();
    let fractional_offset = shifted_point.as_vector() - lower_indices_f32;

    if lower_indices_f32.has_negative_component() {
        // Avoid sampling outside the lower bounds of the SDF grid
        return None;
    }

    let [li, lj, lk] = <[f32; 3]>::from(lower_indices_f32).map(|idx| idx as usize);

    if li + 1 >= grid_dimensions[0] || lj + 1 >= grid_dimensions[1] || lk + 1 >= grid_dimensions[2]
    {
        // Avoid sampling outside the upper bounds of the SDF grid
        return None;
    }

    let containing_cell_corner = norm_point.as_vector().component_floor();
    let [ci, cj, ck] = <[f32; 3]>::from(containing_cell_corner).map(|idx| idx as usize);

    let [chunk_i, chunk_j, chunk_k] = chunks::chunk_indices_from_object_voxel_indices(ci, cj, ck);
    let chunk_idx = object.linear_chunk_idx(&[chunk_i, chunk_j, chunk_k]);
    let chunk = object.chunk_at_idx_maybe_unchecked(chunk_idx);

    let VoxelChunk::NonUniform(chunk) = chunk else {
        return None;
    };

    let chunk_start_voxel_idx = chunk.start_voxel_idx();

    let containing_voxel_idx = chunk_start_voxel_idx
        + chunks::linear_voxel_idx_within_chunk_from_object_voxel_indices(ci, cj, ck);

    let containing_signed_dist = object
        .voxel_at_idx_maybe_unchecked(containing_voxel_idx)
        .signed_distance()
        .to_f32();

    if containing_signed_dist > HALF_VOXEL_DIAGONAL {
        return None;
    }

    // Lower indices within chunk
    let [cli, clj, clk] = chunks::voxel_indices_within_chunk_from_object_voxel_indices(li, lj, lk);

    let all_in_same_chunk = cli != CHUNK_SIZE - 1 && clj != CHUNK_SIZE - 1 && clk != CHUNK_SIZE - 1;

    let signed_distances = if all_in_same_chunk {
        let sample_dist = |i, j, k| {
            let voxel_idx =
                chunk_start_voxel_idx + chunks::linear_voxel_idx_within_chunk(&[i, j, k]);
            let voxel = object.voxel_at_idx_maybe_unchecked(voxel_idx);
            voxel.signed_distance().to_f32()
        };

        [
            sample_dist(cli, clj, clk),
            sample_dist(cli, clj, clk + 1),
            sample_dist(cli, clj + 1, clk),
            sample_dist(cli, clj + 1, clk + 1),
            sample_dist(cli + 1, clj, clk),
            sample_dist(cli + 1, clj, clk + 1),
            sample_dist(cli + 1, clj + 1, clk),
            sample_dist(cli + 1, clj + 1, clk + 1),
        ]
    } else {
        let sample_dist = |i, j, k| {
            object
                .voxel_maybe_unchecked(i, j, k)
                .signed_distance()
                .to_f32()
        };
        [
            sample_dist(li, lj, lk),
            sample_dist(li, lj, lk + 1),
            sample_dist(li, lj + 1, lk),
            sample_dist(li, lj + 1, lk + 1),
            sample_dist(li + 1, lj, lk),
            sample_dist(li + 1, lj, lk + 1),
            sample_dist(li + 1, lj + 1, lk),
            sample_dist(li + 1, lj + 1, lk + 1),
        ]
    };

    let signed_distance =
        sdf::evaluate_sdf_from_corner_samples(&signed_distances, &fractional_offset);

    if signed_distance > 0.0 {
        // The point is fully on the outside of the surface
        return None;
    }

    if (signed_distance - VoxelSignedDistance::MIN_F32).abs() < 1e-3 {
        // The point is deep enough in the SDF interior that the local signed
        // distance is capped, so the gradient will be zero. We don't have
        // enough information to proceed.
        return None;
    }

    let sdf_gradient = sdf::compute_sdf_gradient_from_corner_samples(
        &signed_distances,
        &fractional_offset.aligned(),
    );

    let normal_vector = UnitVector3::normalized_from_if_above(sdf_gradient, 1e-8)?;

    Some((signed_distance, normal_vector))
}

#[inline]
fn compute_voxel_radius(voxel: &Voxel, voxel_extent: f32) -> f32 {
    -voxel.signed_distance().to_f32() * voxel_extent
}
