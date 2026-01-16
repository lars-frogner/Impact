//! Drag force and torque computed from aggregating drag on each point on the
//! body.

mod drag_load;
mod equirectangular_map;
pub mod setup;

pub use drag_load::DragLoad;
use impact_alloc::arena::ArenaPool;

use crate::{
    force::ForceGeneratorRegistry,
    medium::UniformMedium,
    quantities::{Direction, DirectionC, Position},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use anyhow::{Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use drag_load::AveragingDragLoad;
use equirectangular_map::EquirectangularMap;
use impact_containers::{HashMap, hash_map::Entry};
use impact_math::{
    angle::{Angle, Radians},
    consts::f32::PI,
    point::Point3C,
    stringhash32_newtype,
};
use roc_integration::roc;
use std::path::{Path, PathBuf};

/// Manages all [`DetailedDragForceGenerator`]s.
#[derive(Debug)]
pub struct DetailedDragForceRegistry {
    drag_load_map_repository: DragLoadMapRepository,
    generators: ForceGeneratorRegistry<DetailedDragForceGeneratorID, DetailedDragForceGenerator>,
}

define_component_type! {
    /// Identifier for a [`DetailedDragForceGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DetailedDragForceGeneratorID(u64);
}

/// Generator for a shape-dependent drag force on a dynamic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct DetailedDragForceGenerator {
    /// The dynamic rigid body experiencing the drag.
    pub body: DynamicRigidBodyID,
    /// The drag force on the body.
    pub force: DetailedDragForce,
    padding: f32,
}

/// A shape-dependent drag force on a dynamic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct DetailedDragForce {
    /// The drag coefficient of the body.
    pub drag_coefficient: f32,
    /// The ID of the [`DragLoadMap`] encoding the shape-dependence of the drag
    /// force.
    pub drag_load_map: DragLoadMapID,
    /// The scale of the body relative to the mesh the drag load map was
    /// computed from.
    pub scaling: f32,
}

stringhash32_newtype!(
    /// Identifier for a [`DragLoadMap`].
    /// Wraps a [`StringHash32`](impact_math::StringHash32).
    #[roc(parents = "Physics")]
    [pub] DragLoadMapID
);

/// A map containing a [`DragLoad`] for each direction of motion relative to the
/// medium. The directions are discretized onto a 2D grid using an
/// equirectangular projection (meaning the grid coordinates are the spherical
/// azimuthal angle phi and polar angle theta).
pub type DragLoadMap = EquirectangularMap<DragLoad>;

/// Repository where [`DragLoadMap`]s are stored under a unique
/// [`DragLoadMapID`].
#[derive(Debug)]
pub struct DragLoadMapRepository {
    config: DragLoadMapConfig,
    drag_load_maps: HashMap<DragLoadMapID, DragLoadMap>,
}

/// Configuration parameters for the generation of drag load maps.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
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
    pub smoothness: f32,
    /// Whether to store newly generated maps as files on disk.
    pub save_generated_maps: bool,
    /// Whether to overwrite any existing map file with the same name when
    /// storing a generated map to file.
    pub overwrite_existing_map_files: bool,
    /// Whether to check whether a file with the required map is available
    /// before generating the map.
    pub use_saved_maps: bool,
    /// The directory generated maps should be saved to and loaded from.
    pub directory: PathBuf,
}

impl DetailedDragForceRegistry {
    pub fn new(config: DragLoadMapConfig) -> Result<Self> {
        Ok(Self {
            drag_load_map_repository: DragLoadMapRepository::new(config)?,
            generators: ForceGeneratorRegistry::new(),
        })
    }

    pub fn drag_load_map_repository_mut(&mut self) -> &mut DragLoadMapRepository {
        &mut self.drag_load_map_repository
    }

    pub fn generators_mut(
        &mut self,
    ) -> &mut ForceGeneratorRegistry<DetailedDragForceGeneratorID, DetailedDragForceGenerator> {
        &mut self.generators
    }

    /// Applies the drag forces to the appropriate dynamic rigid bodies.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, medium: &UniformMedium) {
        for generator in self.generators.generators() {
            generator.apply(rigid_body_manager, medium, &self.drag_load_map_repository);
        }
    }

    /// Removes all stored drag load generators and maps.
    pub fn clear(&mut self) {
        self.generators.clear();
        self.drag_load_map_repository.clear();
    }
}

impl From<u64> for DetailedDragForceGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl DetailedDragForceGenerator {
    pub fn new(body: DynamicRigidBodyID, force: DetailedDragForce) -> Self {
        Self {
            body,
            force,
            padding: 0.0,
        }
    }

    /// Applies the drag force to the appropriate dynamic rigid body.
    pub fn apply(
        &self,
        rigid_body_manager: &mut RigidBodyManager,
        medium: &UniformMedium,
        drag_load_map_repository: &DragLoadMapRepository,
    ) {
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(self.body) else {
            return;
        };
        self.force
            .apply(medium, drag_load_map_repository, rigid_body);
    }
}

