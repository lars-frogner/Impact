//! Drag force and torque computed from aggregating drag on each point on the
//! body.

mod drag_load;
mod equirectangular_map;

use crate::{
    geometry::{Angle, Radians, TriangleMesh},
    num::Float,
    physics::{
        fph, Direction, Position, RigidBodyComp, SpatialConfigurationComp, Static, UniformMedium,
        VelocityComp,
    },
    rendering::fre,
    scene::{MeshComp, MeshID, MeshRepository, ScalingComp},
};
use anyhow::{anyhow, bail, Result};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{
    archetype::ArchetypeComponentStorage, query, setup, world::World as ECSWorld, Component,
};
use simba::scalar::SubsetOf;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::RwLock,
};

pub use drag_load::DragLoad;

use drag_load::AveragingDragLoad;
use equirectangular_map::EquirectangularMap;

/// A map containing a [`DragLoad`] for each direction of motion relative to the
/// medium. The directions are discretized onto a 2D grid using an
/// equirectangular projection (meaning the grid coordinates are the spherical
/// azimuthal angle phi and polar angle theta).
pub type DragLoadMap<F> = EquirectangularMap<DragLoad<F>>;

/// [`Component`](impact_ecs::component::Component) for entities that should be
/// affected by a drag force and torque computed from aggregating drag on each
/// point on the body.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DetailedDragComp {
    /// The drag coefficient of the body.
    pub drag_coefficient: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// associated [`DragLoadMap`] in the [`DragLoadMapRepository`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DragLoadMapComp {
    /// The ID of the mesh from which the drag load map was computed.
    pub mesh_id: MeshID,
}

/// Repository where [`DragLoadMap`]s are stored under a unique [`MeshID`].
#[derive(Debug)]
pub struct DragLoadMapRepository<F: Float> {
    config: DragLoadMapConfig,
    drag_load_maps: HashMap<MeshID, DragLoadMap<F>>,
}

/// Configuration parameters for the generation of drag load maps.
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

impl DetailedDragComp {
    /// Creates a new component for detailed drag with the given drag
    /// coefficient.
    pub fn new(drag_coefficient: fph) -> Self {
        Self { drag_coefficient }
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for obtaining an associated drag load map, and if so, loads or generates
    /// the map and adds it to the drag load map repository if not present, then
    /// adds the appropriate drag load map component to the entity.
    pub fn add_drag_load_map_component_for_entity(
        mesh_repository: &RwLock<MeshRepository<fre>>,
        drag_load_map_repository: &RwLock<DragLoadMapRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        fn generate_map(
            mesh_repository: &RwLock<MeshRepository<fre>>,
            config: &DragLoadMapConfig,
            mesh_id: MeshID,
            rigid_body: &RigidBodyComp,
            scaling: Option<&ScalingComp>,
        ) -> DragLoadMap<fre> {
            let mut center_of_mass = rigid_body.0.inertial_properties().center_of_mass().clone();

            // Unscale the center of mass from the rigid body
            // inertial properties to make it correct for the mesh
            // (which is unscaled)
            if let Some(scaling) = scaling {
                center_of_mass /= scaling.0.into();
            }

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
                    &center_of_mass,
                    config.n_direction_samples,
                    config.n_theta_coords,
                    config.smoothness,
                )
            });

            map
        }

        fn generate_map_path(mesh_id: MeshID) -> PathBuf {
            // Ensure there are no path delimiters
            let sanitized_mesh_name = format!("{}", mesh_id).replace("/", "_").replace("\\", "_");
            PathBuf::from(format!("assets/drag_load_maps/{}.mpk", sanitized_mesh_name))
        }

        setup!(
            components,
            |mesh: &MeshComp,
             rigid_body: &RigidBodyComp,
             scaling: Option<&ScalingComp>|
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
                            generate_map(mesh_repository, config, mesh_id, rigid_body, scaling)
                        })
                    } else {
                        generate_map(mesh_repository, config, mesh_id, rigid_body, scaling)
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
                DragLoadMapComp { mesh_id }
            },
            [DetailedDragComp]
        );
    }
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
            drag_load_maps: HashMap::new(),
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
        self.drag_load_maps
            .get(&mesh_id)
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

/// Applies the drag force and torque calculated from precomputed detailed
/// [`DragLoad`]s to all applicable rigid bodies.
pub fn apply_detailed_drag(
    ecs_world: &ECSWorld,
    drag_load_map_repository: &DragLoadMapRepository<fre>,
    medium: &UniformMedium,
) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         spatial: &SpatialConfigurationComp,
         velocity: &VelocityComp,
         detailed_drag: &DetailedDragComp,
         drag_load_map_comp: &DragLoadMapComp| {
            apply_detailed_drag_for_entity(
                drag_load_map_repository,
                medium,
                rigid_body,
                spatial,
                velocity,
                detailed_drag,
                drag_load_map_comp,
                1.0,
            )
        },
        ![Static, ScalingComp]
    );
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         spatial: &SpatialConfigurationComp,
         velocity: &VelocityComp,
         detailed_drag: &DetailedDragComp,
         drag_load_map_comp: &DragLoadMapComp,
         scaling: &ScalingComp| {
            apply_detailed_drag_for_entity(
                drag_load_map_repository,
                medium,
                rigid_body,
                spatial,
                velocity,
                detailed_drag,
                drag_load_map_comp,
                scaling.0.into(),
            )
        },
        ![Static]
    );
}

fn apply_detailed_drag_for_entity(
    drag_load_map_repository: &DragLoadMapRepository<fre>,
    medium: &UniformMedium,
    rigid_body: &mut RigidBodyComp,
    spatial: &SpatialConfigurationComp,
    velocity: &VelocityComp,
    detailed_drag: &DetailedDragComp,
    drag_load_map_comp: &DragLoadMapComp,
    mesh_scaling: fph,
) {
    let velocity_relative_to_medium = velocity.0 - medium.velocity;
    let squared_body_speed_relative_to_medium = velocity_relative_to_medium.norm_squared();

    if squared_body_speed_relative_to_medium > 0.0 {
        let body_space_velocity_relative_to_medium = spatial
            .orientation
            .inverse_transform_vector(&velocity_relative_to_medium);

        let body_space_direction_of_motion_relative_to_medium = Direction::new_unchecked(
            body_space_velocity_relative_to_medium
                / fph::sqrt(squared_body_speed_relative_to_medium),
        );

        let phi = compute_phi(&body_space_direction_of_motion_relative_to_medium);
        let theta = compute_theta(&body_space_direction_of_motion_relative_to_medium);

        let drag_load_map = drag_load_map_repository.drag_load_map(drag_load_map_comp.mesh_id);

        let drag_load = drag_load_map.value(phi, theta);

        let (force, torque) = drag_load.compute_world_space_drag_force_and_torque(
            mesh_scaling,
            medium.mass_density,
            detailed_drag.drag_coefficient,
            &spatial.orientation,
            squared_body_speed_relative_to_medium,
        );

        rigid_body.0.apply_force_at_center_of_mass(&force);
        rigid_body.0.apply_torque(&torque);
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
