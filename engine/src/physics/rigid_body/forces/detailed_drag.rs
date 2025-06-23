//! Drag force and torque computed from aggregating drag on each point on the
//! body.

pub mod components;
mod drag_load;
pub mod entity;
mod equirectangular_map;
pub mod systems;

pub use drag_load::DragLoad;
use impact_containers::HashMap;
use serde::{Deserialize, Serialize};

use crate::physics::{
    fph,
    motion::{Direction, Position},
};
use anyhow::{Result, anyhow, bail};
use impact_math::{Angle, Float, Radians};
use impact_mesh::{MeshID, triangle::TriangleMesh};
use simba::scalar::SubsetOf;
use std::collections::hash_map::Entry;

use drag_load::AveragingDragLoad;
use equirectangular_map::EquirectangularMap;

/// A map containing a [`DragLoad`] for each direction of motion relative to the
/// medium. The directions are discretized onto a 2D grid using an
/// equirectangular projection (meaning the grid coordinates are the spherical
/// azimuthal angle phi and polar angle theta).
pub type DragLoadMap<F> = EquirectangularMap<DragLoad<F>>;

/// Repository where [`DragLoadMap`]s are stored under a unique [`MeshID`].
#[derive(Debug)]
pub struct DragLoadMapRepository<F: Float> {
    config: DragLoadMapConfig,
    drag_load_maps: HashMap<MeshID, DragLoadMap<F>>,
}

/// Configuration parameters for the generation of drag load maps.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DragLoadMapConfig {
    /// The number of uniformly distributed directions the aggregate drag load
    /// on the body should be computed for. More directions gives a more
    /// accurate map, but increases generation time.
    pub n_direction_samples: usize,
    /// The number of increments the full range [0, pi] of polar angles theta
    /// should be divided into in the equirectangular drag load map. The number
    /// of azimuthal angles phi, with range [0, 2*pi], will be double this.
    /// Higher resolutions can represent more abrupt changes in drag load, but
    /// require more direction samples to yield good results.
    pub n_theta_coords: usize,
    /// How smoothed out the forces and torques in the drag load map will be.
    /// For a smoothness of one, each location in the map will most likely hold
    /// the drag load from the closest direction sample. For higher values, more
    /// distant direction samples will influence the drag load at each location.
    /// For values lower than one, there may be locations with a default zero
    /// drag load because there is no sufficiently close direction sample, so
    /// this should be avoided.
    pub smoothness: fph,
    /// Whether to store newly generated maps as files on disk.
    pub save_generated_maps: bool,
    /// Whether to overwrite any existing map file with the same name when
    /// storing a generated map to file.
    pub overwrite_existing_map_files: bool,
    /// Whether to check whether a file with the required map is available
    /// before generating the map.
    pub use_saved_maps: bool,
}

impl<F: Float> DragLoadMapRepository<F> {
    /// Creates a new empty drag load map repository with the given
    /// configuration parameters.
    ///
    /// # Errors
    /// Returns an error if the given configuration is invalid.
    pub fn new(config: DragLoadMapConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            drag_load_maps: HashMap::default(),
        })
    }

    /// Returns a reference to the configuration for generating drag load maps.
    pub fn config(&self) -> &DragLoadMapConfig {
        &self.config
    }

    /// Returns a reference to the [`DragLoadMap`] for the mesh with the given
    /// ID.
    ///
    /// # Panics
    /// If no map is present for the mesh with the given ID.
    pub fn drag_load_map(&self, mesh_id: MeshID) -> &DragLoadMap<F> {
        self.get_drag_load_map(mesh_id)
            .expect("Tried to obtain missing drag load map")
    }

    /// Returns a reference to the [`DragLoadMap`] for the mesh with the given
    /// ID, or [`None`] if the map is not present.
    pub fn get_drag_load_map(&self, mesh_id: MeshID) -> Option<&DragLoadMap<F>> {
        self.drag_load_maps.get(&mesh_id)
    }

    /// Whether a drag load map for the mesh with the given ID is present.
    pub fn has_drag_load_map_for_mesh(&self, mesh_id: MeshID) -> bool {
        self.drag_load_maps.contains_key(&mesh_id)
    }

    /// Includes the given drag load map in the repository under the given mesh
    /// ID.
    ///
    /// # Errors
    /// Returns an error if a map for the mesh with the given ID already exists.
    /// The repository will remain unchanged.
    pub fn add_drag_load_map(&mut self, mesh_id: MeshID, map: DragLoadMap<F>) -> Result<()> {
        match self.drag_load_maps.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(map);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Drag load map for mesh {} already present in repository",
                mesh_id
            )),
        }
    }

    /// Includes the given drag load map in the repository under the given mesh
    /// ID, unless a map for a mesh with the same ID is already present.
    pub fn add_drag_load_map_unless_present(&mut self, mesh_id: MeshID, map: DragLoadMap<F>) {
        let _ = self.add_drag_load_map(mesh_id, map);
    }
}

