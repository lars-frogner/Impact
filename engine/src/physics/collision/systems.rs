//! ECS systems related to collisions.

use crate::physics::{
    collision::{CollidableGeometry, CollisionWorld, components::CollidableComp},
    motion::components::ReferenceFrameComp,
};
use impact_ecs::{
    query,
    world::{EntityID, World as ECSWorld},
};
use impact_scene::SceneEntityFlags;

pub fn synchronize_collision_world<G: CollidableGeometry>(
    collision_world: &mut CollisionWorld<G>,
    ecs_world: &ECSWorld,
) {
    collision_world.clear_spatial_state();

    query!(ecs_world, |entity_id: EntityID,
                       collidable: &CollidableComp,
                       frame: &ReferenceFrameComp,
                       flags: &SceneEntityFlags| {
        if flags.is_disabled() {
            return;
        }
        synchronize_collidable(collision_world, entity_id, collidable, frame);
    });
}

pub fn synchronize_collidable<G: CollidableGeometry>(
    collision_world: &mut CollisionWorld<G>,
    entity_id: EntityID,
    collidable: &CollidableComp,
    frame: &ReferenceFrameComp,
) {
    let transform_to_world_space = frame.create_transform_to_parent_space();

    collision_world.synchronize_collidable(
        collidable.collidable_id,
        entity_id,
        transform_to_world_space,
    );
}
