//! [`Component`](impact_ecs::component::Component)s related to light sources.

use crate::{Illumninance, LightID, LuminousIntensity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use impact_math::Degrees;
use nalgebra::UnitVector3;
use roc_integration::roc;

/// [`Component`](impact_ecs::component::Component) for entities that produce a
/// spatially uniform and isotropic (ambient) light field.
#[roc(parents = "Comp", name = "AmbientEmission")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AmbientEmissionComp {
    /// The illuminance (incident flux per area) of a surface due to the ambient
    /// emission.
    ///
    /// # Unit
    /// Lux (lx = lm/m²)
    pub illuminance: Illumninance,
}

/// [`Component`](impact_ecs::component::Component) for entities that emit light
/// uniformly in all directions. The light can not be shadowed (use
/// [`ShadowableOmnidirectionalEmissionComp`] for light with shadows).
#[roc(parents = "Comp", name = "OmnidirectionalEmission")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OmnidirectionalEmissionComp {
    /// The luminous intensity of the emitted light.
    ///
    /// # Unit
    /// Candela (cd = lm/sr)
    pub luminous_intensity: LuminousIntensity,
    /// The physical extent of the light source, which determines the extent of
    /// specular highlights.
    ///
    /// # Unit
    /// Meter (m)
    pub source_extent: f32,
}

/// [`Component`](impact_ecs::component::Component) for entities that emit light
/// uniformly in all directions. The light can be shadowed (use
/// [`OmnidirectionalEmissionComp`] for light without shadows).
#[roc(parents = "Comp", name = "ShadowableOmnidirectionalEmission")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ShadowableOmnidirectionalEmissionComp {
    /// The luminous intensity of the emitted light.
    ///
    /// # Unit
    /// Candela (cd = lm/sr)
    pub luminous_intensity: LuminousIntensity,
    /// The physical extent of the light source, which determines the extent of
    /// specular highlights and the softness of shadows.
    ///
    /// # Unit
    /// Meter (m)
    pub source_extent: f32,
}

/// [`Component`](impact_ecs::component::Component) for entities that emit light
/// in a single direction. The light can not be shadowed (use
/// [`ShadowableUnidirectionalEmissionComp`] for light with shadows).
#[roc(parents = "Comp", name = "UnidirectionalEmission")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UnidirectionalEmissionComp {
    /// The illuminance (incident flux per area) of an illuminated surface
    /// perpendicular to the light direction.
    ///
    /// # Unit
    /// Lux (lx = lm/m²)
    pub perpendicular_illuminance: Illumninance,
    /// The direction of the emitted light.
    pub direction: UnitVector3<f32>,
    /// The angular extent of the light source, which determines the extent of
    /// specular highlights.
    pub angular_source_extent: Degrees<f32>,
}

/// [`Component`](impact_ecs::component::Component) for entities that emit light
/// in a single direction. The light can be shadowed (use
/// [`UnidirectionalEmissionComp`] for light without shadows).
#[roc(parents = "Comp", name = "ShadowableUnidirectionalEmission")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ShadowableUnidirectionalEmissionComp {
    /// The illuminance (incident flux per area) of an illuminated surface
    /// perpendicular to the light direction.
    ///
    /// # Unit
    /// Lux (lx = lm/m²)
    pub perpendicular_illuminance: Illumninance,
    /// The direction of the emitted light.
    pub direction: UnitVector3<f32>,
    /// The angular extent of the light source, which determines the extent of
    /// specular highlights and the softness of shadows.
    pub angular_source_extent: Degrees<f32>,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// [`AmbientLight`](crate::light::AmbientLight).
#[roc(parents = "Comp", name = "AmbientLight")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AmbientLightComp {
    /// The ID of the entity's [`AmbientLight`](crate::light::AmbientLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// [`OmnidirectionalLight`](crate::light::OmnidirectionalLight).
#[roc(parents = "Comp", name = "OmnidirectionalLight")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OmnidirectionalLightComp {
    /// The ID of the entity's
    /// [`OmnidirectionalLight`](crate::light::OmnidirectionalLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// [`ShadowableOmnidirectionalLight`](crate::light::ShadowableOmnidirectionalLight).
#[roc(parents = "Comp", name = "ShadowableOmnidirectionalLight")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ShadowableOmnidirectionalLightComp {
    /// The ID of the entity's
    /// [`ShadowableOmnidirectionalLight`](crate::light::ShadowableOmnidirectionalLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`UnidirectionalLight`](crate::light::UnidirectionalLight).
#[roc(parents = "Comp", name = "UnidirectionalLight")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UnidirectionalLightComp {
    /// The ID of the entity's
    /// [`UnidirectionalLight`](crate::light::UnidirectionalLight).
    pub id: LightID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`ShadowableUnidirectionalLight`](crate::light::ShadowableUnidirectionalLight).
#[roc(parents = "Comp", name = "ShadowableUnidirectionalLight")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ShadowableUnidirectionalLightComp {
    /// The ID of the entity's
    /// [`ShadowableUnidirectionalLight`](crate::light::ShadowableUnidirectionalLight).
    pub id: LightID,
}

#[roc]
impl AmbientEmissionComp {
    /// Creates a new ambient light emission component with the given
    /// illuminance (in lux).
    #[roc(body = "{ illuminance }")]
    pub fn new(illuminance: Illumninance) -> Self {
        Self { illuminance }
    }
}

#[roc]
impl OmnidirectionalEmissionComp {
    /// Creates a new omnidirectional emission component with the given
    /// luminous intensity (in candela) and source extent.
    #[roc(body = "{ luminous_intensity, source_extent }")]
    pub fn new(luminous_intensity: LuminousIntensity, source_extent: f32) -> Self {
        Self {
            luminous_intensity,
            source_extent,
        }
    }
}

#[roc]
impl ShadowableOmnidirectionalEmissionComp {
    /// Creates a new shadowable omnidirectional emission component with
    /// the given luminous intensity (in candela) and source extent.
    #[roc(body = "{ luminous_intensity, source_extent }")]
    pub fn new(luminous_intensity: LuminousIntensity, source_extent: f32) -> Self {
        Self {
            luminous_intensity,
            source_extent,
        }
    }
}

#[roc]
impl UnidirectionalEmissionComp {
    /// Creates a new unidirectional emission component with the given
    /// perpendicular illuminance (in lux), direction, and angular
    /// source extent.
    #[roc(body = "{ perpendicular_illuminance, direction, angular_source_extent }")]
    pub fn new(
        perpendicular_illuminance: Illumninance,
        direction: UnitVector3<f32>,
        angular_source_extent: Degrees<f32>,
    ) -> Self {
        Self {
            perpendicular_illuminance,
            direction,
            angular_source_extent,
        }
    }
}

#[roc]
impl ShadowableUnidirectionalEmissionComp {
    /// Creates a new shadowable unidirectional emission component with the
    /// given perpendicular illuminance (in lux), direction, and angular
    /// source extent.
    #[roc(body = "{ perpendicular_illuminance, direction, angular_source_extent }")]
    pub fn new(
        perpendicular_illuminance: Illumninance,
        direction: UnitVector3<f32>,
        angular_source_extent: Degrees<f32>,
    ) -> Self {
        Self {
            perpendicular_illuminance,
            direction,
            angular_source_extent,
        }
    }
}
