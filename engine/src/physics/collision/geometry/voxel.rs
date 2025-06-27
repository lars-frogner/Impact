//! Implementation of [`CollidableGeometry`](collision::CollidableGeometry) that
//! includes voxel geometry.

use crate::{
    physics::{
        collision::{
            self, Collidable, CollidableID, CollidableOrder,
            geometry::{
                plane::PlaneCollidableGeometry,
                sphere::{
                    SphereCollidableGeometry, generate_sphere_plane_contact_manifold,
                    generate_sphere_sphere_contact_manifold,
                },
            },
        },
        constraint::contact::{Contact, ContactGeometry, ContactID, ContactManifold},
        fph,
    },
    voxel::{VoxelObjectID, VoxelObjectManager},
};
use impact_geometry::{Plane, Sphere};
use nalgebra::{Similarity3, UnitVector3, Vector3};

pub type CollisionWorld = collision::CollisionWorld<CollidableGeometry>;

#[derive(Clone, Debug)]
pub enum CollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
    VoxelObject(VoxelObjectCollidableGeometry),
}

#[derive(Clone, Debug)]
pub enum LocalVoxelCollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
    VoxelObject(VoxelObjectID),
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidableGeometry {
    object_id: VoxelObjectID,
    transform_to_object_space: Similarity3<fph>,
}

impl CollidableGeometry {
    pub fn local_sphere(sphere: Sphere<fph>) -> LocalVoxelCollidableGeometry {
        LocalVoxelCollidableGeometry::Sphere(SphereCollidableGeometry::new(sphere))
    }

    pub fn local_plane(plane: Plane<fph>) -> LocalVoxelCollidableGeometry {
        LocalVoxelCollidableGeometry::Plane(PlaneCollidableGeometry::new(plane))
    }

    pub fn local_voxel_object(object_id: VoxelObjectID) -> LocalVoxelCollidableGeometry {
        LocalVoxelCollidableGeometry::VoxelObject(object_id)
    }
}

impl collision::CollidableGeometry for CollidableGeometry {
    type Local = LocalVoxelCollidableGeometry;
    type Context = VoxelObjectManager;

    fn from_local(geometry: &Self::Local, transform_to_world_space: Similarity3<fph>) -> Self {
        match geometry {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(&transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(&transform_to_world_space)),
            Self::Local::VoxelObject(object_id) => Self::VoxelObject(
                VoxelObjectCollidableGeometry::new(*object_id, transform_to_world_space),
            ),
        }
    }

    fn generate_contact_manifold(
        voxel_object_manager: &VoxelObjectManager,
        collidable_a: &Collidable<Self>,
        collidable_b: &Collidable<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder {
        use CollidableGeometry::{Plane, Sphere, VoxelObject};

        match (&collidable_a.geometry, &collidable_b.geometry) {
            (VoxelObject(voxel_object_a), VoxelObject(voxel_object_b)) => {
                generate_voxel_object_voxel_object_contact_manifold(
                    voxel_object_manager,
                    voxel_object_a,
                    voxel_object_b,
                    collidable_a.id,
                    collidable_b.id,
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), VoxelObject(voxel_object)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_a.id,
                    collidable_b.id,
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (VoxelObject(voxel_object), Sphere(sphere)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_b.id,
                    collidable_a.id,
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (VoxelObject(voxel_object), Plane(plane)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
                    plane,
                    collidable_a.id,
                    collidable_b.id,
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), VoxelObject(voxel_object)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
                    plane,
                    collidable_b.id,
                    collidable_a.id,
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
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

impl VoxelObjectCollidableGeometry {
    fn new(object_id: VoxelObjectID, transform_to_world_space: Similarity3<fph>) -> Self {
        Self {
            object_id,
            transform_to_object_space: transform_to_world_space.inverse(),
        }
    }

    pub fn object_id(&self) -> VoxelObjectID {
        self.object_id
    }

    pub fn transform_to_object_space(&self) -> &Similarity3<fph> {
        &self.transform_to_object_space
    }
}

fn generate_voxel_object_voxel_object_contact_manifold(
    _voxel_object_manager: &VoxelObjectManager,
    _voxel_object_a: &VoxelObjectCollidableGeometry,
    _voxel_object_b: &VoxelObjectCollidableGeometry,
    _voxel_object_a_collidable_id: CollidableID,
    _voxel_object_b_collidable_id: CollidableID,
    _contact_manifold: &mut ContactManifold,
) {
}

fn generate_sphere_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    sphere: &SphereCollidableGeometry,
    voxel_object: &VoxelObjectCollidableGeometry,
    sphere_collidable_id: CollidableID,
    voxel_object_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidableGeometry {
        object_id,
        transform_to_object_space,
    } = voxel_object;

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(*object_id) else {
        return;
    };
    let voxel_object = voxel_object.object();

    let voxel_radius = 0.5 * voxel_object.voxel_extent();

    let sphere = sphere.sphere();
    let sphere_in_object_space = sphere.transformed(transform_to_object_space);

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

            contact_manifold.add_contact(Contact {
                id,
                geometry: ContactGeometry {
                    position,
                    surface_normal,
                    penetration_depth,
                },
            });
        },
    );
}

fn generate_voxel_object_plane_contact_manifold(
    _voxel_object_manager: &VoxelObjectManager,
    _voxel_object: &VoxelObjectCollidableGeometry,
    _plane: &PlaneCollidableGeometry,
    _voxel_object_collidable_id: CollidableID,
    _plane_collidable_id: CollidableID,
    _contact_manifold: &mut ContactManifold,
) {
}

fn contact_id_from_collidable_ids_and_indices(
    a: CollidableID,
    b: CollidableID,
    indices: [usize; 3],
) -> ContactID {
    ContactID::from_two_u32_and_three_indices(a.0, b.0, indices)
}
