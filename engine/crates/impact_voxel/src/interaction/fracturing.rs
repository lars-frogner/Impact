//! Voxel object fracturing.

use crate::{
    VoxelObjectBufferPool, VoxelObjectID, VoxelObjectManager,
    collidable::{Collidable, CollisionWorld},
    interaction::{
        self, Anchors, ExtractedComponents, RemovedMassFate, VoxelObjectInteractionContext,
        VoxelRemovalOutcome,
    },
    mesh::{MeshedVoxelObject, MeshedVoxelObjectBuffers},
    object::{
        VoxelObject, VoxelObjectBuffers, extraction::ExtractionResult,
        inertia::VoxelObjectInertialPropertyManager,
    },
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{
    AVec, Allocator,
    arena::{ArenaPool, PoolArena},
    avec,
};
use impact_containers::{HashMap, hash_map::Entry};
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_id::{EntityID, EntityIDManager};
use impact_math::{
    consts::f32,
    point::{Point3, Point3C},
    quaternion::UnitQuaternion,
    random::Rng,
    transform::Isometry3,
    vector::{UnitVector3, Vector3, Vector3C},
};
use impact_physics::{
    anchor::AnchorManager,
    collision::CollidableID,
    constraint::{ConstrainedBodyManager, ConstraintManager},
    quantities::{AngularVelocity, Velocity},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID, RigidBodyManager},
};
use impact_tesselation::{
    delaunay::{DelaunayTetrahedralization, VertexIdx},
    voronoi::VoronoiPolyhedron,
};
use impact_thread::{
    channel,
    pool::{DynamicTask, DynamicThreadPool},
};
use roc_integration::roc;
use std::time::Instant;

pub trait VoxelObjectFracturingContext {
    /// Returns the fracturing properties of the given entity if they exist.
    fn get_fracturing_properties_for_entity(
        &self,
        entity_id: EntityID,
    ) -> Option<FracturingProperties>;
}

define_component_type! {
    /// Properties governing how an object fractures on impact.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FracturingProperties {
        /// The minimum collisional force (assumed constant during the
        /// collision) required for an impact to cause fracturing.
        pub fracturing_force: f32,
        /// The collisional force per object surface area (approximated by the
        /// square of the object extent) required for the object to shatter (the
        /// fracturing radius reaching the characteristic size of the object).
        pub shattering_pressure: f32,
        /// The characteristic length scale of generated fragments, relative to
        /// the extent of the object.
        pub fragment_scale: f32,
        /// The target minimum extent of generated fragments. Will be converted
        /// to world units based on the object size such that the minimum
        /// fragment size in world space scale increases weakly with the object
        /// size.
        pub min_fragment_extent: f32,
        /// The target maximum extent of generated fragments, relative to the
        /// extent of the object.
        pub max_fragment_extent: f32,
    }
}

