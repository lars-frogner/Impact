//! Omnidirectional light sources.

use crate::{
    physics::PositionComp,
    rendering::fre,
    scene::{LightStorage, Omnidirectional, PointLightComp, Radiance, RadianceComp},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::Point3;
use std::sync::RwLock;

/// An point light source represented by a position and an RGB radiance.
///
/// This struct is intended to be stored in a [`LightStorage`], and its data
/// will be passed directly to the GPU in a uniform buffer. Since the size of a
/// uniform has to be a multiple of 16 bytes, the struct is padded to 32 bytes.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PointLight {
    position: Point3<fre>,
    _padding_1: fre,
    radiance: Radiance,
    _padding_2: fre,
}

impl PointLight {
    fn new(position: Point3<fre>, radiance: Radiance) -> Self {
        Self {
            position,
            _padding_1: 0.0,
            radiance,
            _padding_2: 0.0,
        }
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`PointLight`] to the light storage and adds a [`PointLightComp`] with
    /// the light's ID to the entity.
    pub fn add_point_light_component_for_entity(
        light_storage: &RwLock<LightStorage>,
        components: &mut ArchetypeComponentStorage,
    ) {
        setup!(
            {
                let mut light_storage = light_storage.write().unwrap();
            },
            components,
            |position: &PositionComp, radiance: &RadianceComp| -> PointLightComp {
                let point_light = Self::new(position.0.cast(), radiance.0);
                let id = light_storage.add_point_light(point_light);

                PointLightComp { id }
            },
            [Omnidirectional],
            ![PointLightComp]
        );
    }

    /// Checks if the given entity has a [`PointLightComp`], and if so, removes
    /// the assocated [`PointLight`] from the given [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
    ) {
        if let Some(point_light) = entity.get_component::<PointLightComp>() {
            let light_id = point_light.access().id;
            light_storage.write().unwrap().remove_point_light(light_id);
        }
    }
}
