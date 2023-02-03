//! [`Component`](impact_ecs::component::Component)s related to light sources.

use crate::scene::{LightID, Radiance};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`PointLight`](crate::scene::PointLight).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PointLightComp {
    /// The ID of the entity's [`PointLight`](crate::scene::PointLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`Radiance`] and thus can act as a light source.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RadianceComp(pub Radiance);

/// Marker [`Component`](impact_ecs::component::Component) for light source
/// entities that have a omnidirectional distribution of radiance.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Omnidirectional;
