//! ECS systems for voxel object collidables.

use crate::{
    HasVoxelObject,
    collidable::{CollisionWorld, LocalCollidable},
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ModelTransform;
use impact_id::EntityID;
use impact_physics::collision::{CollidableID, HasCollidable};

pub fn sync_voxel_object_collidables(ecs_world: &ECSWorld, collision_world: &mut CollisionWorld) {
    query!(
        ecs_world,
        |entity_id: EntityID, model_transform: &ModelTransform| {
            let collidable_id = CollidableID::from_entity_id(entity_id);
            if let Some(LocalCollidable::VoxelObject(collidable)) =
                collision_world.get_local_collidable_mut(collidable_id)
            {
                collidable.origin_offset = model_transform.offset;
            }
        },
        [HasVoxelObject, HasCollidable]
    );
}
