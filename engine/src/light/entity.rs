//! Management of lights for entities.

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

/// Checks if the entity-to-be with the given components has the right
/// components for a light source, and if so, adds the corresponding light to
/// the light storage and adds a correspondong light component with the light's
/// ID to the entity.
pub fn setup_light_for_new_entity(
    scene_camera: &RwLock<Option<SceneCamera>>,
    light_storage: &RwLock<LightStorage>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
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

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`AmbientLight`](impact_light::AmbientLight) to the light storage and adds
/// an [`AmbientLightID`] with the light's ID to the entity.
fn setup_ambient_light_for_new_entity(
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

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`OmnidirectionalLight`](impact_light::OmnidirectionalLight) or
/// [`ShadowableOmnidirectionalLight`](impact_light::ShadowableOmnidirectionalLight)
/// to the light storage and adds a [`OmnidirectionalLightID`] or
/// [`ShadowableOmnidirectionalLightID`] with the light's ID to the entity.
fn setup_omnidirectional_light_for_new_entity(
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

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`UnidirectionalLight`](impact_light::UnidirectionalLight) or
/// [`ShadowableUnidirectionalLight`](impact_light::ShadowableUnidirectionalLight)
/// to the light storage and adds a [`UnidirectionalLightID`] or
/// [`ShadowableUnidirectionalLightID`] with the light's ID to the entity.
fn setup_unidirectional_light_for_new_entity(
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
