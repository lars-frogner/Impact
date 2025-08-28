//! Setup of lights for new entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use impact_geometry::ReferenceFrame;
use impact_light::{
    AmbientEmission, AmbientLightID, OmnidirectionalEmission, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalEmission, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLightID, setup,
};
use impact_scene::SceneEntityFlags;
use parking_lot::RwLock;

/// Checks if the entities-to-be with the given components have the right
/// components for a light source, and if so, adds the corresponding lights to
/// the light manager and adds the correspondong light components with the
/// lights' IDs to the entity.
pub fn setup_lights_for_new_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) {
    setup_ambient_lights_for_new_entities(scene, components);
    setup_omnidirectional_lights_for_new_entities(scene, components);
    setup_unidirectional_lights_for_new_entities(scene, components);
}

fn setup_ambient_lights_for_new_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let scene = scene.oread();
            let mut light_manager = scene.light_manager().owrite();
        },
        components,
        |ambient_emission: &AmbientEmission,
         flags: Option<&SceneEntityFlags>|
         -> (AmbientLightID, SceneEntityFlags) {
            (
                setup::setup_ambient_light(&mut light_manager, ambient_emission),
                flags.copied().unwrap_or_default(),
            )
        },
        ![AmbientLightID]
    );
}

fn setup_omnidirectional_lights_for_new_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        components,
        |frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (OmnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_omnidirectional_light(
                    &mut light_manager,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                ),
                flags,
            )
        },
        ![OmnidirectionalLightID]
    );

    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        components,
        |frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (ShadowableOmnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_shadowable_omnidirectional_light(
                    &mut light_manager,
                    &view_transform,
                    &frame.position.cast(),
                    omnidirectional_emission,
                    flags.into(),
                ),
                flags,
            )
        },
        ![ShadowableOmnidirectionalLightID]
    );
}

fn setup_unidirectional_lights_for_new_entities(
    scene: &RwLock<Scene>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        components,
        |unidirectional_emission: &UnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (UnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_unidirectional_light(
                    &mut light_manager,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                ),
                flags,
            )
        },
        ![UnidirectionalLightID]
    );

    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        components,
        |unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> (ShadowableUnidirectionalLightID, SceneEntityFlags) {
            let flags = flags.copied().unwrap_or_default();
            (
                setup::setup_shadowable_unidirectional_light(
                    &mut light_manager,
                    &view_transform,
                    unidirectional_emission,
                    flags.into(),
                ),
                flags,
            )
        },
        ![ShadowableUnidirectionalLightID]
    );
}

pub fn cleanup_light_for_removed_entity(scene: &RwLock<Scene>, entity: &EntityEntry<'_>) {
    if let Some(light_id) = entity.get_component::<AmbientLightID>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = *light_id.access();
        light_manager.remove_ambient_light(light_id);
    }

    if let Some(light_id) = entity.get_component::<OmnidirectionalLightID>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = *light_id.access();
        light_manager.remove_omnidirectional_light(light_id);
    }

    if let Some(light_id) = entity.get_component::<ShadowableOmnidirectionalLightID>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = *light_id.access();
        light_manager.remove_shadowable_omnidirectional_light(light_id);
    }

    if let Some(light_id) = entity.get_component::<UnidirectionalLightID>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = *light_id.access();
        light_manager.remove_unidirectional_light(light_id);
    }

    if let Some(light_id) = entity.get_component::<ShadowableUnidirectionalLightID>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = *light_id.access();
        light_manager.remove_shadowable_unidirectional_light(light_id);
    }
}
