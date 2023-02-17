//! Unidirectional light sources.

use crate::{
    rendering::fre,
    scene::{
        DirectionComp, DirectionalLightComp, LightDirection, LightStorage, Radiance, RadianceComp,
        RenderResourcesDesynchronized, SceneCamera,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::Similarity3;
use std::sync::RwLock;

/// An directional light source represented by a camera space direction and an
/// RGB radiance.
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
pub struct DirectionalLight {
    camera_space_direction: LightDirection,
    _padding_1: fre,
    radiance: Radiance,
    _padding_2: fre,
}

impl DirectionalLight {
    fn new(camera_space_direction: LightDirection, radiance: Radiance) -> Self {
        Self {
            camera_space_direction,
            _padding_1: 0.0,
            radiance,
            _padding_2: 0.0,
        }
    }

    /// Sets the camera space direction of the light to the given direction.
    pub fn set_camera_space_direction(&mut self, camera_space_direction: LightDirection) {
        self.camera_space_direction = camera_space_direction;
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`DirectionalLight`] to the light storage and adds a
    /// [`DirectionalLightComp`] with the light's ID to the entity.
    pub fn add_directional_light_component_for_entity(
        scene_camera: &RwLock<Option<SceneCamera<fre>>>,
        light_storage: &RwLock<LightStorage>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();

                let view_transform = scene_camera
                    .read()
                    .unwrap()
                    .as_ref()
                    .map_or_else(Similarity3::identity, |scene_camera| {
                        *scene_camera.view_transform()
                    });

                let mut light_storage = light_storage.write().unwrap();
            },
            components,
            |direction: &DirectionComp, radiance: &RadianceComp| -> DirectionalLightComp {
                let directional_light = Self::new(
                    // The view transform contains no scaling, so the direction remains normalized
                    LightDirection::new_unchecked(
                        view_transform.transform_vector(&direction.0.cast()),
                    ),
                    radiance.0,
                );
                let id = light_storage.add_directional_light(directional_light);

                DirectionalLightComp { id }
            },
            ![DirectionalLightComp]
        );
    }

    /// Checks if the given entity has a [`DirectionalLightComp`], and if so,
    /// removes the assocated [`DirectionalLight`] from the given
    /// [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(directional_light) = entity.get_component::<DirectionalLightComp>() {
            let light_id = directional_light.access().id;
            light_storage
                .write()
                .unwrap()
                .remove_directional_light(light_id);
            desynchronized.set_yes();
        }
    }
}
