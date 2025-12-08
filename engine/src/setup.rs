//! Setup and cleanup for new and removed entities.

pub mod gizmo;
pub mod physics;
pub mod scene;

use crate::engine::Engine;
use anyhow::Result;
use impact_alloc::Allocator;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};

pub fn perform_setup_for_new_entities<A>(
    arena: A,
    engine: &Engine,
    components: &mut ArchetypeComponentStorage,
) -> Result<()>
where
    A: Allocator + Copy,
{
    scene::setup_scene_data_for_new_entities(
        arena,
        engine.resource_manager(),
        engine.scene(),
        engine.simulator(),
        components,
    )?;

    physics::setup_physics_for_new_entities(
        engine.resource_manager(),
        engine.simulator(),
        components,
    )?;

    scene::add_new_entities_to_scene_graph(
        engine.ecs_world(),
        engine.resource_manager(),
        engine.scene(),
        components,
    )?;

    gizmo::setup_gizmos_for_new_entities(engine.gizmo_manager(), components);

    let (setup_component_ids, setup_component_names, standard_component_names) =
        engine.extract_component_metadata(components);

    impact_log::debug!(
        "Creating {} entities:\nSetup components:\n    {}\nStandard components:\n    {}",
        components.component_count(),
        setup_component_names.join("\n    "),
        standard_component_names.join("\n    "),
    );

    // Remove all setup components
    components.remove_component_types_with_ids(setup_component_ids)?;

    Ok(())
}

pub fn perform_cleanup_for_removed_entity(engine: &Engine, entity: &EntityEntry<'_>) -> Result<()> {
    physics::cleanup_physics_for_removed_entity(engine.simulator(), entity);
    scene::cleanup_scene_data_for_removed_entity(engine.scene(), entity);
    Ok(())
}
