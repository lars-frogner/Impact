//! ECS systems related to collisions.

use crate::{
    physics::{
        collision::{CollisionWorld, components::CollidableComp},
        motion::components::ReferenceFrameComp,
    },
    scene::components::SceneEntityFlagsComp,
};
use impact_ecs::{
    query,
    world::{EntityID, World as ECSWorld},
};

pub fn synchronize_collision_world(collision_world: &mut CollisionWorld, ecs_world: &ECSWorld) {
    collision_world.clear_spatial_state();

    query!(
        ecs_world,
        |entity_id: EntityID,
         collidable: &CollidableComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }

            let transform_to_world_space = frame.create_transform_to_parent_space();

            collision_world.synchronize_collidable(
                collidable.collidable_id,
                entity_id,
                transform_to_world_space,
            );
        }
    );
}
