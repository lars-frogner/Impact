//! [`Component`](impact_ecs::component::Component)s related to rendering.

use crate::rendering::MaterialID;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`MaterialSpecification`](crate::rendering::MaterialSpecification).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MaterialComp {
    /// The ID of the entity's [`MaterialSpecification`](crate::rendering::MaterialSpecification).
    pub material_id: MaterialID,
}
