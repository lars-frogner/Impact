//! Management of collidables for entities.

use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_physics::{
    collision::{
        self, CollidableID,
        setup::{PlanarCollidable, SphericalCollidable},
    },
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID},
};
use impact_voxel::{
    VoxelObjectID,
    collidable::{CollisionWorld, LocalCollidable, setup::VoxelCollidable},
};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a collidable, and if so, creates the corresponding collidable
/// and adds a [`CollidableID`] to the entity.
pub fn setup_collidable_for_new_entity(
    collision_world: &RwLock<CollisionWorld>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |spherical_collidable: &SphericalCollidable,
         rigid_body_id: &DynamicRigidBodyID|
         -> CollidableID {
            collision::setup::setup_spherical_collidable(
                &mut collision_world,
                (*rigid_body_id).into(),
                spherical_collidable,
                LocalCollidable::Sphere,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |spherical_collidable: &SphericalCollidable,
         rigid_body_id: &KinematicRigidBodyID|
         -> CollidableID {
            collision::setup::setup_spherical_collidable(
                &mut collision_world,
                (*rigid_body_id).into(),
                spherical_collidable,
                LocalCollidable::Sphere,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |planar_collidable: &PlanarCollidable,
         rigid_body_id: &DynamicRigidBodyID|
         -> CollidableID {
            collision::setup::setup_planar_collidable(
                &mut collision_world,
                (*rigid_body_id).into(),
                planar_collidable,
                LocalCollidable::Plane,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |planar_collidable: &PlanarCollidable,
         rigid_body_id: &KinematicRigidBodyID|
         -> CollidableID {
            collision::setup::setup_planar_collidable(
                &mut collision_world,
                (*rigid_body_id).into(),
                planar_collidable,
                LocalCollidable::Plane,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |voxel_collidable: &VoxelCollidable,
         voxel_object_id: &VoxelObjectID,
         rigid_body_id: &DynamicRigidBodyID|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                *voxel_object_id,
                (*rigid_body_id).into(),
                voxel_collidable,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |voxel_collidable: &VoxelCollidable,
         voxel_object_id: &VoxelObjectID,
         rigid_body_id: &KinematicRigidBodyID|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                *voxel_object_id,
                (*rigid_body_id).into(),
                voxel_collidable,
            )
        }
    );
}
