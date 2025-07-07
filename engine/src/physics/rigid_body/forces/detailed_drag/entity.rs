//! Management of drag load maps for entities.

use super::DragLoadMapRepository;
use crate::physics::rigid_body::{
    components::RigidBodyComp,
    forces::detailed_drag::{
        DragLoadMap, DragLoadMapConfig,
        components::{DetailedDragComp, DragLoadMapComp},
    },
};
use anyhow::{Context, Result, anyhow};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_mesh::{MeshRepository, TriangleMeshID};
use std::{path::PathBuf, sync::RwLock};

/// Checks if the entity-to-be with the given components has the components
/// for obtaining an associated drag load map, and if so, loads or generates
/// the map and adds it to the drag load map repository if not present, then
/// adds the appropriate drag load map component to the entity.
pub fn setup_drag_load_map_for_new_entity(
    mesh_repository: &RwLock<MeshRepository>,
    drag_load_map_repository: &RwLock<DragLoadMapRepository<f32>>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    fn generate_map(
        mesh_repository: &RwLock<MeshRepository>,
        config: &DragLoadMapConfig,
        mesh_id: TriangleMeshID,
        rigid_body: &RigidBodyComp,
    ) -> Result<DragLoadMap<f32>> {
        impact_log::info!("Generating drag load map for mesh: {mesh_id}");

        let center_of_mass = rigid_body.0.inertial_properties().center_of_mass();

        let mesh_repository = mesh_repository.read().unwrap();
        let mesh = mesh_repository.get_triangle_mesh(mesh_id).ok_or_else(|| {
            anyhow!("Tried to generate drag load map for missing mesh (mesh ID {mesh_id})")
        })?;

        let map = impact_log::with_timing_info_logging!(
            "Generating drag load map with resolution {} and smoothness {} for {} using {} direction samples",
            config.n_theta_coords,
            mesh_id,
            config.smoothness,
            config.n_direction_samples; {
            DragLoadMap::<f32>::compute_from_mesh(
                mesh,
                center_of_mass,
                config.n_direction_samples,
                config.n_theta_coords,
                config.smoothness,
            )
        });

        Ok(map)
    }

    fn generate_map_path(mesh_id: TriangleMeshID) -> PathBuf {
        // Ensure there are no path delimiters
        let sanitized_mesh_name = format!("{mesh_id}").replace('/', "_").replace('\\', "_");
        PathBuf::from(format!("assets/drag_load_maps/{sanitized_mesh_name}.bc"))
    }

    setup!(components, |drag: &DetailedDragComp,
                        mesh_id: &TriangleMeshID,
                        rigid_body: &RigidBodyComp|
     -> Result<DragLoadMapComp> {
        let mesh_id = *mesh_id;

        let drag_load_map_repository_readonly = drag_load_map_repository.read().unwrap();

        if !drag_load_map_repository_readonly.has_drag_load_map_for_mesh(mesh_id) {
            let config = drag_load_map_repository_readonly.config();

            let map_path = generate_map_path(mesh_id);
            let map_file_exists = map_path.exists();

            let map = if config.use_saved_maps && map_file_exists {
                DragLoadMap::<f32>::read_from_file(&map_path).with_context(|| {
                    format!(
                        "Failed to load drag load map from file `{}`",
                        map_path.display()
                    )
                })?
            } else {
                generate_map(mesh_repository, config, mesh_id, rigid_body)?
            };

            if config.save_generated_maps
                && (config.overwrite_existing_map_files || !map_file_exists)
            {
                map.save_to_file(&map_path).with_context(|| {
                    format!(
                        "Failed to save drag load map to file `{}`",
                        map_path.display()
                    )
                })?;
            }

            // Release read lock before attempting to write
            drop(drag_load_map_repository_readonly);
            drag_load_map_repository
                .write()
                .unwrap()
                .add_drag_load_map_unless_present(mesh_id, map);
        }
        Ok(DragLoadMapComp {
            mesh_id,
            drag_coefficient: drag.drag_coefficient,
        })
    })
}