/// Manages voxel object fracturing processes and state.
#[derive(Debug)]
pub struct VoxelObjectFracturingManager {
    staged_processes: HashMap<VoxelObjectID, StagedFracturingProcess>,
    fracture_region_pool: Vec<DelaunayTetrahedralization>,
    fracture_region_buffer_pool: Vec<VoxelObjectBuffers>,
    process_pool: Vec<FracturingProcess>,
    config: VoxelFracturingConfig,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct VoxelFracturingConfig {
    pub impact: VoxelImpactFracturingConfig,
    /// Fragments whose mass is a smaller fraction of the original object's mass
    /// than this will be discarded.
    pub min_relative_fragment_mass: f32,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct VoxelImpactFracturingConfig {
    /// Number of polar grid cells in the hemispherical boundary point sampling
    /// grid. Larger grid sizes makes the fracture region boundary smoother and
    /// more hemispherical.
    pub boundary_polar_grid_size: usize,
    /// Number of azimuthal grid cells in the hemispherical boundary point
    /// sampling grid. Larger grid sizes makes the fracture region boundary
    /// smoother and more hemispherical.
    pub boundary_azimuthal_grid_size: usize,
    /// Max randomized angular change relative the grid cell size when sampling
    /// directions for boundary points. `0.0` puts the sample at the center,
    /// `1.0` samples anywhere within the grid cell.
    pub boundary_angular_jitter: f32,
    /// Max randomized change relative to the boundary region extent when
    /// sampling radial distances for boundary points. `0.0` puts the sample at
    /// the analytical radius, `1.0` samples in a radius-wide band centered on
    /// the radius.
    pub boundary_radial_jitter: f32,
    /// The maximum number of fragments to generate for an impact-fragmented
    /// object.
    pub max_fragment_count: u64,
    /// How steeply the stress from a fracturing impact decreases with distance
    /// from the impact point.
    pub radial_falloff_power: f32,
    /// How steeply the stress from a fracturing impact decreases with angle
    /// away from the impact force direction.
    pub angular_falloff_power: f32,
    /// Number of cells along the radial dimension of the grid used for fracture
    /// point generation.
    pub radial_grid_size: usize,
    /// Number of cells along the angular dimension of the grid used for
    /// fracture point generation.
    pub angular_grid_size: usize,
    /// The number of times a generated fragment position can be rejected due to
    /// being too close to another fragment or outside the object before sample
    /// generation is stopped. The total rejection budged will be the specified
    /// value times the target number of fragments to generate.
    pub max_position_rejections_per_sample: u64,
    /// The seed to use for fragment generation.
    pub seed: u64,
}

#[derive(Debug)]
struct StagedFracturingProcess {
    fracture_region_tetras: Option<DelaunayTetrahedralization>,
    process: FracturingProcess,
}

#[derive(Debug)]
struct ActiveFracturingProcess {
    fracture_region_object: Option<FractureRegionObject>,
    process: FracturingProcess,
}

#[derive(Debug)]
struct FractureRegionObject {
    voxel_object: VoxelObject,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
    rigid_body: DynamicRigidBody,
    anchors: Anchors,
    origin_offset_in_parent: [usize; 3],
    original_object_mass: f32,
}

#[derive(Debug)]
struct FracturingProcess {
    state: FracturingProcessState,
    tetrahedralization: DelaunayTetrahedralization,
    vertex_indices: Vec<VertexIdx>,
    fragments: Vec<Fragment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FracturingProcessState {
    Idle,
    Initialized,
    Cancelled,
}

#[derive(Debug)]
struct Fragment {
    meshed_voxel_object: MeshedVoxelObject,
    origin_offset_in_parent: [usize; 3],
    inertial_property_manager: VoxelObjectInertialPropertyManager,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum FractureObjectGenerationResult {
    Generated(Fragment),
    NotGenerated(MeshedVoxelObjectBuffers),
}

#[derive(Clone, Debug)]
pub enum FracturePointGenerator {
    RandomizedGrid(RandomizedGridFracturePointGenerator),
}

#[derive(Clone, Debug)]
pub struct RandomizedGridFracturePointGenerator {
    points_per_dim: usize,
}

#[derive(Clone, Debug)]
struct FractureForce {
    position: Point3,
    direction: UnitVector3,
    magnitude: f32,
}

impl VoxelObjectFracturingManager {
    /// Creates a new empty fracturing manager with the given configuration.
    pub fn new(config: VoxelFracturingConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            staged_processes: HashMap::default(),
            fracture_region_pool: Vec::new(),
            fracture_region_buffer_pool: Vec::new(),
            process_pool: Vec::new(),
            config,
        })
    }

    /// Stages a fracturing process for the given voxel object. If boundary
    /// points for the fracture region are specified, their convex hull will be
    /// used as the boundary of the fractured region. If not specified, the
    /// fracture region will be the whole object. The fragments within the
    /// fracture region will be the Voronoi cells centered on the given fracture
    /// points.
    ///
    /// Note that both the fracture region boundary points and fracture points
    /// should be specified in the normalized space of the voxel object (where
    /// distance is in units of voxels).
    ///
    /// Call [`Self::execute_fracturing_processes`] to execute the staged
    /// fracturing processes.
    ///
    /// # Returns
    /// `true` if the fracture region and points result in a staged fracturing
    /// process.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The voxel object does not exist.
    /// - Another fracturing process is currently staged for the same object.
    /// - The Delaunay tetrahedralization for the convex hull of the fracture
    ///   region boundary points can't be computed.
    pub fn stage_fracturing_process_for_object(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_id: VoxelObjectID,
        fracture_region_boundary_points: Option<&[Point3C]>,
        fracture_points: &[Point3C],
    ) -> Result<bool> {
        if !voxel_object_manager.has_voxel_object(voxel_object_id) {
            bail!("Tried to stage fracturing process for missing voxel object {voxel_object_id}");
        }

        let Entry::Vacant(process_entry) = self.staged_processes.entry(voxel_object_id) else {
            bail!(
                "Tried to stage multiple fracturing processes for voxel object {voxel_object_id}"
            );
        };

        log::debug!("Staging fracturing process for voxel object {voxel_object_id}");

        let fracture_region_tetras = if let Some(fracture_region_boundary_points) =
            fracture_region_boundary_points
        {
            let mut fracture_region_tetras = self.fracture_region_pool.pop().unwrap_or_default();

            let result = fracture_region_tetras
                .reconstruct(fracture_region_boundary_points)
                .with_context(|| {
                    format!("Failed to construct fracture region polyhedron for voxel object {voxel_object_id}")
                });

            // Reclaim resources before returning (with or without an error)
            if result.is_err() || fracture_region_tetras.n_tetrahedra() == 0 {
                self.fracture_region_pool.push(fracture_region_tetras);
                return result.map(|_| false);
            }

            Some(fracture_region_tetras)
        } else {
            None
        };

        let mut process = self.process_pool.pop().unwrap_or_default();

        let result = process.initialize(fracture_points).with_context(|| {
            format!("Failed to tetrahedralize fracture points for voxel object {voxel_object_id}")
        });

        if result.is_err() || !process.is_initialized() {
            // Reclaim resources before returning (with or without an error)
            if let Some(fracture_region_tetras) = fracture_region_tetras {
                self.fracture_region_pool.push(fracture_region_tetras);
            }
            self.process_pool.push(process);
            return result.map(|_| false);
        }

        let staged_process = StagedFracturingProcess {
            fracture_region_tetras,
            process,
        };

        process_entry.insert(staged_process);

        Ok(true)
    }

    /// Whether a fracturing process is currently staged for the given voxel
    /// object.
    pub fn object_has_staged_fracturing_process(&self, voxel_object_id: VoxelObjectID) -> bool {
        self.staged_processes.contains_key(&voxel_object_id)
    }

    /// Executes all fracturing processes staged with
    /// [`Self::stage_fracturing_process_for_object`].
    pub fn execute_fracturing_processes<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
    ) where
        C: VoxelObjectInteractionContext,
    {
        self.execute_fracturing_processes_with_closure(
            context,
            entity_id_manager,
            voxel_type_registry,
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            |voxel_object_buffer_pool, process, voxel_object_id, voxel_object| {
                process.execute(
                    voxel_type_registry,
                    voxel_object_buffer_pool,
                    voxel_object_id,
                    voxel_object,
                );
            },
        );
    }

    /// Executes all fracturing processes staged with
    /// [`Self::stage_fracturing_process_for_object`] in parallel.
    pub fn execute_fracturing_processes_in_parallel<C>(
        &mut self,
        thread_pool: &DynamicThreadPool,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
    ) where
        C: VoxelObjectInteractionContext,
    {
        self.execute_fracturing_processes_with_closure(
            context,
            entity_id_manager,
            voxel_type_registry,
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            |voxel_object_buffer_pool, process, voxel_object_id, voxel_object| {
                process.execute_in_parallel(
                    thread_pool,
                    voxel_type_registry,
                    voxel_object_buffer_pool,
                    voxel_object_id,
                    voxel_object,
                );
            },
        );
    }

    fn execute_fracturing_processes_with_closure<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        execute_process: impl Fn(
            &mut VoxelObjectBufferPool,
            &mut FracturingProcess,
            VoxelObjectID,
            &VoxelObject,
        ),
    ) where
        C: VoxelObjectInteractionContext,
    {
        for (voxel_object_id, staged_process) in self.staged_processes.drain() {
            let Some(active_process) = staged_process.initiate(
                context,
                entity_id_manager,
                voxel_type_registry,
                voxel_object_manager,
                voxel_object_buffer_pool,
                rigid_body_manager,
                anchor_manager,
                &mut self.fracture_region_pool,
                &mut self.fracture_region_buffer_pool,
                &mut self.process_pool,
                voxel_object_id,
            ) else {
                continue;
            };

            let ActiveFracturingProcess {
                fracture_region_object,
                mut process,
            } = active_process;

            assert!(process.is_initialized());

            if let Some(voxel_object) = fracture_region_object
                .as_ref()
                .map(|object| &object.voxel_object)
                .or_else(|| {
                    Self::get_voxel_object_for_execution(
                        voxel_object_manager,
                        rigid_body_manager,
                        voxel_object_id,
                    )
                })
            {
                execute_process(
                    voxel_object_buffer_pool,
                    &mut process,
                    voxel_object_id,
                    voxel_object,
                );
            } else {
                // The voxel object no longer exist, so we cancel the process
                process.cancel();
            }

            if process.has_processed() {
                if let Some(fracture_region_object) = fracture_region_object {
                    process.complete_for_fracture_region_object(
                        &self.config,
                        context,
                        entity_id_manager,
                        voxel_object_manager,
                        voxel_object_buffer_pool,
                        rigid_body_manager,
                        anchor_manager,
                        &mut self.fracture_region_buffer_pool,
                        fracture_region_object,
                        voxel_object_id.as_entity_id(),
                    );
                } else {
                    process.complete_for_existing_object(
                        &self.config,
                        context,
                        entity_id_manager,
                        voxel_object_manager,
                        voxel_object_buffer_pool,
                        rigid_body_manager,
                        anchor_manager,
                        voxel_object_id,
                    );
                }
            } else {
                assert!(process.is_cancelled());
            }

            // Reset and return to pool (whether it was successfully completed
            // or cancelled before or during completion)
            process.reset(voxel_object_buffer_pool);
            self.process_pool.push(process);
        }
    }

