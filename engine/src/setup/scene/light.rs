//! Setup of lights for new entities.

use impact_camera::buffer::BufferableCamera;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_geometry::ReferenceFrame;
use impact_light::{
    AmbientEmission, AmbientLightID, LightStorage, OmnidirectionalEmission, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalEmission, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLightID, setup,
};
use impact_scene::{SceneEntityFlags, camera::SceneCamera};
use nalgebra::Isometry3;
use std::sync::RwLock;

/// Checks if the entities-to-be with the given components have the right
/// components for a light source, and if so, adds the corresponding lights to
/// the light storage and adds the correspondong light components with the
/// lights' IDs to the entity.
pub fn setup_lights_for_new_entities(
    scene_camera: &RwLock<Option<SceneCamera>>,
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) {
    setup_ambient_lights_for_new_entities(light_storage, components, desynchronized);
    setup_omnidirectional_lights_for_new_entities(
        scene_camera,
        light_storage,
        components,
        desynchronized,
    );
    setup_unidirectional_lights_for_new_entities(
        scene_camera,
        light_storage,
        components,
        desynchronized,
    );
}

fn setup_ambient_lights_for_new_entities(
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) {
    setup!(
        {
            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |ambient_emission: &AmbientEmission,
         flags: Option<&SceneEntityFlags>|
         -> (AmbientLightID, SceneEntityFlags) {
            (
                setup::setup_ambient_light(&mut light_storage, ambient_emission, desynchronized),
                flags.copied().unwrap_or_default(),
            )
        },
        ![AmbientLightID]
    );
}

fn setup_omnidirectional_lights_for_new_entities(
    scene_camera: &RwLock<Option<SceneCamera>>,
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) {
    setup!(
        {
            let view_transform = scene_camera
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Isometry3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (OmnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_omnidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                flags,
            )
        },
        ![OmnidirectionalLightID]
    );

    setup!(
        {
            let view_transform = scene_camera
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Isometry3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (ShadowableOmnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_shadowable_omnidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                flags,
            )
        },
        ![ShadowableOmnidirectionalLightID]
    );
}

fn setup_unidirectional_lights_for_new_entities(
    scene_camera: &RwLock<Option<SceneCamera>>,
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) {
    setup!(
        {
            let view_transform = scene_camera
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Isometry3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |unidirectional_emission: &UnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (UnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_unidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                flags,
            )
        },
        ![UnidirectionalLightID]
    );

    setup!(
        {
            let view_transform = scene_camera
                .read()
                .unwrap()
                .as_ref()
                .map_or_else(Isometry3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (ShadowableUnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_shadowable_unidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                flags,
            )
        },
        ![ShadowableUnidirectionalLightID]
    );
}
