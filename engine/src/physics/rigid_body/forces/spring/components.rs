//! [`Component`](impact_ecs::component::Component)s related to spring forces.

use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, world::EntityID};
use roc_integration::roc;

use super::{Spring, SpringState};
use crate::physics::motion::Position;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spring connecting two other entities.
#[roc(parents = "Comp", name = "Spring")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpringComp {
    /// The first entity the spring is attached to.
    pub entity_1_id: EntityID,
    /// The second entity the spring is attached to.
    pub entity_2_id: EntityID,
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

#[roc]
impl SpringComp {
    /// Creates a new component for a spring connecting two entities.
    #[roc(body = r#"
    {
        entity_1_id,
        entity_2_id,
        attachment_point_1,
        attachment_point_2,
        spring,
        spring_state: Physics.SpringState.new({})
    }"#)]
    pub fn new(
        entity_1_id: EntityID,
        entity_2_id: EntityID,
        attachment_point_1: Position,
        attachment_point_2: Position,
        spring: Spring,
    ) -> Self {
        Self {
            entity_1_id,
            entity_2_id,
            attachment_point_1,
            attachment_point_2,
            spring,
            spring_state: SpringState::new(),
        }
    }

    /// Creates a new component for a spring connecting the origins of two
    /// entities' reference frames.
    pub fn attached_to_origins(
        entity_1_id: EntityID,
        entity_2_id: EntityID,
        spring: Spring,
    ) -> Self {
        Self::new(
            entity_1_id,
            entity_2_id,
            Position::origin(),
            Position::origin(),
            spring,
        )
    }
}
