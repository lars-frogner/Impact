//! Setup of lights for new entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use anyhow::Result;
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;
use impact_light::{
    AmbientEmission, AmbientLightID, OmnidirectionalEmission, OmnidirectionalLightID,
    ShadowableOmnidirectionalEmission, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalEmission, ShadowableUnidirectionalLightID, UnidirectionalEmission,
    UnidirectionalLightID, setup,
};
use impact_scene::SceneEntityFlags;
use parking_lot::RwLock;

/// Checks if the given entities have the right components for a light source,
/// and if so, adds the corresponding lights to the light manager and adds the
/// correspondong light components with the
/// lights' IDs to the entity.
pub fn setup_lights_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup_ambient_lights_for_new_entities(scene, entities)?;
    setup_omnidirectional_lights_for_new_entities(scene, entities)?;
    setup_unidirectional_lights_for_new_entities(scene, entities)?;
    Ok(())
}

fn setup_ambient_lights_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let mut light_manager = scene.light_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         ambient_emission: &AmbientEmission,
         flags: Option<&SceneEntityFlags>|
         -> Result<SceneEntityFlags> {
            setup::setup_ambient_light(&mut light_manager, entity_id, ambient_emission)?;
            Ok(flags.copied().unwrap_or_default())
        }
    )
}

fn setup_omnidirectional_lights_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &OmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> Result<SceneEntityFlags> {
            let position = frame.position.aligned();
            let flags = flags.copied().unwrap_or_default();
            setup::setup_omnidirectional_light(
                &mut light_manager,
                entity_id,
                &view_transform,
                &position,
                omnidirectional_emission,
                flags.into(),
            )?;
            Ok(flags)
        }
    )?;

    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         frame: &ReferenceFrame,
         omnidirectional_emission: &ShadowableOmnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> Result<SceneEntityFlags> {
            let position = frame.position.aligned();
            let flags = flags.copied().unwrap_or_default();
            setup::setup_shadowable_omnidirectional_light(
                &mut light_manager,
                entity_id,
                &view_transform,
                &position,
                omnidirectional_emission,
                flags.into(),
            )?;
            Ok(flags)
        }
    )
}

fn setup_unidirectional_lights_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         unidirectional_emission: &UnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> Result<SceneEntityFlags> {
            let flags = flags.copied().unwrap_or_default();
            setup::setup_unidirectional_light(
                &mut light_manager,
                entity_id,
                &view_transform,
                unidirectional_emission,
                flags.into(),
            )?;
            Ok(flags)
        }
    )?;

    setup!(
        {
            let scene = scene.oread();
            let view_transform = scene.camera_manager().oread().active_view_transform();
            let mut light_manager = scene.light_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         unidirectional_emission: &ShadowableUnidirectionalEmission,
         flags: Option<&SceneEntityFlags>|
         -> Result<SceneEntityFlags> {
            let flags = flags.copied().unwrap_or_default();
            setup::setup_shadowable_unidirectional_light(
                &mut light_manager,
                entity_id,
                &view_transform,
                unidirectional_emission,
                flags.into(),
            )?;
            Ok(flags)
        }
    )
}

pub fn cleanup_light_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<AmbientEmission>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = AmbientLightID::from_entity_id(entity_id);
        light_manager.remove_ambient_light(light_id);
    }

    if entity.has_component::<OmnidirectionalEmission>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = OmnidirectionalLightID::from_entity_id(entity_id);
        light_manager.remove_omnidirectional_light(light_id);
    }

    if entity.has_component::<ShadowableOmnidirectionalEmission>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = ShadowableOmnidirectionalLightID::from_entity_id(entity_id);
        light_manager.remove_shadowable_omnidirectional_light(light_id);
    }

    if entity.has_component::<UnidirectionalEmission>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = UnidirectionalLightID::from_entity_id(entity_id);
        light_manager.remove_unidirectional_light(light_id);
    }

    if entity.has_component::<ShadowableUnidirectionalEmission>() {
        let scene = scene.oread();
        let mut light_manager = scene.light_manager().owrite();
        let light_id = ShadowableUnidirectionalLightID::from_entity_id(entity_id);
        light_manager.remove_shadowable_unidirectional_light(light_id);
    }
}
