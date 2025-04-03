//! [`Component`](impact_ecs::component::Component)s related to spring forces.

use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, world::Entity};

use super::{Spring, SpringState};
use crate::physics::motion::Position;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spring connecting two other entities.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpringComp {
    /// The first entity the spring is attached to.
    pub entity_1: Entity,
    /// The second entity the spring is attached to.
    pub entity_2: Entity,
    /// The point where the spring is attached to the first entity, in that
    /// entity's reference frame.
    pub attachment_point_1: Position,
    /// The point where the spring is attached to the second entity, in that
    /// entity's reference frame.
    pub attachment_point_2: Position,
    /// The spring connecting the entities.
    pub spring: Spring,
    /// The current state of the spring.
    pub spring_state: SpringState,
}

impl SpringComp {
    /// Creates a new component for a spring connecting two entities.
    pub fn new(
        entity_1: Entity,
        entity_2: Entity,
        attachment_point_1: Position,
        attachment_point_2: Position,
        spring: Spring,
    ) -> Self {
        Self {
            entity_1,
            entity_2,
            attachment_point_1,
            attachment_point_2,
            spring,
            spring_state: SpringState::new(),
        }
    }

    /// Creates a new component for a spring connecting the origins of two
    /// entities' reference frames.
    pub fn attached_to_origins(entity_1: Entity, entity_2: Entity, spring: Spring) -> Self {
        Self::new(
            entity_1,
            entity_2,
            Position::origin(),
            Position::origin(),
            spring,
        )
    }
}
