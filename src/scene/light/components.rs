//! [`Component`](impact_ecs::component::Component)s related to light sources.

use crate::{
    components::ComponentRegistry,
    geometry::Degrees,
    rendering::fre,
    scene::{Irradiance, LightDirection, LightID, Radiance},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a [`Radiance`] and thus can act as a light source.
///
/// The purpose of this component is to aid in constructing an
/// [`OmnidirectionalLightComp`] or [`UnidirectionalLightComp`] for the entity.
/// It is therefore not kept after entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RadianceComp(pub Radiance);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniformly distributed [`Irradiance`] and thus can act
/// as an ambient light source.
///
/// The purpose of this component is to aid in constructing an
/// [`AmbientLightComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformIrradianceComp(pub Irradiance);

/// Setup [`Component`](impact_ecs::component::Component) for light source
/// initializing entities that have a physical extent, and thus produce soft
/// shadows.
///
/// The purpose of this component is to aid in constructing an
/// [`OmnidirectionalLightComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissionExtentComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for light source
/// initializing entities that have an angular extent, and thus produce soft
/// shadows.
///
/// The purpose of this component is to aid in constructing a
/// [`UnidirectionalLightComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AngularExtentComp(pub Degrees<fre>);

/// Setup [`Component`](impact_ecs::component::Component) for light source
/// initializing entities that have an omnidirectional distribution of radiance.
///
/// The purpose of this component is to aid in constructing a
/// [`OmnidirectionalLightComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OmnidirectionalComp;

/// [`Component`](impact_ecs::component::Component) for light source entities
/// that have a [`LightDirection`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DirectionComp(pub LightDirection);

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// [`AmbientLight`](crate::scene::AmbientLight).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AmbientLightComp {
    /// The ID of the entity's [`AmbientLight`](crate::scene::AmbientLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// [`OmnidirectionalLight`](crate::scene::OmnidirectionalLight).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OmnidirectionalLightComp {
    /// The ID of the entity's [`OmnidirectionalLight`](crate::scene::OmnidirectionalLight).
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

/// Registers all light [`Component`](impact_ecs::component::Component)s.
pub fn register_light_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, RadianceComp)?;
    register_setup_component!(registry, UniformIrradianceComp)?;
    register_setup_component!(registry, EmissionExtentComp)?;
    register_setup_component!(registry, AngularExtentComp)?;
    register_setup_component!(registry, OmnidirectionalComp)?;
    register_component!(registry, DirectionComp)?;
    register_component!(registry, AmbientLightComp)?;
    register_component!(registry, OmnidirectionalLightComp)?;
    register_component!(registry, UnidirectionalLightComp)
}
