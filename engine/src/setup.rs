//! Setup and cleanup for new and removed entities.

pub mod gizmo;
pub mod physics;
pub mod scene;

use crate::{engine::Engine, lock_order::OrderedRwLock, setup::scene::camera::CameraRenderState};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};

pub fn perform_setup_for_new_entities(
    engine: &Engine,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    scene::setup_scene_data_for_new_entities(
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
        &mut || {
            let renderer = engine.renderer().oread();
            let postprocessor = renderer.postprocessor().oread();
            CameraRenderState {
                aspect_ratio: renderer.rendering_surface().surface_aspect_ratio(),
                jittering_enabled: postprocessor.temporal_anti_aliasing_config().enabled,
            }
        },
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
