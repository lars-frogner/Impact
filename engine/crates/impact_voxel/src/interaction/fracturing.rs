//! Voxel object fracturing.

use crate::{
    VoxelObjectBufferPool, VoxelObjectID, VoxelObjectManager,
    collidable::{Collidable, CollisionWorld},
    interaction::{self, ExtractedComponents, VoxelObjectInteractionContext},
    mesh::{MeshedVoxelObject, MeshedVoxelObjectBuffers},
    object::{
        ChunkRanges, VoxelObject, extraction::ExtractionResult,
        inertia::VoxelObjectInertialPropertyManager,
    },
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Context, Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{
    AVec, Allocator,
    arena::{ArenaPool, PoolArena},
    avec,
};
use impact_containers::{HashMap, HashSet};
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_id::{EntityID, EntityIDManager};
use impact_math::{
    consts::f32,
    point::{Point3, Point3C},
    quaternion::UnitQuaternion,
    random::Rng,
    transform::Isometry3,
    vector::{UnitVector3, UnitVector3C, Vector3, Vector3C},
};
use impact_physics::{
    anchor::AnchorManager,
    collision::CollidableID,
    constraint::{ConstrainedBodyManager, ConstraintManager},
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
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
use std::{
    cmp::Ordering,
    collections::VecDeque,
    time::{Duration, Instant},
};

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
        pub force_threshold: f32,
        /// The characteristic length scale of generated fragments, relative to
        /// the extent of the object.
        pub fragment_scale: f32,
        /// The target minimum extent of generated fragments. Will be converted
        /// to world units based on the object size such that the minimum
        /// fragment size in world space scale increases weakly with the object
        /// size.
        pub min_fragment_extent: f32,
        /// The target maximum extent of generated fragments, relative to the
        /// extent of the object. Should be ~0.5 in most cases.
        pub max_fragment_extent: f32,
    }
}

