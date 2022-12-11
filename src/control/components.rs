//! [`Component`](impact_ecs::component::Component)s related to user control.

use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// Marker [`Component`](impact_ecs::component::Component) for entities that can
/// be controlled by a user.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Controllable;