impl DetailedDragForce {
    /// Applies the drag force to the given dynamic rigid body.
    pub fn apply(
        &self,
        medium: &UniformMedium,
        drag_load_map_repository: &DragLoadMapRepository,
        rigid_body: &mut DynamicRigidBody,
    ) {
        let body_orientation = rigid_body.orientation().aligned();
        let body_velocity = rigid_body.compute_velocity();
        let medium_velocity = medium.velocity.aligned();

        let velocity_relative_to_medium = body_velocity - medium_velocity;
        let squared_body_speed_relative_to_medium = velocity_relative_to_medium.norm_squared();

        if squared_body_speed_relative_to_medium > 0.0 {
            let body_space_velocity_relative_to_medium =
                rigid_body.transform_vector_from_world_to_body_space(&velocity_relative_to_medium);

            let body_space_direction_of_motion_relative_to_medium = Direction::unchecked_from(
                body_space_velocity_relative_to_medium
                    / f32::sqrt(squared_body_speed_relative_to_medium),
            );

            let phi = compute_phi(&body_space_direction_of_motion_relative_to_medium);
            let theta = compute_theta(&body_space_direction_of_motion_relative_to_medium);

            let drag_load_map = drag_load_map_repository.drag_load_map(self.drag_load_map);

            let drag_load = drag_load_map.value(phi, theta);

            let (force, torque) = drag_load.compute_world_space_drag_force_and_torque(
                self.scaling,
                medium.mass_density,
                self.drag_coefficient,
                &body_orientation,
                squared_body_speed_relative_to_medium,
            );

            rigid_body.apply_force_at_center_of_mass(&force);
            rigid_body.apply_torque(&torque);
        }
    }
}

impl DragLoadMapRepository {
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

    /// Returns a reference to the [`DragLoadMap`] with the given ID.
    ///
    /// # Panics
    /// If no map with the given ID is present.
    pub fn drag_load_map(&self, id: DragLoadMapID) -> &DragLoadMap {
        self.get_drag_load_map(id)
            .expect("Tried to obtain missing drag load map")
    }

    /// Returns a reference to the [`DragLoadMap`] with the given ID, or
    /// [`None`] if the map is not present.
    pub fn get_drag_load_map(&self, id: DragLoadMapID) -> Option<&DragLoadMap> {
        self.drag_load_maps.get(&id)
    }

    /// Whether a drag load map with the given ID is present.
    pub fn has_drag_load_map(&self, id: DragLoadMapID) -> bool {
        self.drag_load_maps.contains_key(&id)
    }

    /// Includes the given drag load map in the repository under the given ID.
    ///
    /// # Errors
    /// Returns an error if a map with the given ID already exists. The
    /// repository will remain unchanged.
    pub fn add_drag_load_map(&mut self, id: DragLoadMapID, map: DragLoadMap) -> Result<()> {
        match self.drag_load_maps.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(map);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Drag load map {} already present in repository",
                id
            )),
        }
    }

    /// Includes the given drag load map in the repository under the given ID,
    /// unless a map with the same ID is already present.
    pub fn add_drag_load_map_unless_present(&mut self, id: DragLoadMapID, map: DragLoadMap) {
        let _ = self.add_drag_load_map(id, map);
    }

    /// Removes all stored drag load maps.
    pub fn clear(&mut self) {
        self.drag_load_maps.clear();
    }
}

impl DragLoadMapConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.directory = root_path.join(&self.directory);
    }

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
            directory: PathBuf::from("resources/drag_load_maps"),
        }
    }
}

impl DragLoadMap {
    /// Computes a drag load map for the mesh with the given triangles and
    /// center of mass, using the given number of direction samples and map
    /// resoulution and smoothness.
    ///
    /// # Panics
    /// If the given number of direction samples or theta coordinates is zero.
    pub fn compute_from_mesh<'a>(
        triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
        center_of_mass: &Position,
        n_direction_samples: usize,
        n_theta_coords: usize,
        smoothness: f32,
    ) -> Self {
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

        let arena = ArenaPool::get_arena();

        let drag_loads =
            drag_load::compute_aggregate_drag_loads_for_uniformly_distributed_directions(
                &arena,
                triangle_vertex_positions,
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

fn generate_map_from_drag_loads(
    drag_loads: &[(DirectionC, DragLoad)],
    n_theta_coords: usize,
    angular_interpolation_distance: Radians,
) -> DragLoadMap {
    let angular_interpolation_distance = angular_interpolation_distance.radians();
    assert!(angular_interpolation_distance > 0.0);

    let mut averaging_map = EquirectangularMap::<AveragingDragLoad>::empty(n_theta_coords);

    for (direction, load) in drag_loads {
        let direction = direction.aligned();

        let direction_phi = compute_phi(&direction);
        let direction_theta = compute_theta(&direction);

        // Increase the interpolation distance when we are closer to the poles
        // to account for the disproportionate number of grid cells that must be
        // filled near the poles in the equirectangular map relative to the
        // local (uniform) density of samples. We increase the distance by a
        // maximum factor of four to prevent the samples near the poles from
        // becoming too influential.
        let scaled_angular_interpolation_distance =
            angular_interpolation_distance / (1.0 - 0.75 * f32::abs(direction.z()));

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

fn compute_phi(direction: &Direction) -> Radians {
    Radians(f32::atan2(direction.y(), direction.x()))
}

fn compute_theta(direction: &Direction) -> Radians {
    Radians(f32::acos(direction.z()))
}

fn compute_angular_interpolation_distance_from_smoothness(
    smoothness: f32,
    n_direction_samples: usize,
) -> Radians {
    // For a smoothness of one, the angular areas covered by the quadratic
    // angular regions surrounding the direction samples add exactly up to the
    // total angular area of the sphere
    Radians(smoothness * f32::sqrt(4.0 * PI / (n_direction_samples as f32)))
}

fn compute_weight(normalized_angular_distance: f32) -> f32 {
    // Use a quartic weighting function with finite support (it reaches zero
    // when normalized angular distance reaches unity)
    f32::max(0.0, 1.0 - normalized_angular_distance.powi(2)).powi(2)
}
