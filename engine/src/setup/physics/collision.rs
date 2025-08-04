//! Setup of collidables for new entities.

use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ModelTransform;
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
use parking_lot::RwLock;

/// Checks if the entities-to-be with the given components have a component
/// representing a collidable, and if so, creates the corresponding collidables
/// and adds the [`CollidableID`]s to the entity.
pub fn setup_collidables_for_new_entities(
    collision_world: &RwLock<CollisionWorld>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut collision_world = collision_world.write();
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
            let mut collision_world = collision_world.write();
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
            let mut collision_world = collision_world.write();
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
            let mut collision_world = collision_world.write();
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
            let mut collision_world = collision_world.write();
        },
        components,
        |voxel_collidable: &VoxelCollidable,
         voxel_object_id: &VoxelObjectID,
         rigid_body_id: &DynamicRigidBodyID,
         model_transform: &ModelTransform|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                *voxel_object_id,
                (*rigid_body_id).into(),
                model_transform.offset,
                voxel_collidable,
            )
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write();
        },
        components,
        |voxel_collidable: &VoxelCollidable,
         voxel_object_id: &VoxelObjectID,
         rigid_body_id: &KinematicRigidBodyID,
         model_transform: Option<&ModelTransform>|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                *voxel_object_id,
                (*rigid_body_id).into(),
                model_transform
                    .map(|model_transform| model_transform.offset)
                    .unwrap_or_default(),
                voxel_collidable,
            )
        }
    );
}