    /// Identifies collisions involving fragmentable objects with impulses
    /// strong enough to cause fragmentation and stages fracturing with
    /// appropriately generated fracture points for those objects.
    pub fn handle_fracturing_impacts<C>(
        &mut self,
        context: &C,
        voxel_object_manager: &VoxelObjectManager,
        rigid_body_manager: &RigidBodyManager,
        constraint_manager: &mut ConstraintManager,
        collision_world: &CollisionWorld,
        time_step_duration: f32,
    ) where
        C: VoxelObjectFracturingContext,
    {
        let Some(collisions) = collision_world.cached_collisions() else {
            return;
        };

        let arena = ArenaPool::get_arena();

        // We will cache rigid body state as `ConstrainedBody`s for efficient
        // impulse computation
        let mut body_manager = ConstrainedBodyManager::new_in(&arena);

        // Reusable buffer for generated fracture region boundary points and
        // fracture points
        let mut fracture_region_boundary_points = AVec::new_in(&arena);
        let mut fracture_points = AVec::new_in(&arena);

        let collidable_voxel_object = |id: CollidableID| {
            let object_id = VoxelObjectID::from_entity_id(id.as_entity_id());
            voxel_object_manager
                .get_voxel_object(object_id)
                .map(|object| (object_id, object.object()))
        };

        let get_fracturing_properties = |object: Option<(VoxelObjectID, &VoxelObject)>| {
            let (object_id, _) = object?;
            context.get_fracturing_properties_for_entity(object_id.as_entity_id())
        };

        let inverse_time_step_duration = time_step_duration.recip();

        let mut rng = Rng::with_seed(self.config.impact.seed);

        for collision in collisions {
            let voxel_object_a_with_id = collidable_voxel_object(collision.collidable_a_id);
            let voxel_object_b_with_id = collidable_voxel_object(collision.collidable_b_id);

            let properties_a = get_fracturing_properties(voxel_object_a_with_id);
            let properties_b = get_fracturing_properties(voxel_object_b_with_id);

            if properties_a.is_none() && properties_b.is_none() {
                // None of the objects can be fractured
                continue;
            }

            let Some(descriptor_a) =
                collision_world.get_collidable_descriptor(collision.collidable_a_id)
            else {
                continue;
            };
            let Some(descriptor_b) =
                collision_world.get_collidable_descriptor(collision.collidable_b_id)
            else {
                continue;
            };
            let Some(collidable_a) = collision_world.get_collidable_with_descriptor(descriptor_a)
            else {
                continue;
            };
            let Some(collidable_b) = collision_world.get_collidable_with_descriptor(descriptor_b)
            else {
                continue;
            };

            // Cache the rigid body states if not already cached and get the
            // cached states
            let Some((body_a_idx, body_b_idx)) = body_manager.add_body_pair(
                rigid_body_manager,
                descriptor_a.rigid_body_id(),
                descriptor_b.rigid_body_id(),
            ) else {
                continue;
            };
            let body_a = body_manager.body(body_a_idx);
            let body_b = body_manager.body(body_b_idx);

            // We will compute the mutual normal force at each contact and pick
            // the contact with the maximum force
            let mut max_contact_force = Vector3::zeros();
            let mut max_contact_position = Point3::origin();
            let mut max_impulse = f32::NEG_INFINITY;

            for contact in collision.contact_manifold.contacts() {
                let contact = &contact.contact;
                let geometry = &contact.geometry;

                let impulse = contact.compute_normal_impulse(body_a, body_b);

                if impulse > max_impulse {
                    // The impulse is the momentum transfer over the full time
                    // step, so the force (assumed constant during the time
                    // step) is the impulse divided by the time step
                    let force = (impulse * inverse_time_step_duration) * geometry.surface_normal;
                    max_contact_force = force;
                    max_contact_position = geometry.position;
                    max_impulse = impulse;
                }
            }

            let force_magnitude = max_impulse * inverse_time_step_duration;

            let mut should_skip_collision_response = false;

            for (voxel_object_with_id, other_collidable_id, collidable, properties, force) in [
                (
                    voxel_object_a_with_id,
                    collision.collidable_b_id,
                    collidable_a,
                    properties_a,
                    max_contact_force,
                ),
                (
                    voxel_object_b_with_id,
                    collision.collidable_a_id,
                    collidable_b,
                    properties_b,
                    -max_contact_force,
                ),
            ] {
                let Some(properties) = properties else {
                    // The object can not be fractured
                    continue;
                };
                if force_magnitude <= properties.fracturing_force {
                    // The force is insufficient to cause fracturing
                    continue;
                }
                let Some((voxel_object_id, voxel_object)) = voxel_object_with_id else {
                    // The colliding body is not a voxel object
                    continue;
                };

                if self.object_has_staged_fracturing_process(voxel_object_id) {
                    // There is already a fracturing process staged from another
                    // collision, so we can't process the fracturing due to this
                    // collision this frame. We therefore just skip the
                    // collision response for this frame. If this collision is
                    // still relevant next frame, we will likely re-detect it
                    // and stage a fracturing process then.
                    should_skip_collision_response = true;
                    log::debug!(
                        "Disregarding fracturing collision of voxel object {voxel_object_id} \
                         with entity {other_collidable_id} due to already staged fracturing process",
                    );
                    continue;
                }

                let Collidable::VoxelObject(collidable) = collidable.collidable() else {
                    panic!("Unexpected collidable for voxel object");
                };

                let force_direction = UnitVector3::unchecked_from(force / force_magnitude);

                let world_to_object_transform = collidable.transform_to_object_space().aligned();
                let aabb = voxel_object.compute_aabb();

                fracture_region_boundary_points.clear();
                fracture_points.clear();

                let fracture_force = FractureForce {
                    position: max_contact_position,
                    direction: force_direction,
                    magnitude: force_magnitude,
                };

                generate_impact_fracture_points(
                    &self.config.impact,
                    &arena,
                    &mut fracture_region_boundary_points,
                    &mut fracture_points,
                    &properties,
                    voxel_object.inverse_voxel_extent(),
                    &world_to_object_transform,
                    &aabb,
                    &fracture_force,
                    &mut rng,
                );

                if fracture_region_boundary_points.is_empty() || fracture_points.is_empty() {
                    continue;
                }

                log::debug!(
                    "Fracturing voxel object {voxel_object_id} due to collision with entity {other_collidable_id}: \
                     force magnitude {force_magnitude:.3e} exceeds threshold {threshold:.3e}, \
                     direction = [{dx:.3}, {dy:.3}, {dz:.3}], \
                     {fracture_point_count} fracture point(s) generated",
                    threshold = properties.fracturing_force,
                    dx = force_direction.x(),
                    dy = force_direction.y(),
                    dz = force_direction.z(),
                    fracture_point_count = fracture_points.len(),
                );

                let was_staged = self
                    .stage_fracturing_process_for_object(
                        voxel_object_manager,
                        voxel_object_id,
                        Some(&fracture_region_boundary_points),
                        &fracture_points,
                    )
                    .unwrap();

                should_skip_collision_response |= was_staged;
            }

            if should_skip_collision_response {
                // Prevent the impulses from the fracturing collision from being
                // applied until the fragments are generated
                constraint_manager.skip_collision_for_steps(
                    [
                        collision.collidable_a_id.as_entity_id(),
                        collision.collidable_b_id.as_entity_id(),
                    ],
                    1,
                );
            }
        }
    }

    /// Removes all fracturing state and frees up all allocated memory.
    pub fn reset_and_free(&mut self) {
        self.staged_processes = HashMap::default();
        self.fracture_region_pool = Vec::new();
        self.fracture_region_buffer_pool = Vec::new();
        self.process_pool = Vec::new();
    }

    fn get_voxel_object_for_execution<'a>(
        voxel_object_manager: &'a VoxelObjectManager,
        rigid_body_manager: &RigidBodyManager,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&'a VoxelObject> {
        let entity_id = voxel_object_id.as_entity_id();
        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);

        let Some(meshed_voxel_object) = voxel_object_manager.get_voxel_object(voxel_object_id)
        else {
            log::warn!("Tried to execute fracturing for missing voxel object: {voxel_object_id}");
            return None;
        };
        if !voxel_object_manager.has_physics_context(voxel_object_id)
            || !rigid_body_manager.has_dynamic_rigid_body(rigid_body_id)
        {
            log::warn!(
                "Tried to execute fracturing for voxel object {voxel_object_id} \
                 without physics"
            );
            return None;
        }

        Some(meshed_voxel_object.object())
    }
}

impl VoxelFracturingConfig {
    fn validate(&self) -> Result<()> {
        self.impact.validate()?;

        if self.min_relative_fragment_mass < 0.0 {
            bail!(
                "Minimum relative fragment mass for fracturing must be non-negative: {}",
                self.min_relative_fragment_mass
            );
        }

        Ok(())
    }
}

impl Default for VoxelFracturingConfig {
    fn default() -> Self {
        Self {
            impact: Default::default(),
            min_relative_fragment_mass: 1e-3,
        }
    }
}

