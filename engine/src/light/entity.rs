//! Management of lights for entities.

use crate::{
    camera::SceneCamera,
    physics::motion::components::ReferenceFrameComp,
    scene::{SceneEntityFlags, components::SceneEntityFlagsComp},
};
use impact_camera::buffer::BufferableCamera;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_light::{
    LightStorage,
    components::{
        AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
        OmnidirectionalLightComp, ShadowableOmnidirectionalEmissionComp,
        ShadowableOmnidirectionalLightComp, ShadowableUnidirectionalEmissionComp,
        ShadowableUnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
    },
};
use nalgebra::Similarity3;
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
/// [`AmbientLight`] to the light storage and adds an [`AmbientLightComp`] with
/// the light's ID to the entity.
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
        |ambient_emission: &AmbientEmissionComp,
         flags: Option<&SceneEntityFlagsComp>|
         -> (AmbientLightComp, SceneEntityFlagsComp) {
            (
                impact_light::entity::setup_ambient_light(
                    &mut light_storage,
                    ambient_emission,
                    desynchronized,
                ),
                flags.copied().unwrap_or_default(),
            )
        },
        ![AmbientLightComp]
    );
}

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`OmnidirectionalLight`] or [`ShadowableOmnidirectionalLight`] to the light
/// storage and adds a [`OmnidirectionalLightComp`] or
/// [`ShadowableOmnidirectionalLightComp`] with the light's ID to the entity.
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
                .map_or_else(Similarity3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         flags: Option<&SceneEntityFlagsComp>|
         -> (OmnidirectionalLightComp, SceneEntityFlagsComp) {
            let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);
            (
                impact_light::entity::setup_omnidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                SceneEntityFlagsComp(flags),
            )
        },
        ![OmnidirectionalLightComp]
    );

    setup!(
        {
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
         omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
         flags: Option<&SceneEntityFlagsComp>|
         -> (ShadowableOmnidirectionalLightComp, SceneEntityFlagsComp) {
            let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);
            (
                impact_light::entity::setup_shadowable_omnidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                SceneEntityFlagsComp(flags),
            )
        },
        ![ShadowableOmnidirectionalLightComp]
    );
}

/// Checks if the entity-to-be with the given components has the right
/// components for this light source, and if so, adds the corresponding
/// [`UnidirectionalLight`] or [`ShadowableUnidirectionalLight`] to the light
/// storage and adds a [`UnidirectionalLightComp`] or
/// [`ShadowableUnidirectionalLightComp`] with the light's ID to the entity.
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
                .map_or_else(Similarity3::identity, |scene_camera| {
                    *scene_camera.view_transform()
                });

            let mut light_storage = light_storage.write().unwrap();
        },
        components,
        |unidirectional_emission: &UnidirectionalEmissionComp,
         flags: Option<&SceneEntityFlagsComp>|
         -> (UnidirectionalLightComp, SceneEntityFlagsComp) {
            let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);
            (
                impact_light::entity::setup_unidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                SceneEntityFlagsComp(flags),
            )
        },
        ![UnidirectionalLightComp]
    );

    setup!(
        {
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
        |unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         flags: Option<&SceneEntityFlagsComp>|
         -> (ShadowableUnidirectionalLightComp, SceneEntityFlagsComp) {
            let flags = flags.map_or_else(SceneEntityFlags::empty, |flags| flags.0);
            (
                impact_light::entity::setup_shadowable_unidirectional_light(
                    &mut light_storage,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                    desynchronized,
                ),
                SceneEntityFlagsComp(flags),
            )
        },
        ![ShadowableUnidirectionalLightComp]
    );
}