/// Manages voxel object fracturing processes and state.
#[derive(Debug)]
pub struct VoxelObjectFracturingManager {
    active_processes: HashMap<VoxelObjectID, FracturingProcess>,
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
    /// If set, the processing time for generating fracture objects per frame
    /// will be attempted limited to this number of microseconds. The time
    /// budget may be exceeded to spawn the fracture objects for completed
    /// processes and to make sure all processes make enough progress to counter
    /// the rate of invalidation due to objects being modified.
    ///
    /// Note: Setting this duration breaks determinism.
    pub max_processing_duration_us: Option<u64>,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct VoxelImpactFracturingConfig {
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
struct FracturingProcess {
    state: FracturingProcessState,
    fracture_points: Vec<Point3C>,
    processing_direction: Option<Vector3C>,
    tetrahedralization: DelaunayTetrahedralization,
    dual_vertex_queue: VecDeque<VertexIdx>,
    fragments: Vec<Fragment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FracturingProcessState {
    Idle,
    Initiated,
    Cancelled,
}

#[derive(Debug)]
struct Fragment {
    dual_vertex_idx: VertexIdx,
    meshed_voxel_object: MeshedVoxelObject,
    origin_offset_in_parent: [usize; 3],
    chunk_ranges_in_parent: ChunkRanges,
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
pub struct ImpulseFracturePointGenerator {}

#[derive(Clone, Debug)]
struct FractureForce {
    position: Point3,
    direction: UnitVector3,
    magnitude: f32,
}

impl VoxelObjectFracturingManager {
    /// Creates a new empty fracturing manager with the given configuration.
    pub fn new(config: VoxelFracturingConfig) -> Self {
        Self {
            active_processes: HashMap::default(),
            process_pool: Vec::new(),
            config,
        }
    }

    /// Adds the given fracture points to use in the fracturing process of the
    /// given voxel object. Fracture points can be added multiple times. When
    /// all fracturing points are added, call
    /// [`Self::initiate_fracturing_process`] to commit the fracture points and
    /// enable the fracturing process to be executed with
    /// [`Self::execute_fracturing_processes`].
    ///
    /// If `processing_direction` is specified, the fracture objects will be
    /// generated in order of their projected distance along that direction.
    /// This can reduce the amount of wasted work if chunk invalidation is
    /// expected to happen on a particular side of the object. When multiple
    /// processing directions are specified (across multiple calls), their
    /// average is used.
    ///
    /// Note that both the fracture points and processing direction should be
    /// specified in the normalized space of the voxel object (where distance is
    /// in units of voxels).
    ///
    /// # Errors
    /// Returns an error if:
    /// - The voxel object does not exist.
    /// - Fracturing has already been initiated for the object.
    pub fn add_fracture_points_for_object(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_id: VoxelObjectID,
        fracture_points: &[Point3C],
        processing_direction: Option<&UnitVector3C>,
    ) -> Result<()> {
        if !voxel_object_manager.has_voxel_object(voxel_object_id) {
            bail!("Tried to add fracture points for missing voxel object {voxel_object_id}");
        }

        let process = self
            .active_processes
            .entry(voxel_object_id)
            .or_insert_with(|| {
                self.process_pool
                    .pop()
                    .unwrap_or_else(FracturingProcess::new)
            });

        log::debug!(
            "Adding {} fracture points for voxel object: {voxel_object_id}",
            fracture_points.len()
        );
        process
            .add_fracture_points(fracture_points, processing_direction)
            .with_context(|| {
                format!("Failed to add fracture points for voxel object: {voxel_object_id}")
            })
    }

    /// Stages the given voxel object for fracturing using all fracture points
    /// added for the object through [`Self::add_fracture_points_for_object`].
    /// The actual processing will not happen until
    /// [`Self::execute_fracturing_processes`] is called.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The voxel object does not exist.
    /// - [`Self::add_fracture_points_for_object`] has not been called for the object.
    /// - Fracturing has already been initiated for the object.
    pub fn initiate_fracturing_process(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_id: VoxelObjectID,
    ) -> Result<()> {
        if !voxel_object_manager.has_voxel_object(voxel_object_id) {
            bail!("Tried to initiate fracturing for missing voxel object {voxel_object_id}");
        }

        let process = self
            .active_processes
            .get_mut(&voxel_object_id)
            .ok_or_else(|| {
                anyhow!(
                    "Tried to initiate fracturing for voxel object {voxel_object_id} \
                     without adding fracture points first"
                )
            })?;

        log::debug!("Initiating fracturing process for voxel object: {voxel_object_id}");
        process.initiate().with_context(|| {
            format!("Failed to initiate fracturing process for voxel object: {voxel_object_id}")
        })
    }

    /// Executes all initiated fracturing processes.
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
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            |voxel_object_manager,
             voxel_object_buffer_pool,
             rigid_body_manager,
             process,
             voxel_object_id,
             remaining_duration| {
                process.execute(
                    voxel_type_registry,
                    voxel_object_manager,
                    voxel_object_buffer_pool,
                    rigid_body_manager,
                    voxel_object_id,
                    remaining_duration,
                );
            },
        );
    }

    /// Executes all initiated fracturing processes.
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
            voxel_object_manager,
            voxel_object_buffer_pool,
            rigid_body_manager,
            anchor_manager,
            |voxel_object_manager,
             voxel_object_buffer_pool,
             rigid_body_manager,
             process,
             voxel_object_id,
             remaining_duration| {
                process.execute_in_parallel(
                    thread_pool,
                    voxel_type_registry,
                    voxel_object_manager,
                    voxel_object_buffer_pool,
                    rigid_body_manager,
                    voxel_object_id,
                    remaining_duration,
                );
            },
        );
    }

    /// Whether a fracturing process has been initiated for the given voxel
    /// object.
    pub fn object_has_initiated_fracturing_process(&self, voxel_object_id: VoxelObjectID) -> bool {
        self.active_processes
            .get(&voxel_object_id)
            .is_some_and(|process| process.is_initiated())
    }

