//! Management of lights for entities.

use crate::{
    camera::SceneCamera,
    geometry::Degrees,
    gpu::rendering::fre,
    light::{
        components::{
            AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
            OmnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
        },
        AmbientLight, LightStorage, OmnidirectionalLight, UnidirectionalLight,
    },
    physics::ReferenceFrameComp,
    scene::RenderResourcesDesynchronized,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{Similarity3, UnitVector3};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the right
/// components for a light source, and if so, adds the corresponding light to
/// the light storage and adds a correspondong light component with the light's
/// ID to the entity.
pub fn setup_light_for_new_entity(
    scene_camera: &RwLock<Option<SceneCamera<fre>>>,
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    setup_ambient_light_for_new_entity(light_storage, components, desynchronized);
    setup_omnidirectional_light_for_new_entity(
        scene_camera,
        light_storage,
        components,
        desynchronized,
    );
    setup_unidirectional_light_for_new_entity(
        scene_camera,
        light_storage,
        components,
        desynchronized,
    );
}

/// Checks if the given entity has a light component, and if so, removes the
/// assocated light from the given [`LightStorage`].
pub fn cleanup_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    cleanup_ambient_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_omnidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
    cleanup_unidirectional_light_for_removed_entity(light_storage, entity, desynchronized);
}

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`AmbientLight`] to the light storage and adds an [`AmbientLightComp`] with
/// the light's ID to the entity.
pub fn setup_ambient_light_for_new_entity(
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
            let ambient_light = AmbientLight::new(
                super::compute_luminance_for_uniform_illuminance(&ambient_emission.illuminance),
            );
            let id = light_storage.add_ambient_light(ambient_light);

            AmbientLightComp { id }
        },
        ![AmbientLightComp]
    );
}

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`OmnidirectionalLight`] to the light storage and adds a
/// [`OmnidirectionalLightComp`] with the light's ID to the entity.
pub fn setup_omnidirectional_light_for_new_entity(
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
        |frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp|
         -> OmnidirectionalLightComp {
            let omnidirectional_light = OmnidirectionalLight::new(
                view_transform.transform_point(&frame.position.cast()),
                omnidirectional_emission.luminous_intensity,
                fre::max(omnidirectional_emission.source_extent, 0.0),
            );
            let id = light_storage.add_omnidirectional_light(omnidirectional_light);

            OmnidirectionalLightComp { id }
        },
        ![OmnidirectionalLightComp]
    );
}

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`UnidirectionalLight`] to the light storage and adds a
/// [`UnidirectionalLightComp`] with the light's ID to the entity.
pub fn setup_unidirectional_light_for_new_entity(
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
        |unidirectional_emission: &UnidirectionalEmissionComp| -> UnidirectionalLightComp {
            let unidirectional_light = UnidirectionalLight::new(
                // The view transform contains no scaling, so the direction remains normalized
                UnitVector3::new_unchecked(
                    view_transform.transform_vector(&unidirectional_emission.direction),
                ),
                unidirectional_emission.perpendicular_illuminance,
                Degrees(fre::max(
                    unidirectional_emission.angular_source_extent.0,
                    0.0,
                )),
            );
            let id = light_storage.add_unidirectional_light(unidirectional_light);

            UnidirectionalLightComp { id }
        },
        ![UnidirectionalLightComp]
    );
}

/// Checks if the given entity has a [`AmbientLightComp`], and if so, removes
/// the assocated [`AmbientLight`] from the given [`LightStorage`].
pub fn cleanup_ambient_light_for_removed_entity(
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

/// Checks if the given entity has a [`OmnidirectionalLightComp`], and if so,
/// removes the assocated [`OmnidirectionalLight`] from the given
/// [`LightStorage`].
pub fn cleanup_omnidirectional_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    if let Some(omnidirectional_light) = entity.get_component::<OmnidirectionalLightComp>() {
        let light_id = omnidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_omnidirectional_light(light_id);
        desynchronized.set_yes();
    }
}

/// Checks if the given entity has a [`UnidirectionalLightComp`], and if so,
/// removes the assocated [`UnidirectionalLight`] from the given
/// [`LightStorage`].
pub fn cleanup_unidirectional_light_for_removed_entity(
    light_storage: &RwLock<LightStorage>,
    entity: &EntityEntry<'_>,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    if let Some(unidirectional_light) = entity.get_component::<UnidirectionalLightComp>() {
        let light_id = unidirectional_light.access().id;
        light_storage
            .write()
            .unwrap()
            .remove_unidirectional_light(light_id);
        desynchronized.set_yes();
    }
}
