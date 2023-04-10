//! Ambient light sources.

use crate::{
    rendering::fre,
    scene::{
        AmbientLightComp, Irradiance, LightStorage, RenderResourcesDesynchronized,
        UniformIrradianceComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use std::sync::RwLock;

/// A spatially uniform and isotropic radiance field, represented by an RGB
/// irradiance.
///
/// This struct is intended to be stored in a [`LightStorage`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct AmbientLight {
    irradiance: Irradiance,
    // Padding to make size multiple of 16-bytes
    _padding: fre,
}

impl AmbientLight {
    fn new(irradiance: Irradiance) -> Self {
        Self {
            irradiance,
            _padding: 0.0,
        }
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`AmbientLight`] to the light storage and adds a [`AmbientLightComp`]
    /// with the light's ID to the entity.
    pub fn add_ambient_light_component_for_entity(
        light_storage: &RwLock<LightStorage>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();
                let mut light_storage = light_storage.write().unwrap();
            },
            components,
            |irradiance: &UniformIrradianceComp| -> AmbientLightComp {
                let ambient_light = Self::new(irradiance.0);
                let id = light_storage.add_ambient_light(ambient_light);

                AmbientLightComp { id }
            },
            ![AmbientLightComp]
        );
    }

    /// Checks if the given entity has a [`AmbientLightComp`], and if so,
    /// removes the assocated [`AmbientLight`] from the given [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(ambient_light) = entity.get_component::<AmbientLightComp>() {
            let light_id = ambient_light.access().id;
            light_storage
                .write()
                .unwrap()
                .remove_ambient_light(light_id);
            desynchronized.set_yes();
        }
    }
}