    fn execute_fracturing_processes_with_closure<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        execute_process: impl Fn(
            &mut VoxelObjectManager,
            &mut VoxelObjectBufferPool,
            &mut RigidBodyManager,
            &mut FracturingProcess,
            VoxelObjectID,
            Duration,
        ),
    ) where
        C: VoxelObjectInteractionContext,
    {
        let arena = ArenaPool::get_arena();
        let mut finished_voxel_object_ids = AVec::new_in(&arena);

        let mut remaining_duration = self
            .config
            .max_processing_duration_us
            .map_or(Duration::MAX, Duration::from_micros);

        for (&voxel_object_id, process) in &mut self.active_processes {
            if process.is_idle() {
                continue;
            }
            assert!(!process.is_cancelled());

            let start_time = Instant::now();

            execute_process(
                voxel_object_manager,
                voxel_object_buffer_pool,
                rigid_body_manager,
                process,
                voxel_object_id,
                remaining_duration,
            );

            if process.is_complete() || process.is_cancelled() {
                finished_voxel_object_ids.push(voxel_object_id);
            }

            // We don't break when the remaining duration reaches zero, because
            // we need to allow every process to regenerate enough of their
            // invalidated objects
            remaining_duration = remaining_duration.saturating_sub(start_time.elapsed());
        }

        for voxel_object_id in finished_voxel_object_ids {
            let mut process = self.active_processes.remove(&voxel_object_id).unwrap();

            if process.is_complete() {
                log::debug!("Completing fracturing for voxel object: {voxel_object_id}");
                process.complete(
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

            // Reset and return to pool (whether it was successfully completed
            // or cancelled before or during completion)
            process.reset(voxel_object_buffer_pool);
            self.process_pool.push(process);
        }
    }

    /// Identifies collisions involving fragmentable objects with impulses
    /// strong enough to cause fragmentation and initiates fracturing with
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

        // Reusable buffer for generated fracture points
        let mut fracture_points = AVec::new_in(&arena);

        // Voxels objects to initiate fracturing for
        let mut fractured_objects =
            HashSet::with_capacity_and_hasher_in(0, Default::default(), &arena);

        // Entity pairs for collisions where one or both of the bodies were
        // fractured
        let mut fracture_collision_entities = AVec::new_in(&arena);

        let collidable_voxel_object = |id: CollidableID| {
            let object_id = VoxelObjectID::from_entity_id(id.as_entity_id());
            voxel_object_manager
                .get_voxel_object(object_id)
                .map(|object| (object_id, object.object()))
        };

        let get_fracturing_properties =
            |fracturing_manager: &VoxelObjectFracturingManager,
             object: Option<(VoxelObjectID, &VoxelObject)>| {
                let (object_id, _) = object?;
                if fracturing_manager.object_has_initiated_fracturing_process(object_id) {
                    // Treat the object as not fracturable if there is has an
                    // ongoing fracturing process
                    return None;
                }
                context.get_fracturing_properties_for_entity(object_id.as_entity_id())
            };

        let inverse_time_step_duration = time_step_duration.recip();

        let mut rng = Rng::with_seed(self.config.impact.seed);

        for collision in collisions {
            let voxel_object_a_with_id = collidable_voxel_object(collision.collidable_a_id);
            let voxel_object_b_with_id = collidable_voxel_object(collision.collidable_b_id);

            let properties_a = get_fracturing_properties(self, voxel_object_a_with_id);
            let properties_b = get_fracturing_properties(self, voxel_object_b_with_id);

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

            let mut created_fracture = false;

            for (voxel_object_with_id, collidable, properties, force) in [
                (
                    voxel_object_a_with_id,
                    collidable_a,
                    properties_a,
                    max_contact_force,
                ),
                (
                    voxel_object_b_with_id,
                    collidable_b,
                    properties_b,
                    -max_contact_force,
                ),
            ] {
                let Some(properties) = properties else {
                    // The object can not be fractured
                    continue;
                };
                if force_magnitude <= properties.force_threshold {
                    // The force is insufficient to cause fracturing
                    continue;
                }
                let Some((voxel_object_id, voxel_object)) = voxel_object_with_id else {
                    // The colliding body is not a voxel object
                    continue;
                };
                let Collidable::VoxelObject(collidable) = collidable.collidable() else {
                    panic!("Unexpected collidable for voxel object");
                };

                let force_direction = UnitVector3::unchecked_from(force / force_magnitude);

                let world_to_object_transform = collidable.transform_to_object_space().aligned();
                let aabb = voxel_object.compute_aabb();

                fracture_points.clear();

                let fracture_force = FractureForce {
                    position: max_contact_position,
                    direction: force_direction,
                    magnitude: force_magnitude,
                };

                generate_impact_fracture_points(
                    &self.config.impact,
                    &arena,
                    &mut fracture_points,
                    &properties,
                    voxel_object.inverse_voxel_extent(),
                    &world_to_object_transform,
                    &aabb,
                    &fracture_force,
                    &mut rng,
                );

                if fracture_points.is_empty() {
                    continue;
                }

                // Process fragments starting farthest away from the impact
                // point since the chunks near the impact point are most likely
                // to get invalidated if processing takes more than one frame
                let processing_direction = (-force_direction).compact();

                log::debug!(
                    "Fracturing voxel object {voxel_object_id}: force magnitude {force_magnitude:.3e} \
                     exceeds threshold {threshold:.3e}, \
                     direction = [{dx:.3}, {dy:.3}, {dz:.3}], \
                     {fracture_point_count} fracture point(s) generated",
                    threshold = properties.force_threshold,
                    dx = force_direction.x(),
                    dy = force_direction.y(),
                    dz = force_direction.z(),
                    fracture_point_count = fracture_points.len(),
                );

                self.add_fracture_points_for_object(
                    voxel_object_manager,
                    voxel_object_id,
                    &fracture_points,
                    Some(&processing_direction),
                )
                .unwrap();

                fractured_objects.insert(voxel_object_id);
                created_fracture = true;
            }

            if created_fracture {
                fracture_collision_entities.push([
                    collision.collidable_a_id.as_entity_id(),
                    collision.collidable_b_id.as_entity_id(),
                ]);
            }
        }

        for &object_id in &fractured_objects {
            self.initiate_fracturing_process(voxel_object_manager, object_id)
                .unwrap();
        }

        for entity_ids in fracture_collision_entities {
            // Prevent the impulses from the fracturing collision from being
            // applied until the fragments are generated
            constraint_manager.add_collision_to_ignore_list(entity_ids);

            // if entity_ids.iter().all(|&entity_id| {
            //     voxel_object_manager.has_voxel_object(VoxelObjectID::from_entity_id(entity_id))
            // }) {
            //     todo!("Enable mutual voxel absorption for the pair of objects")
            // }
        }
    }

    /// Removes all fracturing state and frees up all allocated memory.
    pub fn reset_and_free(&mut self) {
        self.active_processes = HashMap::default();
        self.process_pool = Vec::new();
    }
}

