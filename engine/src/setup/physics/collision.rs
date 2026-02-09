//! Setup of collidables for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ModelTransform;
use impact_id::EntityID;
use impact_physics::{
    collision::{
        self, CollidableID,
        setup::{PlanarCollidable, SphericalCollidable},
    },
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID},
};
use impact_voxel::{
    HasVoxelObject,
    collidable::{LocalCollidable, setup::VoxelCollidable},
};
use parking_lot::RwLock;

/// Checks if the given entities have a component representing a collidable, and
/// if so, creates the corresponding collidables and adds the [`CollidableID`]s
/// to the entity.
pub fn setup_collidables_for_new_entities(
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) {
    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
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
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
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
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
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
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
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
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_collidable: &VoxelCollidable,
         rigid_body_id: &DynamicRigidBodyID,
         model_transform: &ModelTransform|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                model_transform.offset,
                voxel_collidable,
            )
        },
        [HasVoxelObject]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_collidable: &VoxelCollidable,
         rigid_body_id: &KinematicRigidBodyID,
         model_transform: Option<&ModelTransform>|
         -> CollidableID {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                model_transform
                    .map(|model_transform| model_transform.offset)
                    .unwrap_or_default(),
                voxel_collidable,
            )
        },
        [HasVoxelObject]
    );
}

pub fn remove_collidable_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity: &EntityEntry<'_>,
) {
    if let Some(collidable_id) = entity.get_component::<CollidableID>() {
        let simulator = simulator.oread();
        let mut collision_world = simulator.collision_world().owrite();
        collision_world.remove_collidable(*collidable_id.access());
    }
}
