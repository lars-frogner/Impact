//! Setup and cleanup for new and removed entities.

pub mod gizmo;
pub mod physics;
pub mod scene;

use crate::engine::Engine;
use anyhow::Result;
use impact_ecs::world::{EntityEntry, PrototypeEntities};

pub fn perform_setup_for_new_entities(
    engine: &Engine,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    scene::setup_scene_data_for_new_entities(
        engine.resource_manager(),
        engine.scene(),
        engine.simulator(),
        entities,
    )?;

    physics::setup_physics_for_new_entities(
        engine.resource_manager(),
        engine.simulator(),
        entities,
    )?;

    scene::add_new_entities_to_scene_graph(
        engine.ecs_world(),
        engine.resource_manager(),
        engine.scene(),
        entities,
    )?;

    gizmo::setup_gizmos_for_new_entities(engine.gizmo_manager(), entities);

    engine.app().on_new_entities(entities)?;

    let (setup_component_ids, setup_component_names, standard_component_names) =
        engine.extract_component_metadata(entities);

    log::info!(
        "Creating {} entities:\nSetup components:\n    {}\nStandard components:\n    {}",
        entities.count(),
        if setup_component_names.is_empty() {
            String::from("<None>")
        } else {
            setup_component_names.join("\n    ")
        },
        if standard_component_names.is_empty() {
            String::from("<None>")
        } else {
            standard_component_names.join("\n    ")
        },
    );

    // Remove all setup components
    entities.remove_component_types_with_ids(setup_component_ids)?;

    Ok(())
}

pub fn perform_cleanup_for_removed_entity(engine: &Engine, entity: &EntityEntry<'_>) -> Result<()> {
    physics::cleanup_physics_for_removed_entity(engine.simulator(), entity);
    scene::cleanup_scene_data_for_removed_entity(engine.scene(), entity);
    Ok(())
}