impl VoxelImpactFracturingConfig {
    fn validate(&self) -> Result<()> {
        if self.boundary_polar_grid_size < 2 {
            bail!(
                "Boundary polar grid size for impact fragment generation must be at least 2: {}",
                self.boundary_polar_grid_size
            );
        }
        if self.boundary_azimuthal_grid_size < 3 {
            bail!(
                "Boundary azimuthal grid size for impact fragment generation must be at least 3: {}",
                self.boundary_azimuthal_grid_size
            );
        }
        if self.boundary_angular_jitter < 0.0 || self.boundary_angular_jitter > 1.0 {
            bail!(
                "Boundary angular jitter for impact fragment generation must be between 0.0 and 1.0: {}",
                self.boundary_angular_jitter
            );
        }
        if self.boundary_radial_jitter < 0.0 || self.boundary_radial_jitter > 1.0 {
            bail!(
                "Boundary radial jitter for impact fragment generation must be between 0.0 and 1.0: {}",
                self.boundary_radial_jitter
            );
        }
        if self.radial_falloff_power < 0.0 {
            bail!(
                "Radial falloff power for impact fragment generation must be non-negative: {}",
                self.radial_falloff_power
            );
        }
        if self.angular_falloff_power < 0.0 {
            bail!(
                "Angular falloff power for impact fragment generation must be non-negative: {}",
                self.angular_falloff_power
            );
        }
        if self.radial_grid_size < 2 {
            bail!(
                "Radial grid size for impact fragment generation must be at least 2: {}",
                self.radial_grid_size
            );
        }
        if self.angular_grid_size < 2 {
            bail!(
                "Angular grid size for impact fragment generation must be at least 2: {}",
                self.angular_grid_size
            );
        }
        Ok(())
    }
}

impl Default for VoxelImpactFracturingConfig {
    fn default() -> Self {
        Self {
            boundary_polar_grid_size: 3,
            boundary_azimuthal_grid_size: 6,
            boundary_angular_jitter: 0.8,
            boundary_radial_jitter: 0.2,
            max_fragment_count: 512,
            radial_falloff_power: 2.0,
            angular_falloff_power: 0.5,
            radial_grid_size: 128,
            angular_grid_size: 128,
            max_position_rejections_per_sample: 128,
            seed: 0,
        }
    }
}

impl StagedFracturingProcess {
    fn initiate<C>(
        self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        fracture_region_pool: &mut Vec<DelaunayTetrahedralization>,
        fracture_region_buffer_pool: &mut Vec<VoxelObjectBuffers>,
        process_pool: &mut Vec<FracturingProcess>,
        voxel_object_id: VoxelObjectID,
    ) -> Option<ActiveFracturingProcess>
    where
        C: VoxelObjectInteractionContext,
    {
        let Self {
            fracture_region_tetras,
            mut process,
        } = self;

        let Some(fracture_region_tetras) = fracture_region_tetras else {
            return Some(ActiveFracturingProcess {
                fracture_region_object: None,
                process,
            });
        };

        let Some(fracture_region_object) = extract_fracture_region_object(
            context,
            entity_id_manager,
            voxel_type_registry,
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            fracture_region_buffer_pool,
            voxel_object_id,
            &fracture_region_tetras,
        ) else {
            fracture_region_pool.push(fracture_region_tetras);
            process.reset(voxel_object_buffer_pool);
            process_pool.push(process);
            return None;
        };

        fracture_region_pool.push(fracture_region_tetras);

        // The fracture points were defined relative to the original object, but
        // we now need them relative to the fracture region object
        process.offset_tetrahedralization_to_fracture_region_object(
            fracture_region_object.origin_offset_in_parent,
        );

        Some(ActiveFracturingProcess {
            fracture_region_object: Some(fracture_region_object),
            process,
        })
    }
}

impl FracturingProcess {
    fn new() -> Self {
        Self {
            state: FracturingProcessState::Idle,
            tetrahedralization: DelaunayTetrahedralization::new(),
            vertex_indices: Vec::new(),
            fragments: Vec::new(),
        }
    }

    fn is_idle(&self) -> bool {
        self.state == FracturingProcessState::Idle
    }

    fn is_initialized(&self) -> bool {
        self.state == FracturingProcessState::Initialized
    }

    fn is_cancelled(&self) -> bool {
        self.state == FracturingProcessState::Cancelled
    }

    fn has_processed(&self) -> bool {
        self.is_initialized() && self.vertex_indices.is_empty()
    }

    fn cancel(&mut self) {
        self.state = FracturingProcessState::Cancelled;
    }

    fn mark_processed(&mut self) {
        assert!(self.is_initialized());
        self.vertex_indices.clear();
    }

    fn initialize(&mut self, fracture_points: &[Point3C]) -> Result<()> {
        assert!(self.is_idle());
        assert!(self.vertex_indices.is_empty());
        assert!(self.fragments.is_empty());

        self.tetrahedralization.reconstruct(fracture_points)?;

        if self.tetrahedralization.n_tetrahedra() == 0 {
            return Ok(());
        }

        self.vertex_indices
            .extend(self.tetrahedralization.internal_vertex_indices());

        self.state = FracturingProcessState::Initialized;

        Ok(())
    }

    fn offset_tetrahedralization_to_fracture_region_object(
        &mut self,
        origin_offset_in_parent: [usize; 3],
    ) {
        let origin_offset: Vector3C = origin_offset_in_parent.map(|offset| offset as f32).into();
        self.tetrahedralization.displace_vertices(-origin_offset);
    }

    fn execute(
        &mut self,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        voxel_object_id: VoxelObjectID,
        voxel_object: &VoxelObject,
    ) {
        if !self.is_initialized() {
            return;
        }

        let aabb = voxel_object.compute_normalized_chunk_grid_bounds();

        let arena = ArenaPool::get_arena();
        let mut polyhedron = VoronoiPolyhedron::empty_in(&arena);

        let start_time = log::log_enabled!(log::Level::Debug).then(Instant::now);

        for &dual_vertex_idx in &self.vertex_indices {
            let buffers = voxel_object_buffer_pool.take_or_create_buffers();

            let result = Self::generate_fragment(
                voxel_type_registry,
                &self.tetrahedralization,
                voxel_object,
                &aabb,
                dual_vertex_idx,
                buffers,
                &mut polyhedron,
            );

            match result {
                FractureObjectGenerationResult::Generated(fragment) => {
                    self.fragments.push(fragment);
                }
                FractureObjectGenerationResult::NotGenerated(buffers) => {
                    // Store the buffers for reuse
                    voxel_object_buffer_pool.add_buffers(buffers);
                }
            }
        }

        self.mark_processed();

        if let Some(start_time) = start_time {
            log::debug!(
                "Generated {n_generated} fragments in {elapsed_ms:.2} ms \
                 for voxel object {voxel_object_id}",
                n_generated = self.fragments.len(),
                elapsed_ms = 1e3 * start_time.elapsed().as_secs_f64()
            );
        }
    }