impl DragLoadMapConfig {
    fn validate(&self) -> Result<()> {
        if self.n_direction_samples == 0 {
            bail!(
                "Invalid number of direction samples for drag load map: {}",
                self.n_direction_samples
            );
        }
        if self.n_theta_coords == 0 {
            bail!(
                "Invalid number of theta coordinates for drag load map: {}",
                self.n_theta_coords
            );
        }
        if self.smoothness <= 0.0 {
            bail!("Invalid smoothness for drag load map: {}", self.smoothness);
        }
        Ok(())
    }
}

impl Default for DragLoadMapConfig {
    fn default() -> Self {
        Self {
            n_direction_samples: 5000,
            n_theta_coords: 64,
            smoothness: 2.0,
            save_generated_maps: true,
            overwrite_existing_map_files: false,
            use_saved_maps: true,
        }
    }
}

impl<F: Float> DragLoadMap<F> {
    /// Computes a drag load map for the given mesh with the given center of
    /// mass, using the given number of direction samples and map resoulution
    /// and smoothness.
    ///
    /// # Panics
    /// If the given number of direction samples or theta coordinates is zero.
    pub fn compute_from_mesh<FMESH>(
        mesh: &TriangleMesh<FMESH>,
        center_of_mass: &Position,
        n_direction_samples: usize,
        n_theta_coords: usize,
        smoothness: fph,
    ) -> Self
    where
        FMESH: Float + SubsetOf<fph>,
        fph: SubsetOf<F>,
    {
        assert_ne!(
            n_direction_samples, 0,
            "Tried to compute drag load map based on zero direction samples"
        );
        assert_ne!(
            n_theta_coords, 0,
            "Tried to compute zero sized drag load map"
        );

        let angular_interpolation_distance =
            compute_angular_interpolation_distance_from_smoothness(smoothness, n_direction_samples);

        let drag_loads =
            drag_load::compute_aggregate_drag_loads_for_uniformly_distributed_directions(
                mesh,
                center_of_mass,
                n_direction_samples,
            );

        let map = generate_map_from_drag_loads(
            &drag_loads,
            n_theta_coords,
            angular_interpolation_distance,
        );

        map
    }
}

fn generate_map_from_drag_loads<F>(
    drag_loads: &[(Direction, DragLoad<fph>)],
    n_theta_coords: usize,
    angular_interpolation_distance: Radians<fph>,
) -> DragLoadMap<F>
where
    F: Float,
    fph: SubsetOf<F>,
{
    let angular_interpolation_distance = angular_interpolation_distance.radians();
    assert!(angular_interpolation_distance > 0.0);

    let mut averaging_map = EquirectangularMap::<AveragingDragLoad<fph>>::empty(n_theta_coords);

    for (direction, load) in drag_loads {
        let direction_phi = compute_phi(direction);
        let direction_theta = compute_theta(direction);

        // Increase the interpolation distance when we are closer to the poles
        // to account for the disproportionate number of grid cells that must be
        // filled near the poles in the equirectangular map relative to the
        // local (uniform) density of samples. We increase the distance by a
        // maximum factor of four to prevent the samples near the poles from
        // becoming too influential.
        let scaled_angular_interpolation_distance =
            angular_interpolation_distance / (1.0 - 0.75 * fph::abs(direction.z));

        let inverse_scaled_angular_interpolation_distance =
            1.0 / scaled_angular_interpolation_distance;

        for (phi_idx, theta_idx, angular_distance) in averaging_map
            .find_angle_indices_and_angular_distances_for_region(
                direction_phi,
                direction_theta,
                Radians(scaled_angular_interpolation_distance),
            )
        {
            let weight = compute_weight(
                angular_distance.radians() * inverse_scaled_angular_interpolation_distance,
            );

            let averaging_drag_load = averaging_map.value_at_indices_mut(phi_idx, theta_idx);

            averaging_drag_load.add_sample(load, weight);
        }
    }

    averaging_map.map_values(|averaging_load| averaging_load.into_average_load())
}

fn compute_phi(direction: &Direction) -> Radians<fph> {
    Radians(fph::atan2(direction.y, direction.x))
}

fn compute_theta(direction: &Direction) -> Radians<fph> {
    Radians(fph::acos(direction.z))
}

fn compute_angular_interpolation_distance_from_smoothness(
    smoothness: fph,
    n_direction_samples: usize,
) -> Radians<fph> {
    // For a smoothness of one, the angular areas covered by the quadratic
    // angular regions surrounding the direction samples add exactly up to the
    // total angular area of the sphere
    Radians(smoothness * fph::sqrt(4.0 * fph::PI / (n_direction_samples as fph)))
}

fn compute_weight(normalized_angular_distance: fph) -> fph {
    // Use a quartic weighting function with finite support (it reaches zero
    // when normalized angular distance reaches unity)
    fph::max(0.0, 1.0 - normalized_angular_distance.powi(2)).powi(2)
}
