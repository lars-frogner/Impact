//! Setup and cleanup for new and removed entities.

pub mod gizmo;
pub mod physics;
pub mod scene;

use crate::{engine::Engine, setup::scene::camera::CameraRenderState};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};

pub fn perform_setup_for_new_entities(
    engine: &Engine,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    let mut render_resources_desynchronized = false;

    scene::setup_scene_data_for_new_entities(
        &engine.scene().read(),
        engine.graphics_device(),
        &*engine.assets().read(),
        engine.simulator().read().rigid_body_manager(),
        components,
        &mut render_resources_desynchronized,
    )?;

    physics::setup_physics_for_new_entities(
        &engine.simulator().read(),
        engine.scene().read().mesh_repository(),
        components,
    )?;

    scene::add_new_entities_to_scene_graph(
        &engine.scene().read(),
        engine.ecs_world(),
        &mut || {
            let renderer = engine.renderer().read();
            let postprocessor = renderer.postprocessor().read();
            CameraRenderState {
                aspect_ratio: renderer.rendering_surface().surface_aspect_ratio(),
                jittering_enabled: postprocessor.temporal_anti_aliasing_config().enabled,
            }
        },
        components,
        &mut render_resources_desynchronized,
    )?;

    gizmo::setup_gizmos_for_new_entities(&engine.gizmo_manager().read(), components);

    if render_resources_desynchronized {
        engine
            .renderer()
            .read()
            .declare_render_resources_desynchronized();
    }

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
    let mut render_resources_desynchronized = false;

    physics::cleanup_physics_for_removed_entity(&engine.simulator().read(), entity);

    scene::cleanup_scene_data_for_removed_entity(
        &engine.scene().read(),
        entity,
        &mut render_resources_desynchronized,
    );

    if render_resources_desynchronized {
        engine
            .renderer()
            .read()
            .declare_render_resources_desynchronized();
    }

    Ok(())
}