    fn execute_in_parallel(
        &mut self,
        thread_pool: &DynamicThreadPool,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        voxel_object_id: VoxelObjectID,
        voxel_object: &VoxelObject,
    ) {
        if !self.is_initialized() {
            return;
        }

        let num_threads = thread_pool.n_workers().get();

        let aabb = voxel_object.compute_normalized_chunk_grid_bounds();

        let start_time = log::log_enabled!(log::Level::Debug).then(Instant::now);

        thread_pool
            .with_scope(|scope| {
                const INPUT_CAPACITY_PER_THREAD: usize = 2;
                const RECEIVE_BATCH_SIZE_PER_THREAD: usize = 1;
                const { assert!(RECEIVE_BATCH_SIZE_PER_THREAD <= INPUT_CAPACITY_PER_THREAD) };

                struct TaskInput {
                    dual_vertex_idx: VertexIdx,
                    buffers: MeshedVoxelObjectBuffers,
                }

                let input_capacity = num_threads * INPUT_CAPACITY_PER_THREAD;

                let (input_sender, input_receiver) = channel::bounded::<TaskInput>(input_capacity);

                let (result_sender, result_receiver) =
                    channel::bounded::<FractureObjectGenerationResult>(num_threads);

                let tetrahedralization = &self.tetrahedralization;
                let aabb = &aabb;

                scope
                    .execute((0..num_threads).map(|_| {
                        let input_receiver = input_receiver.clone();
                        let result_sender = result_sender.clone();

                        DynamicTask::new(move |_| {
                            let arena = ArenaPool::get_arena();
                            let mut polyhedron = VoronoiPolyhedron::empty_in(&arena);

                            while let Ok(input) = input_receiver.recv() {
                                let result = Self::generate_fragment(
                                    voxel_type_registry,
                                    tetrahedralization,
                                    voxel_object,
                                    aabb,
                                    input.dual_vertex_idx,
                                    input.buffers,
                                    &mut polyhedron,
                                );

                                result_sender.send(result).unwrap();
                            }

                            // Channel is empty and disconnected (sender is
                            // dropped), so we are done
                        })
                    }))
                    .unwrap();

                let receive_batch_size = num_threads * RECEIVE_BATCH_SIZE_PER_THREAD;

                let mut in_flight_count = 0;

                'outer: while !self.vertex_indices.is_empty() {
                    // Send task inputs to the workers until the input buffer is
                    // full (or we are out of vertices to process)
                    while !input_sender.is_full() {
                        let Some(dual_vertex_idx) = self.vertex_indices.pop() else {
                            // We have dispatched all vertices, so break out of
                            // the outer loop so we can receive all remaining
                            // results
                            break 'outer;
                        };
                        let buffers = voxel_object_buffer_pool.take_or_create_buffers();

                        let task_input = TaskInput {
                            dual_vertex_idx,
                            buffers,
                        };

                        input_sender.send(task_input).unwrap();
                        in_flight_count += 1;
                    }

                    // We have filled up the input buffer, now we start
                    // receiving until we have received `receive_batch_size`
                    // results. By not receiving all results now, we keep the
                    // input buffers populated so that the workers never sit
                    // idle.
                    let mut received_count = 0;
                    while received_count < receive_batch_size {
                        match result_receiver.recv().unwrap() {
                            FractureObjectGenerationResult::Generated(fragment) => {
                                self.fragments.push(fragment);
                            }
                            FractureObjectGenerationResult::NotGenerated(buffers) => {
                                // Store the buffers for reuse
                                voxel_object_buffer_pool.add_buffers(buffers);
                            }
                        }
                        received_count += 1;
                    }
                    in_flight_count -= received_count;
                }

                // Receive all in-flight results before we exit
                while in_flight_count > 0 {
                    match result_receiver.recv().unwrap() {
                        FractureObjectGenerationResult::Generated(fragment) => {
                            self.fragments.push(fragment);
                        }
                        FractureObjectGenerationResult::NotGenerated(buffers) => {
                            voxel_object_buffer_pool.add_buffers(buffers);
                        }
                    }
                    in_flight_count -= 1;
                }

                // The input sender will be dropped here, disconnecting the
                // input channel and allowing the workers to exit their task
            })
            .unwrap();

        assert!(self.has_processed());

        if let Some(start_time) = start_time {
            log::debug!(
                "Generated {n_generated} fragments in {elapsed_ms:.2} ms \
                 ({num_threads} thread(s)) for voxel object {voxel_object_id}",
                n_generated = self.fragments.len(),
                elapsed_ms = 1e3 * start_time.elapsed().as_secs_f64()
            );
        }
    }

    fn generate_fragment<A: Allocator>(
        voxel_type_registry: &VoxelTypeRegistry,
        tetrahedralization: &DelaunayTetrahedralization,
        voxel_object: &VoxelObject,
        aabb: &AxisAlignedBox,
        dual_vertex_idx: VertexIdx,
        buffers: MeshedVoxelObjectBuffers,
        polyhedron: &mut VoronoiPolyhedron<A>,
    ) -> FractureObjectGenerationResult {
        polyhedron.extract_from_delaunay_tetrahedra(tetrahedralization, dual_vertex_idx);

        let Some(polyhedron_aabb) = polyhedron.compute_bounded_aabb(aabb) else {
            return FractureObjectGenerationResult::NotGenerated(buffers);
        };

        // Shrink the polyhedron slightly to avoid slowing down collision
        // detection with a lot of exactly touching flat surfaces
        polyhedron.shift_face_planes(-0.1);

        let mut poly_inertial_property_manager = VoxelObjectInertialPropertyManager::zeroed();

        let mut inertial_property_copier = poly_inertial_property_manager.begin_computation(
            voxel_object.voxel_extent(),
            voxel_type_registry.mass_densities(),
        );

        let extraction_result = voxel_object.copy_polyhedron_with_property_computer(
            buffers.object_buffers,
            &polyhedron_aabb,
            &polyhedron.face_planes,
            &mut inertial_property_copier,
        );

        match extraction_result {
            ExtractionResult::Extracted(poly_object) => {
                let meshed_poly_object =
                    MeshedVoxelObject::create(buffers.mesh_buffers, poly_object.voxel_object);

                FractureObjectGenerationResult::Generated(Fragment {
                    meshed_voxel_object: meshed_poly_object,
                    origin_offset_in_parent: poly_object.origin_offset_in_parent,
                    inertial_property_manager: poly_inertial_property_manager,
                })
            }
            ExtractionResult::NotExtracted(object_buffers) => {
                // Store the buffers for reuse
                FractureObjectGenerationResult::NotGenerated(MeshedVoxelObjectBuffers {
                    object_buffers,
                    mesh_buffers: buffers.mesh_buffers,
                })
            }
        }
    }

    fn complete_for_fracture_region_object<C>(
        &mut self,
        config: &VoxelFracturingConfig,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        fracture_region_buffer_pool: &mut Vec<VoxelObjectBuffers>,
        fracture_region_object: FractureRegionObject,
        original_entity_id: EntityID,
    ) where
        C: VoxelObjectInteractionContext,
    {
        assert!(self.has_processed());

        // We need the original voxel object entity to still exist so the
        // fragments can inherit its components
        if !context.entity_exists(original_entity_id) {
            log::warn!(
                "Tried to complete fracturing for missing voxel object entity: {original_entity_id}"
            );
            fracture_region_buffer_pool.push(fracture_region_object.voxel_object.into_buffers());
            self.state = FracturingProcessState::Cancelled;
            return;
        }

        let original_local_center_of_mass = fracture_region_object
            .inertial_property_manager
            .derive_center_of_mass();

        let original_position = fracture_region_object.rigid_body.position().aligned();
        let orientation = fracture_region_object.rigid_body.orientation().aligned();
        let original_linear_velocity = fracture_region_object.rigid_body.compute_velocity();
        let angular_velocity = fracture_region_object.rigid_body.compute_angular_velocity();

        let mut original_anchors = fracture_region_object.anchors;

        let min_mass =
            config.min_relative_fragment_mass * fracture_region_object.original_object_mass;

        self.complete(
            context,
            entity_id_manager,
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            original_entity_id,
            original_local_center_of_mass,
            original_position,
            orientation,
            original_linear_velocity,
            angular_velocity,
            &mut original_anchors,
            min_mass,
        );

        // All remaining anchors are not attached to any fragments, so we remove
        // them
        for (anchor_id, _) in original_anchors {
            anchor_manager.dynamic_mut().remove(anchor_id);
        }

        fracture_region_buffer_pool.push(fracture_region_object.voxel_object.into_buffers());
    }

    fn complete_for_existing_object<C>(
        &mut self,
        config: &VoxelFracturingConfig,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        original_voxel_object_id: VoxelObjectID,
    ) where
        C: VoxelObjectInteractionContext,
    {
        assert!(self.has_processed());

        let original_entity_id = original_voxel_object_id.as_entity_id();
        let original_rigid_body_id = DynamicRigidBodyID::from_entity_id(original_entity_id);

        // We need the original voxel object entity to still exist so the
        // fragments can inherit its components
        if !context.entity_exists(original_entity_id) {
            log::warn!(
                "Tried to complete fracturing for missing voxel object entity: {original_entity_id}"
            );
            self.state = FracturingProcessState::Cancelled;
            return;
        }
        let Some(physics_context) =
            voxel_object_manager.get_physics_context(original_voxel_object_id)
        else {
            log::warn!(
                "Tried to execute fracturing for voxel object {original_voxel_object_id} \
                 with missing physics context"
            );
            self.state = FracturingProcessState::Cancelled;
            return;
        };
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(original_rigid_body_id)
        else {
            log::warn!(
                "Tried to execute fracturing for voxel object {original_voxel_object_id} \
                 with missing rigid body"
            );
            self.state = FracturingProcessState::Cancelled;
            return;
        };

        let original_local_center_of_mass = physics_context
            .inertial_property_manager
            .derive_center_of_mass();

        let original_mass = rigid_body.mass();
        let original_position = rigid_body.position().aligned();
        let orientation = rigid_body.orientation().aligned();
        let original_linear_velocity = rigid_body.compute_velocity();
        let angular_velocity = rigid_body.compute_angular_velocity();

        let mut original_anchors =
            interaction::get_all_rigid_body_anchors(anchor_manager, original_rigid_body_id);

        let min_mass = config.min_relative_fragment_mass * original_mass;

        self.complete(
            context,
            entity_id_manager,
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            original_entity_id,
            original_local_center_of_mass,
            original_position,
            orientation,
            original_linear_velocity,
            angular_velocity,
            &mut original_anchors,
            min_mass,
        );

        context.remove_voxel_object_entity(original_entity_id);
    }

    fn complete<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        original_entity_id: EntityID,
        original_local_center_of_mass: Vector3,
        original_position: Point3,
        orientation: UnitQuaternion,
        original_linear_velocity: Velocity,
        angular_velocity: AngularVelocity,
        original_anchors: &mut Anchors,
        min_mass: f32,
    ) where
        C: VoxelObjectInteractionContext,
    {
        assert!(self.has_processed());

        let mut entity_ids = Vec::with_capacity(self.fragments.len());

        for mut fragment in self.fragments.drain(..) {
            let voxel_object = fragment.meshed_voxel_object.object();

            let dynamics = interaction::determine_extracted_voxel_object_dynamics(
                voxel_object,
                fragment.origin_offset_in_parent,
                &mut fragment.inertial_property_manager,
                original_local_center_of_mass,
                original_position,
                orientation,
                original_linear_velocity,
                angular_velocity,
            );

            if dynamics.rigid_body.mass() < min_mass {
                voxel_object_buffer_pool.add_buffers(fragment.meshed_voxel_object.into_buffers());
                continue;
            }

            let anchors = interaction::transfer_anchors_to_extracted_voxel_object(
                original_anchors,
                voxel_object,
                &dynamics.coordinate_changes,
            );

            let extracted_components = ExtractedComponents {
                meshed_voxel_object: fragment.meshed_voxel_object,
                inertial_property_manager: fragment.inertial_property_manager,
                rigid_body: dynamics.rigid_body,
                anchors,
            };

            let entity_id = entity_id_manager.provide_id();

            interaction::spawn_extracted_voxel_object(
                voxel_object_manager,
                rigid_body_manager,
                anchor_manager,
                extracted_components,
                entity_id,
            );

            entity_ids.push(entity_id);
        }

        context.create_extracted_voxel_object_entities(entity_ids, original_entity_id);

        self.reset(voxel_object_buffer_pool);
    }

    fn reset(&mut self, voxel_object_buffer_pool: &mut VoxelObjectBufferPool) {
        self.vertex_indices.clear();
        self.reclaim_fragment_buffers(voxel_object_buffer_pool);
        self.state = FracturingProcessState::Idle;
    }

    fn reclaim_fragment_buffers(&mut self, voxel_object_buffer_pool: &mut VoxelObjectBufferPool) {
        for fragment in self.fragments.drain(..) {
            let buffers = fragment.meshed_voxel_object.into_buffers();
            voxel_object_buffer_pool.add_buffers(buffers);
        }
    }
}

