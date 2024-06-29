//! Ambient light sources.

use crate::{
    rendering::fre,
    scene::{
        self, AmbientEmissionComp, AmbientLightComp, Illumninance, LightStorage, Luminance,
        RenderResourcesDesynchronized,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use std::sync::RwLock;

/// A spatially uniform and isotropic light field, represented by an RGB
/// incident luminance that applies to any surface affected by the light.
///
/// This struct is intended to be stored in a [`LightStorage`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct AmbientLight {
    luminance: Luminance,
    // Padding to make size multiple of 16-bytes
    _padding: fre,
}

impl AmbientLight {
    fn new(luminance: Luminance) -> Self {
        Self {
            luminance,
            _padding: 0.0,
        }
    }

    /// Sets the uniform illuminance due to the light to the given value.
    pub fn set_illuminance(&mut self, illuminance: Illumninance) {
        self.luminance = scene::compute_luminance_for_uniform_illuminance(&illuminance);
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`AmbientLight`] to the light storage and adds an [`AmbientLightComp`]
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
            |ambient_emission: &AmbientEmissionComp| -> AmbientLightComp {
                let ambient_light = Self::new(scene::compute_luminance_for_uniform_illuminance(
                    &ambient_emission.illuminance,
                ));
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