impl Default for VoxelFracturingConfig {
    fn default() -> Self {
        Self {
            impact: Default::default(),
            min_relative_fragment_mass: 1e-3,
            max_processing_duration_us: None,
        }
    }
}

impl Default for VoxelImpactFracturingConfig {
    fn default() -> Self {
        Self {
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

impl FracturingProcess {
    fn new() -> Self {
        Self {
            state: FracturingProcessState::Idle,
            fracture_points: Vec::new(),
            processing_direction: None,
            tetrahedralization: DelaunayTetrahedralization::new(),
            dual_vertex_queue: VecDeque::new(),
            fragments: Vec::new(),
        }
    }

    fn is_idle(&self) -> bool {
        self.state == FracturingProcessState::Idle
    }

    fn is_initiated(&self) -> bool {
        self.state == FracturingProcessState::Initiated
    }

    fn is_cancelled(&self) -> bool {
        self.state == FracturingProcessState::Cancelled
    }

    fn is_complete(&self) -> bool {
        self.is_initiated() && self.dual_vertex_queue.is_empty()
    }

    fn add_fracture_points(
        &mut self,
        fracture_points: &[Point3C],
        processing_direction: Option<&UnitVector3C>,
    ) -> Result<()> {
        if fracture_points.is_empty() {
            return Ok(());
        }
        if !self.is_idle() {
            bail!(
                "Tried to add fracture points to a non-idle fracturing process: {:?}",
                self.state
            );
        }
        assert!(self.dual_vertex_queue.is_empty());
        assert!(self.fragments.is_empty());

        self.fracture_points.extend_from_slice(fracture_points);

        if let Some(&dir) = processing_direction.map(UnitVector3C::as_vector) {
            self.processing_direction = Some(
                self.processing_direction
                    .map_or(dir, |current| current + dir),
            );
        }

        Ok(())
    }

    fn initiate(&mut self) -> Result<()> {
        if !self.is_idle() {
            bail!(
                "Tried to initiate a non-idle fracturing process: {:?}",
                self.state
            );
        }
        assert!(self.dual_vertex_queue.is_empty());
        assert!(self.fragments.is_empty());

        self.tetrahedralization.reconstruct(&self.fracture_points)?;

        if self.tetrahedralization.n_tetrahedra() == 0 {
            self.fracture_points.clear();
            self.processing_direction = None;
            return Ok(());
        }

        if let Some(direction) = self
            .processing_direction
            .and_then(|dir| UnitVector3C::normalized_from_if_above(dir, 1e-6))
        {
            Self::queue_vertices_sorted_along_direction(
                &mut self.dual_vertex_queue,
                &self.tetrahedralization,
                &direction,
            );
        } else {
            self.dual_vertex_queue
                .extend(self.tetrahedralization.internal_vertex_indices());
        }

        self.state = FracturingProcessState::Initiated;

        Ok(())
    }

    fn queue_vertices_sorted_along_direction(
        vertex_queue: &mut VecDeque<VertexIdx>,
        tetrahedralization: &DelaunayTetrahedralization,
        direction: &UnitVector3C,
    ) {
        let arena = ArenaPool::get_arena();

        let vertex_range = tetrahedralization.internal_vertex_indices();
        let mut sorted_vertices = AVec::with_capacity_in(vertex_range.len(), &arena);
        sorted_vertices.extend(vertex_range);

        sorted_vertices.sort_unstable_by(|&idx_a, &idx_b| {
            let position_a = tetrahedralization.vertices()[idx_a as usize].point;
            let position_b = tetrahedralization.vertices()[idx_b as usize].point;

            let displacement_a = direction.dot(position_a.as_vector());
            let displacement_b = direction.dot(position_b.as_vector());

            if displacement_a < displacement_b {
                Ordering::Less
            } else if displacement_a > displacement_b {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        vertex_queue.extend(sorted_vertices);
    }

    fn execute(
        &mut self,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &RigidBodyManager,
        voxel_object_id: VoxelObjectID,
        max_duration: Duration,
    ) {
        if !self.is_initiated() {
            return;
        }

        let Some(voxel_object) = Self::get_voxel_object_for_execution(
            voxel_object_manager,
            rigid_body_manager,
            voxel_object_id,
        ) else {
            self.state = FracturingProcessState::Cancelled;
            return;
        };

        let aabb = voxel_object.compute_normalized_chunk_grid_bounds();

        let arena = ArenaPool::get_arena();

        let max_remaining = self.invalidate_required_completed_objects_and_get_max_remaining(
            voxel_object_buffer_pool,
            &arena,
            voxel_object,
        );

        let mut polyhedron = VoronoiPolyhedron::empty_in(&arena);

        let n_generated_before = self.fragments.len();
        let start_time = Instant::now();

        while let Some(dual_vertex_idx) = self.dual_vertex_queue.pop_front() {
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

            if start_time.elapsed() > max_duration && self.dual_vertex_queue.len() <= max_remaining
            {
                break;
            }
        }

        self.log_execution_stats(voxel_object_id, 1, n_generated_before, &start_time);
    }

    fn execute_in_parallel(
        &mut self,
        thread_pool: &DynamicThreadPool,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &RigidBodyManager,
        voxel_object_id: VoxelObjectID,
        max_duration: Duration,
    ) {
        if !self.is_initiated() {
            return;
        }

        let Some(voxel_object) = Self::get_voxel_object_for_execution(
            voxel_object_manager,
            rigid_body_manager,
            voxel_object_id,
        ) else {
            self.state = FracturingProcessState::Cancelled;
            return;
        };

        let aabb = voxel_object.compute_normalized_chunk_grid_bounds();

        let arena = ArenaPool::get_arena();

        let max_remaining = self.invalidate_required_completed_objects_and_get_max_remaining(
            voxel_object_buffer_pool,
            &arena,
            voxel_object,
        );

        let num_threads = thread_pool.n_workers().get();

        let n_generated_before = self.fragments.len();
        let start_time = Instant::now();

        let deadline_exceeded = |queue: &VecDeque<VertexIdx>| {
            start_time.elapsed() > max_duration && queue.len() <= max_remaining
        };

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

                'outer: while !self.dual_vertex_queue.is_empty() {
                    // Send task inputs to the workers until the input buffer is
                    // full (or we are out of vertices to process)
                    while !input_sender.is_full() {
                        let Some(dual_vertex_idx) = self.dual_vertex_queue.pop_front() else {
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

                        // Stop sending and start receiving the in-flight
                        // results if we have run out of time
                        if deadline_exceeded(&self.dual_vertex_queue) {
                            break 'outer;
                        }
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

                    // Break the send-receive loop and start receiving all
                    // in-flight results if we have run out of time
                    if deadline_exceeded(&self.dual_vertex_queue) {
                        break 'outer;
                    }
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

        self.log_execution_stats(
            voxel_object_id,
            num_threads,
            n_generated_before,
            &start_time,
        );
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
                    dual_vertex_idx,
                    meshed_voxel_object: meshed_poly_object,
                    origin_offset_in_parent: poly_object.origin_offset_in_parent,
                    chunk_ranges_in_parent: poly_object.chunk_ranges_in_parent,
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

    fn invalidate_required_completed_objects_and_get_max_remaining(
        &mut self,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        arena: &PoolArena,
        voxel_object: &VoxelObject,
    ) -> usize {
        let original_completed_count = self.fragments.len();

        self.invalidate_required_completed_objects(voxel_object_buffer_pool, arena, voxel_object);

        // If invalidation happens faster than we can keep up, we are allowed to
        // exceed the time budget. We require that at least twice the number of
        // invalidated objects must be generated during each execution so that
        // we are guaranteed to finish.
        let invalidated_count = original_completed_count - self.fragments.len();
        let min_generated = 2 * invalidated_count;
        let max_remaining = self.dual_vertex_queue.len().saturating_sub(min_generated);

        max_remaining
    }

    fn invalidate_required_completed_objects(
        &mut self,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        arena: &PoolArena,
        voxel_object: &VoxelObject,
    ) {
        let mut invalidated_object_indices = AVec::new_in(arena);

        for invalidated_chunk_indices in voxel_object.invalidated_mesh_chunk_indices() {
            // Find each completed object whose chunk ranges in the parent
            // contain the invalidated chunk and store the object's index
            for (object_idx, fragment) in self.fragments.iter().enumerate() {
                let chunk_ranges = fragment.chunk_ranges_in_parent.iter();
                if chunk_ranges
                    .zip(invalidated_chunk_indices)
                    .all(|(range, idx)| range.contains(idx))
                {
                    invalidated_object_indices.push(object_idx);
                }
            }

            // Remove the invalidated objects from the list of completed
            // objects, return their buffers to the pool and push their vertex
            // index to the back of the queue so it will be recreated. The
            // reason we push to the back of the queue and not to the front is
            // that an invalidated object is likely to be invalidated again, so
            // we defer recreation as long as possible to limit wasted work. We
            // iterate over the invalidated indices in descending order so that
            // the swap-removes do not invalidate indices we have not yet
            // processed.
            for &object_idx in invalidated_object_indices.iter().rev() {
                let fragment = self.fragments.swap_remove(object_idx);

                voxel_object_buffer_pool.add_buffers(fragment.meshed_voxel_object.into_buffers());

                self.dual_vertex_queue.push_back(fragment.dual_vertex_idx);
            }

            invalidated_object_indices.clear();
        }
    }

    fn log_execution_stats(
        &self,
        voxel_object_id: VoxelObjectID,
        num_threads: usize,
        n_generated_before: usize,
        start_time: &Instant,
    ) {
        let n_generated = self.fragments.len() - n_generated_before;

        let n_total = self.tetrahedralization.internal_vertex_indices().len();
        let n_completed_total = n_total - self.dual_vertex_queue.len();

        let elapsed_ms = 1e3 * start_time.elapsed().as_secs_f64();

        log::debug!(
            "Generated {n_generated} fracture objects ({n_completed_total}/{n_total} complete) \
             in {elapsed_ms:.2} ms ({num_threads} thread(s)) for voxel object: {voxel_object_id}"
        );
    }

    fn complete<C>(
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
        assert!(self.is_complete());

        let original_entity_id = original_voxel_object_id.as_entity_id();
        let original_rigid_body_id = DynamicRigidBodyID::from_entity_id(original_entity_id);

        if !voxel_object_manager.has_voxel_object(original_voxel_object_id) {
            log::warn!(
                "Tried to complete fracturing for missing voxel object: {original_voxel_object_id}"
            );
            self.state = FracturingProcessState::Cancelled;
            return;
        };
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

            if dynamics.rigid_body.mass() < config.min_relative_fragment_mass * original_mass {
                voxel_object_buffer_pool.add_buffers(fragment.meshed_voxel_object.into_buffers());
                continue;
            }

            let anchors = interaction::get_anchors_on_extracted_voxel_object(
                anchor_manager,
                original_rigid_body_id,
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
        context.remove_voxel_object_entity(original_entity_id);

        self.reset(voxel_object_buffer_pool);
    }

    fn reset(&mut self, voxel_object_buffer_pool: &mut VoxelObjectBufferPool) {
        self.fracture_points.clear();
        self.processing_direction = None;
        self.dual_vertex_queue.clear();
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

/// Generates fracture points for a voxel object based on the force of an
/// impact.
///
/// The approach computes a probability density field for fracture points in the
/// spherical coordinate frame aligned with the force and samples points using
/// rejection sampling. The probability densities are computed based on the
/// force and the fracturing properties of the object using a physically
/// motivated toy model described in the `fracturing.jl` Pluto notebook.
fn generate_impact_fracture_points<A: Allocator>(
    config: &VoxelImpactFracturingConfig,
    arena: &PoolArena,
    points: &mut AVec<Point3C, A>,
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

    let relative_force = force.magnitude / properties.force_threshold;

    if relative_force <= 1.0 {
        // Force does not exceed the fracturing threshold
        return;
    }

    // Scale the fracturing properties appropriately based on object extent.
    // These scalings are set empirically to get believable but practical
    // fragment sizes and counts as the object extent changes.
    let fragment_scale = properties.fragment_scale * object_extent;
    let min_fragment_extent = properties.min_fragment_extent * object_extent.sqrt();
    let max_fragment_extent = properties.max_fragment_extent * object_extent;

    let radial_power = config.radial_falloff_power;
    let angular_power = config.angular_falloff_power;

    // The extent of the contact area affects how far from the impact point the
    // load from the impact can propagate and lead to fragmentation. Since a
    // Voronoi diagram by nature always will subdivide the whole object, we
    // compute the contact extent ad-hoc so that the model produces fracture
    // points across the entire object.
    let mut contact_extent =
        object_extent / (relative_force.powf(radial_power.recip()) - 1.0).max(0.0);

    // For small forces the computed contact extent exceeds the object extent,
    // so we cap it to keep the value reasonable
    contact_extent = contact_extent.min(object_extent);

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
    let dr = object_extent / (nr - 1) as f32;
    let du = 1.0 / (nu - 1) as f32;

    // Grid for the fragment number density times the spherical volume element
    // as a function of `r` and `u`. This will be used both for determining the
    // total fragment count by integrating and as the unnormalized probability
    // density field for rejection sampling of fracture points.
    let mut n_dv_grid = avec![in arena; 0.0; nr * nu];

    // Grid for the minimum squared distance between fracture points as a
    // function of `r` and `u`, in units of voxels rather than world units
    let mut min_squared_norm_dist_grid = avec![in arena; 0.0; nr * nu];

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

            // We use the local fragment extent as the limit on how close a
            // neighboring fracure point is allowed to be. We divide by voxel
            // extent here to get normalized units.
            min_squared_norm_dist_grid[idx] = (fragment_extent * inverse_voxel_extent).powi(2);
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

    let start_point_count = points.len();

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

        // Draw a uniformly random azimuthal angle for the point
        let phi = f32::TWO_PI * rng.random_f32_fraction();
        let (sin_phi, cos_phi) = phi.sin_cos();

        let r = r_coord(r_idx);

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

        let min_squared_norm_dist = min_squared_norm_dist_grid[idx];

        // Check the distance to each already generated point and reject if too close
        for point in &points[start_point_count..] {
            if Point3C::squared_distance_between(&sample_point, point) < min_squared_norm_dist {
                position_rejection_count += 1;
                continue 'sampling;
            }
        }

        points.push(sample_point);
        sample_count += 1;
    }
}
