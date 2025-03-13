//! [`Component`](impact_ecs::component::Component)s related to collisions.

use crate::physics::collision::CollidableID;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// collidable in the [`CollisionWorld`](super::CollisionWorld).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CollidableComp {
    /// The ID of the entity's collidable.
    pub collidable_id: CollidableID,
}
