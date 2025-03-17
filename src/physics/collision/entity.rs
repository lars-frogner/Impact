//! Management of collidables for entities.

use crate::physics::collision::{
    CollisionWorld,
    components::{CollidableComp, PlaneCollidableComp, SphereCollidableComp},
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a collidable, and if so, creates the corresponding collidable
/// and adds a [`CollidableComp`] to the entity.
pub fn setup_collidable_for_new_entity(
    collision_world: &RwLock<CollisionWorld>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |sphere_collidable: &SphereCollidableComp| -> CollidableComp {
            let collidable_id = collision_world
                .add_sphere_collidable(sphere_collidable.kind(), sphere_collidable.sphere());

            CollidableComp { collidable_id }
        }
    );

    setup!(
        {
            let mut collision_world = collision_world.write().unwrap();
        },
        components,
        |plane_collidable: &PlaneCollidableComp| -> CollidableComp {
            let collidable_id = collision_world
                .add_plane_collidable(plane_collidable.kind(), plane_collidable.plane());

            CollidableComp { collidable_id }
        }
    );
}
