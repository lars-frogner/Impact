//! Setup of detailed drag forces.

use crate::{
    force::{
        ForceGeneratorManager,
        detailed_drag::{
            DetailedDragForce, DetailedDragForceGenerator, DetailedDragForceGeneratorID,
            DragLoadMap, DragLoadMapConfig, DragLoadMapID,
        },
    },
    fph,
    quantities::Position,
    rigid_body::DynamicRigidBodyID,
};
#[cfg(feature = "bincode")]
use anyhow::Context;
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_geometry::ModelTransform;
use impact_math::StringHash64;
use nalgebra::Point3;
use roc_integration::roc;
use std::path::PathBuf;

define_setup_type! {
    target = DetailedDragForceGeneratorID;
    /// The properties governing the effect of a shape-dependent drag on a body.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DetailedDragProperties {
        /// The drag coefficient of the body.
        pub drag_coefficient: fph,
    }
}

#[roc]
impl DetailedDragProperties {
    #[roc(body = "{ drag_coefficient }")]
    pub fn new(drag_coefficient: fph) -> Self {
        Self { drag_coefficient }
    }
}

pub fn setup_detailed_drag_force<'a>(
    force_generator_manager: &mut ForceGeneratorManager,
    drag_properties: DetailedDragProperties,
    rigid_body_id: DynamicRigidBodyID,
    model_transform: &ModelTransform,
    drag_load_map_id: StringHash64,
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3<f32>; 3]>,
) -> Result<DetailedDragForceGeneratorID> {
    let drag_load_map_id = DragLoadMapID(drag_load_map_id);

    let detailed_drag_force_registry = force_generator_manager.detailed_drag_forces_mut();
    let drag_load_map_repository = detailed_drag_force_registry.drag_load_map_repository_mut();

    if !drag_load_map_repository.has_drag_load_map(drag_load_map_id) {
        let config = drag_load_map_repository.config();

        let map_path = generate_map_path(drag_load_map_id);
        let map_file_exists = map_path.exists();

        let map = if config.use_saved_maps && map_file_exists {
            #[cfg(not(feature = "bincode"))]
            anyhow::bail!("Enable the `bincode` feature to read drag load maps");
            #[cfg(feature = "bincode")]
            DragLoadMap::<f32>::read_from_file(&map_path).with_context(|| {
                format!(
                    "Failed to load drag load map from file `{}`",
                    map_path.display()
                )
            })?
        } else {
            let center_of_mass = Point3::from(model_transform.offset.cast());
            let map = generate_map(
                config,
                &center_of_mass,
                drag_load_map_id,
                triangle_vertex_positions,
            )?;

            if config.save_generated_maps
                && (config.overwrite_existing_map_files || !map_file_exists)
            {
                #[cfg(not(feature = "bincode"))]
                anyhow::bail!("Enable the `bincode` feature to save drag load maps");
                #[cfg(feature = "bincode")]
                map.save_to_file(&map_path).with_context(|| {
                    format!(
                        "Failed to save drag load map to file `{}`",
                        map_path.display()
                    )
                })?;
            }

            map
        };

        drag_load_map_repository.add_drag_load_map_unless_present(drag_load_map_id, map);
    }

    let generator_id = detailed_drag_force_registry
        .generators_mut()
        .insert_generator(DetailedDragForceGenerator {
            body: rigid_body_id,
            force: DetailedDragForce {
                drag_coefficient: drag_properties.drag_coefficient,
                drag_load_map: drag_load_map_id,
                scaling: fph::from(model_transform.scale),
            },
        });

    Ok(generator_id)
}

fn generate_map<'a>(
    config: &DragLoadMapConfig,
    center_of_mass: &Position,
    drag_load_map_id: DragLoadMapID,
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3<f32>; 3]>,
) -> Result<DragLoadMap<f32>> {
    impact_log::info!("Generating drag load map: {drag_load_map_id}");

    let map = impact_log::with_timing_info_logging!(
        "Generating drag load map with resolution {} and smoothness {} for {} using {} direction samples",
        config.n_theta_coords,
        drag_load_map_id,
        config.smoothness,
        config.n_direction_samples; {
        DragLoadMap::<f32>::compute_from_mesh(
            triangle_vertex_positions,
            center_of_mass,
            config.n_direction_samples,
            config.n_theta_coords,
            config.smoothness,
        )
    });

    Ok(map)
}

fn generate_map_path(drag_load_map_id: DragLoadMapID) -> PathBuf {
    // Ensure there are no path delimiters
    let sanitized_map_name = format!("{drag_load_map_id}")
        .replace('/', "_")
        .replace('\\', "_");
    PathBuf::from(format!("assets/drag_load_maps/{sanitized_map_name}.bc"))
}
