//! ECS systems for voxel object collidables.

use crate::{
    HasVoxelObject,
    collidable::{CollisionWorld, LocalCollidable},
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ModelTransform;
use impact_physics::collision::CollidableID;

pub fn sync_voxel_object_collidables(ecs_world: &ECSWorld, collision_world: &mut CollisionWorld) {
    query!(
        ecs_world,
        |collidable_id: &CollidableID, model_transform: &ModelTransform| {
            if let Some(LocalCollidable::VoxelObject(collidable)) =
                collision_world.get_local_collidable_mut(*collidable_id)
            {
                collidable.origin_offset = model_transform.offset;
            }
        },
        [HasVoxelObject]
    );
}