impl Default for FracturingProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl FracturePointGenerator {
    pub fn add_fracture_points<A: Allocator>(
        &self,
        points: &mut AVec<Point3C, A>,
        aabb: &AxisAlignedBoxC,
        seed: u64,
    ) {
        let mut rng = Rng::with_seed(seed);
        match self {
            Self::RandomizedGrid(seeder) => seeder.add_fracture_points(points, aabb, &mut rng),
        }
    }
}

impl RandomizedGridFracturePointGenerator {
    pub fn new(points_per_dim: usize) -> Self {
        assert_ne!(points_per_dim, 0);
        Self { points_per_dim }
    }

    pub fn add_fracture_points<A: Allocator>(
        &self,
        points: &mut AVec<Point3C, A>,
        aabb: &AxisAlignedBoxC,
        rng: &mut Rng,
    ) {
        let start = aabb.lower_corner();
        let scale = aabb.extents() / (self.points_per_dim as f32);

        points.reserve(self.points_per_dim.pow(3));

        for i in 0..self.points_per_dim {
            for j in 0..self.points_per_dim {
                for k in 0..self.points_per_dim {
                    points.push(
                        start
                            + Vector3C::new(
                                i as f32 + rng.random_f32_fraction(),
                                j as f32 + rng.random_f32_fraction(),
                                k as f32 + rng.random_f32_fraction(),
                            )
                            .component_mul(&scale),
                    );
                }
            }
        }
    }
}

