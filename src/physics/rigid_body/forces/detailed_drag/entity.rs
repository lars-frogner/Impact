//! Management of drag load maps for entities.

use super::DragLoadMapRepository;
use crate::{
    gpu::rendering::fre,
    mesh::{components::MeshComp, MeshID, MeshRepository},
    physics::rigid_body::{
        components::RigidBodyComp,
        forces::detailed_drag::{
            components::{DetailedDragComp, DragLoadMapComp},
            DragLoadMap, DragLoadMapConfig,
        },
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::{path::PathBuf, sync::RwLock};

/// Checks if the entity-to-be with the given components has the components
/// for obtaining an associated drag load map, and if so, loads or generates
/// the map and adds it to the drag load map repository if not present, then
/// adds the appropriate drag load map component to the entity.
pub fn setup_drag_load_map_for_new_entity(
    mesh_repository: &RwLock<MeshRepository<fre>>,
    drag_load_map_repository: &RwLock<DragLoadMapRepository<fre>>,
    components: &mut ArchetypeComponentStorage,
) {
    fn generate_map(
        mesh_repository: &RwLock<MeshRepository<fre>>,
        config: &DragLoadMapConfig,
        mesh_id: MeshID,
        rigid_body: &RigidBodyComp,
    ) -> DragLoadMap<fre> {
        let center_of_mass = rigid_body.0.inertial_properties().center_of_mass();

        let mesh_repository = mesh_repository.read().unwrap();
        let mesh = mesh_repository
            .get_mesh(mesh_id)
            .expect("Missing mesh for generating drag load map");

        let map = with_timing_info_logging!(
            "Generating drag load map with resolution {} and smoothness {} for {} using {} direction samples",
            config.n_theta_coords,
            mesh_id,
            config.smoothness,
            config.n_direction_samples; {
            DragLoadMap::<fre>::compute_from_mesh(
                mesh,
                center_of_mass,
                config.n_direction_samples,
                config.n_theta_coords,
                config.smoothness,
            )
        });

        map
    }

    fn generate_map_path(mesh_id: MeshID) -> PathBuf {
        // Ensure there are no path delimiters
        let sanitized_mesh_name = format!("{}", mesh_id).replace('/', "_").replace('\\', "_");
        PathBuf::from(format!("assets/drag_load_maps/{}.mpk", sanitized_mesh_name))
    }

    setup!(components, |drag: &DetailedDragComp,
                        mesh: &MeshComp,
                        rigid_body: &RigidBodyComp|
     -> DragLoadMapComp {
        let mesh_id = mesh.id;

        let drag_load_map_repository_readonly = drag_load_map_repository.read().unwrap();

        if !drag_load_map_repository_readonly.has_drag_load_map_for_mesh(mesh_id) {
            let config = drag_load_map_repository_readonly.config();

            let map_path = generate_map_path(mesh_id);
            let map_file_exists = map_path.exists();

            let map = if config.use_saved_maps && map_file_exists {
                DragLoadMap::<fre>::read_from_file(&map_path).unwrap_or_else(|err| {
                    log::error!("Could not load drag load map from file: {}", err);
                    generate_map(mesh_repository, config, mesh_id, rigid_body)
                })
            } else {
                generate_map(mesh_repository, config, mesh_id, rigid_body)
            };

            if config.save_generated_maps
                && (config.overwrite_existing_map_files || !map_file_exists)
            {
                if let Err(err) = map.save_to_file(&map_path) {
                    log::error!("Could not save drag load map to file: {}", err);
                }
            }

            // Release read lock before attempting to write
            drop(drag_load_map_repository_readonly);
            drag_load_map_repository
                .write()
                .unwrap()
                .add_drag_load_map_unless_present(mesh_id, map);
        }
        DragLoadMapComp {
            mesh_id,
            drag_coefficient: drag.drag_coefficient,
        }
    });
}
