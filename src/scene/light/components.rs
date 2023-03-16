//! [`Component`](impact_ecs::component::Component)s related to light sources.

use crate::{
    geometry::Degrees,
    rendering::fre,
    scene::{LightDirection, LightID, Radiance},
};
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
/// have a [`UnidirectionalLight`](crate::scene::UnidirectionalLight).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UnidirectionalLightComp {
    /// The ID of the entity's [`UnidirectionalLight`](crate::scene::UnidirectionalLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for light source entities
/// that have a [`LightDirection`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DirectionComp(pub LightDirection);

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`Radiance`] and thus can act as a light source.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RadianceComp(pub Radiance);

/// [`Component`](impact_ecs::component::Component) for light source entities
/// that have an extent.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissionExtentComp(pub fre);

/// [`Component`](impact_ecs::component::Component) for light source entities
/// that have an angular extent, and thus produce soft shadows.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AngularExtentComp(pub Degrees<fre>);

/// Marker [`Component`](impact_ecs::component::Component) for light source
/// entities that have a omnidirectional distribution of radiance.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Omnidirectional;