fn extract_fracture_region_object<C>(
    context: &mut C,
    entity_id_manager: &mut EntityIDManager,
    voxel_type_registry: &VoxelTypeRegistry,
    voxel_object_manager: &mut VoxelObjectManager,
    voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
    rigid_body_manager: &mut RigidBodyManager,
    anchor_manager: &mut AnchorManager,
    fracture_region_buffer_pool: &mut Vec<VoxelObjectBuffers>,
    voxel_object_id: VoxelObjectID,
    fracture_region_tetras: &DelaunayTetrahedralization,
) -> Option<FractureRegionObject>
where
    C: VoxelObjectInteractionContext,
{
    let entity_id = voxel_object_id.as_entity_id();
    let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);

    let (Some((voxel_object, physics_context)), Some(rigid_body)) = (
        voxel_object_manager.get_voxel_object_with_physics_context_mut(voxel_object_id),
        rigid_body_manager.get_dynamic_rigid_body_mut(rigid_body_id),
    ) else {
        return None;
    };

    let arena = ArenaPool::get_arena();

    let object_aabb = voxel_object.object().compute_normalized_chunk_grid_bounds();
    let Some(fracture_region_aabb) = fracture_region_tetras
        .compute_aabb()
        .compute_overlap_with(&object_aabb)
    else {
        // If the fracture region is completely outside the object, there is
        // nothing to do
        return None;
    };

    // The convex hull of the boundary region points corresponds to the boundary
    // of the Delaunay tetrahedralization
    let normalized_face_planes = fracture_region_tetras.compute_boundary_face_planes(&arena);

    let original_local_center_of_mass = physics_context
        .inertial_property_manager
        .derive_center_of_mass();

    let original_mass = rigid_body.mass();
    let original_position = rigid_body.position().aligned();
    let orientation = rigid_body.orientation().aligned();
    let original_linear_velocity = rigid_body.compute_velocity();
    let angular_velocity = rigid_body.compute_angular_velocity();

    let mut fracture_region_inertial_property_manager =
        VoxelObjectInertialPropertyManager::zeroed();

    let mut inertial_property_transferrer =
        physics_context.inertial_property_manager.begin_transfer_to(
            &mut fracture_region_inertial_property_manager,
            voxel_object.object().voxel_extent(),
            voxel_type_registry.mass_densities(),
        );

    let object_buffers = fracture_region_buffer_pool.pop().unwrap_or_default();

    // We don't check the result yet, since the extraction could have modified
    // the object regardless of whether it resulted in a significant enough
    // extracted object, so we need to run post-removal updates regardless
    let extraction_result = voxel_object
        .object_mut()
        .extract_polyhedron_with_property_transferrer(
            object_buffers,
            &fracture_region_aabb,
            &normalized_face_planes,
            &mut inertial_property_transferrer,
        );

    let VoxelRemovalOutcome {
        original_object_empty,
        extracted_components_for_disconnected_objects,
        mut lost_anchors,
    } = interaction::handle_voxel_object_after_removing_voxels(
        &arena,
        anchor_manager,
        voxel_type_registry,
        voxel_object_buffer_pool,
        voxel_object.object_mut(),
        &mut physics_context.inertial_property_manager,
        rigid_body_id,
        rigid_body,
        original_local_center_of_mass,
        // The removed mass was transferred to the boundary region object
        RemovedMassFate::Transferred,
    );

    if original_object_empty {
        context.remove_voxel_object_entity(entity_id);
    }

    let disconnected_entity_ids =
        entity_id_manager.provide_id_vec(extracted_components_for_disconnected_objects.len());

    for (disconnected_entity_id, extracted_components) in disconnected_entity_ids
        .iter()
        .copied()
        .zip(extracted_components_for_disconnected_objects)
    {
        interaction::spawn_extracted_voxel_object(
            voxel_object_manager,
            rigid_body_manager,
            anchor_manager,
            extracted_components,
            disconnected_entity_id,
        );
    }

    context.create_extracted_voxel_object_entities(disconnected_entity_ids, entity_id);

    let extracted_fracture_region_object = match extraction_result {
        ExtractionResult::Extracted(extracted) => extracted,
        ExtractionResult::NotExtracted(object_buffers) => {
            fracture_region_buffer_pool.push(object_buffers);

            // Even though no significant object was extracted, anchors could
            // have been lost, so we must delete them before we abort
            for (anchor_id, _) in lost_anchors {
                anchor_manager.dynamic_mut().remove(anchor_id);
            }

            return None;
        }
    };

    let fracture_region_object_dynamics = interaction::determine_extracted_voxel_object_dynamics(
        &extracted_fracture_region_object.voxel_object,
        extracted_fracture_region_object.origin_offset_in_parent,
        &mut fracture_region_inertial_property_manager,
        original_local_center_of_mass,
        original_position,
        orientation,
        original_linear_velocity,
        angular_velocity,
    );

    let fracture_region_object_anchors = interaction::transfer_anchors_to_extracted_voxel_object(
        &mut lost_anchors,
        &extracted_fracture_region_object.voxel_object,
        &fracture_region_object_dynamics.coordinate_changes,
    );

    let fracture_region_object = FractureRegionObject {
        voxel_object: extracted_fracture_region_object.voxel_object,
        inertial_property_manager: fracture_region_inertial_property_manager,
        rigid_body: fracture_region_object_dynamics.rigid_body,
        anchors: fracture_region_object_anchors,
        origin_offset_in_parent: extracted_fracture_region_object.origin_offset_in_parent,
        original_object_mass: original_mass,
    };

    // Remaining anchors are not attached to anything, so we delete them
    for (anchor_id, _) in lost_anchors {
        anchor_manager.dynamic_mut().remove(anchor_id);
    }

    Some(fracture_region_object)
}

/// Generates fracture points and fracture region boundary points for a voxel
/// object based on the force of an impact.
///
/// The approach computes a probability density field for fracture points in the
/// spherical coordinate frame aligned with the force and samples points using
/// rejection sampling. The probability densities are computed based on the
/// force and the fracturing properties of the object using a physically
/// motivated toy model described in the `fracturing.jl` Pluto notebook.
fn generate_impact_fracture_points<A: Allocator>(
    config: &VoxelImpactFracturingConfig,
    arena: &PoolArena,
    fracture_region_boundary_points: &mut AVec<Point3C, A>,
    fracture_points: &mut AVec<Point3C, A>,
    properties: &FracturingProperties,
    inverse_voxel_extent: f32,
    world_to_object_transform: &Isometry3,
    aabb: &AxisAlignedBox,
    force: &FractureForce,
    rng: &mut Rng,
) {
    let force_to_world_transform = Isometry3::from_parts(
        *force.position.as_vector(),
        UnitQuaternion::rotation_between_axes(&UnitVector3::unit_z(), &force.direction),
    );
    let force_to_object_transform = world_to_object_transform * force_to_world_transform;

    let object_extent = aabb.volume().cbrt();
    assert!(object_extent > 0.0);

    let relative_force = force.magnitude / properties.fracturing_force;

    if relative_force <= 1.0 {
        // Force does not exceed the fracturing threshold
        return;
    }

    // By having the user specify a shattering pressure and scaling it here with
    // the squared object extent, larger objects will naturally require a larger
    // shattering force, which is the intuitive behavior
    let shattering_force = properties.shattering_pressure * object_extent.powi(2);

    // Scale the fragments appropriately based on object extent. These scalings
    // are set empirically to get believable but practical fragment sizes and
    // counts as the object extent changes.
    let fragment_scale = properties.fragment_scale * object_extent;
    let min_fragment_extent = properties.min_fragment_extent * object_extent.sqrt();
    let max_fragment_extent = properties.max_fragment_extent * object_extent;

    let radial_power = config.radial_falloff_power;
    let angular_power = config.angular_falloff_power;

    // Since a Voronoi diagram by nature always will subdivide the whole volume
    // it's defined for, we will first create a fracture region representing the
    // part of the object where the propagated force is significant enough to
    // cause fragmentation, and then create a Voronoi diagram for fragments
    // applied only to that region

    // The extent of the contact area affects how far from the impact point the
    // load from the impact can propagate and lead to fragmentation. It is
    // implicitly specified through the provided shattering force.
    let mut contact_extent = object_extent
        / ((shattering_force / properties.fracturing_force).powf(radial_power.recip()) - 1.0)
            .max(0.0);

    // For small shatter to fracture ratios the computed contact extent exceeds
    // the object extent, so we cap it to keep the value reasonable
    contact_extent = contact_extent.min(object_extent);

    // The fracture region extent is the maximum distance from the impact point
    // where the propagated force can lead to fragmentation. We will use it to
    // create a semi-hemispherical fracture region boundary within which all
    // fragmentation will occur.
    let fracture_region_extent =
        (contact_extent * (relative_force.powf(radial_power.recip()) - 1.0)).max(0.0);

    if fracture_region_extent < min_fragment_extent {
        // Fracture region is too small to contain a single valid fragment
        return;
    }

    let radial_scale = contact_extent.recip();

    // Limit for the load to ensure that the model's computed fragment extent
    // does not exceed the imposed maximum fragment extent
    let min_relative_load = fragment_scale / max_fragment_extent + 1.0;

    let nr = config.radial_grid_size;
    let nu = config.angular_grid_size;
    assert!(nr >= 2 && nu >= 2);

    // Grid cell extents along the radial (`r`) and cos(polar angle) (`u`)
    // dimensions in the spherical coordinate system aligned with the force. The
    // model assumes azimuthal symmetry around the force vector, so we don't
    // need an azimuthal dimension.
    let dr = fracture_region_extent / (nr - 1) as f32;
    let du = 1.0 / (nu - 1) as f32;

    // Grid for the fragment number density times the spherical volume element
    // as a function of `r` and `u`. This will be used both for determining the
    // total fragment count by integrating and as the unnormalized probability
    // density field for rejection sampling of fracture points.
    let mut n_dv_grid = avec![in arena; 0.0; nr * nu];

    // Grid for the fragment extent as a function of `r` and `u`
    let mut fragment_extent_grid = avec![in arena; 0.0; nr * nu];

    let grid_idx = |r_idx, u_idx| u_idx * nr + r_idx;
    let r_coord = |r_idx| dr * r_idx as f32;
    let u_coord = |u_idx| du * u_idx as f32;

    // Fill in the grids
    for u_idx in 0..nu {
        let u = u_coord(u_idx);
        for r_idx in 0..nr {
            let r = r_coord(r_idx);

            // The load is a measure of the influence of the impact force at a
            // given point in the object
            let relative_load = relative_force
                * (r * radial_scale + 1.0).powf(-radial_power)
                * u.powf(angular_power);

            // The expected fragment extent at a given point depends on by how
            // much the local load exceeds the fracture force threshold. But we
            // clamp the result to the imposed minimum and maximum extents.
            let mut fragment_extent = fragment_scale / (relative_load.max(min_relative_load) - 1.0);
            fragment_extent = fragment_extent.max(min_fragment_extent);

            // The local number density of fragments is the inverse of the local
            // fragment volume
            let number_density = fragment_extent.powi(-3);

            // Multiply with the spherical volume element
            let n_dv = f32::TWO_PI * r.powi(2) * number_density;

            let idx = grid_idx(r_idx, u_idx);

            n_dv_grid[idx] = n_dv;
            fragment_extent_grid[idx] = fragment_extent;
        }
    }

    // We need the maximum value of `n * dV` for rejection sampling. The model
    // guarantees that it occurs for `u = 1.0`, so we only need to check the
    // final row in the grid.
    let mut max_n_dv = f32::NEG_INFINITY;
    for &n_dv in &n_dv_grid[(nu - 1) * nr..] {
        if n_dv > max_n_dv {
            max_n_dv = n_dv;
        }
    }

    // Integrate up `n * dV` on the grid to get the fragment count
    let integrated_fragment_count = n_dv_grid.iter().sum::<f32>() * dr * du;

    // The computed fragment count (clamped to the imposed maximum) is the
    // number of fracture points we will try to generate through rejection
    // sampling
    let max_sample_count =
        (integrated_fragment_count.floor().max(1.0) as u64).min(config.max_fragment_count);

    // There may simply not be room for the integrated fragment count once we
    // impose our minimum distance between fracture points, so we will stop
    // generation after a maximum number of rejections that scales with fragment
    // count
    let max_position_rejections = config.max_position_rejections_per_sample * max_sample_count;

    let mut sample_count = 0;
    let mut position_rejection_count = 0;

    let start_point_count = fracture_points.len();

    'sampling: while sample_count < max_sample_count
        && position_rejection_count < max_position_rejections
    {
        // Pick uniformly random `r` and `u` values on the grid
        let r_idx = rng.random_usize_in_range(0..nr);
        let u_idx = rng.random_usize_in_range(0..nu);
        let idx = grid_idx(r_idx, u_idx);

        // Accept the point by probability proportional to `n * dV`
        let random_fraction = rng.random_f32_fraction();
        if random_fraction * max_n_dv > n_dv_grid[idx] {
            continue;
        }

        let fragment_extent = fragment_extent_grid[idx];

        let r = r_coord(r_idx);

        // Reject if the fragment (roughly) would protrude from the fracture
        // region
        if fracture_region_extent - r < 0.5 * fragment_extent {
            position_rejection_count += 1;
            continue;
        }

        // Draw a uniformly random azimuthal angle for the point
        let phi = f32::TWO_PI * rng.random_f32_fraction();
        let (sin_phi, cos_phi) = phi.sin_cos();

        let cos_theta = u_coord(u_idx);
        let sin_theta = (1.0 - cos_theta.powi(2)).max(0.0).sqrt();

        // Obtain the cartesian coordinates of the sampled fracture point, still
        // in the frame aligned with the force vector
        let force_sample_point = Point3C::new(
            r * sin_theta * cos_phi,
            r * sin_theta * sin_phi,
            r * cos_theta,
        );

        // Transform to the model space (not normalized yet) of the voxel object
        let object_sample_point =
            force_to_object_transform.transform_point(&force_sample_point.aligned());

        // Reject if it falls outside the object's AABB
        if !aabb.contains_point(&object_sample_point) {
            position_rejection_count += 1;
            continue;
        }

        // Now normalize to voxel units
        let sample_point = (object_sample_point * inverse_voxel_extent).compact();

        // We use the local fragment extent as the limit on how close a
        // neighboring fracure point is allowed to be. We divide by voxel extent
        // here to get normalized units.
        let min_squared_norm_dist = (fragment_extent * inverse_voxel_extent).powi(2);

        // Check the distance to each already generated point and reject if too close
        for point in &fracture_points[start_point_count..] {
            if Point3C::squared_distance_between(&sample_point, point) < min_squared_norm_dist {
                position_rejection_count += 1;
                continue 'sampling;
            }
        }

        fracture_points.push(sample_point);
        sample_count += 1;
    }

    generate_impact_fracture_region_boundary_points(
        config,
        fracture_region_boundary_points,
        force_to_object_transform,
        inverse_voxel_extent,
        fracture_region_extent,
        rng,
    );
}

/// Generates boundary points by stratified sampling on a hemispherical grid (in
/// the frame of the impact force) with radius `fracture_region_extent`.
fn generate_impact_fracture_region_boundary_points<A: Allocator>(
    config: &VoxelImpactFracturingConfig,
    fracture_region_boundary_points: &mut AVec<Point3C, A>,
    force_to_object_transform: Isometry3,
    inverse_voxel_extent: f32,
    fracture_region_extent: f32,
    rng: &mut Rng,
) {
    let mut add_point = |force_boundary_point: Point3C| {
        let object_boundary_point =
            force_to_object_transform.transform_point(&force_boundary_point.aligned());

        let boundary_point = (object_boundary_point * inverse_voxel_extent).compact();

        fracture_region_boundary_points.push(boundary_point);
    };

    // Polar coordinates are regular in u = cos(theta)
    let nu = config.boundary_polar_grid_size;
    let nphi = config.boundary_azimuthal_grid_size;

    // By not subtracting 1 in the denominator we avoid duplicate coordinates
    // (at u = 1) and (phi = 0 and 2π)
    let du = 1.0 / nu as f32;
    let dphi = f32::TWO_PI / nphi as f32;

    let u_jitter = du * config.boundary_angular_jitter;
    let phi_jitter = dphi * config.boundary_angular_jitter;
    let r_jitter = fracture_region_extent * config.boundary_radial_jitter;

    let u_center_coord = |u_idx| du * (u_idx as f32 + 0.5);
    let phi_center_coord = |phi_idx| dphi * (phi_idx as f32 + 0.5);

    for u_idx in 0..nu {
        let u_center = u_center_coord(u_idx);
        for phi_idx in 0..nphi {
            let phi_center = phi_center_coord(phi_idx);

            let u = u_center + u_jitter * (0.5 - rng.random_f32_fraction());
            let phi = phi_center + phi_jitter * (0.5 - rng.random_f32_fraction());
            let r = fracture_region_extent + r_jitter * (0.5 - rng.random_f32_fraction());

            let cos_theta = u.clamp(0.0, 1.0);
            let sin_theta = f32::sqrt((1.0 - u.powi(2)).max(0.0));

            let (sin_phi, cos_phi) = phi.sin_cos();

            add_point(Point3C::new(
                r * sin_theta * cos_phi,
                r * sin_theta * sin_phi,
                r * cos_theta,
            ));
        }
    }

    // Add a final point at the apex of the opposite hemisphere to close the
    // shape in a way that doesn't lead to sharp artifacts if the impact point
    // lies in a concavity
    add_point(Point3C::new(0.0, 0.0, -fracture_region_extent));
}
