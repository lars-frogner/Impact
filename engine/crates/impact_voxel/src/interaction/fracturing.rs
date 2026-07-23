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
use impact_alloc::{
    AVec, Allocator,
    arena::{ArenaPool, PoolArena},
};
use impact_containers::{HashMap, HashSet};
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_id::EntityIDManager;
use impact_math::{
    point::{Point3, Point3C},
    random::Rng,
    transform::Similarity3,
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
use std::{
    cmp::Ordering,
    collections::VecDeque,
    time::{Duration, Instant},
};

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
    /// If set, the processing time for generating fracture objects per frame
    /// will be attempted limited to this number of microseconds. The time
    /// budget may be exceeded to spawn the fracture objects for completed
    /// processes and to make sure all processes make enough progress to counter
    /// the rate of invalidation due to objects being modified.
    ///
    /// Note: Setting this duration breaks determinism.
    pub max_processing_duration_us: Option<u64>,
}

#[derive(Debug)]
struct FracturingProcess {
    state: FracturingProcessState,
    fracture_points: Vec<Point3C>,
    processing_direction: Option<Vector3C>,
    tetrahedralization: DelaunayTetrahedralization,
    dual_vertex_queue: VecDeque<VertexIdx>,
    fracture_objects: Vec<FractureObject>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FracturingProcessState {
    Idle,
    Initiated,
    Cancelled,
}

#[derive(Debug)]
struct FractureObject {
    dual_vertex_idx: VertexIdx,
    meshed_voxel_object: MeshedVoxelObject,
    origin_offset_in_parent: [usize; 3],
    chunk_ranges_in_parent: ChunkRanges,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum FractureObjectGenerationResult {
    Generated(FractureObject),
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

            log::trace!(target: "contact_gen",
                "fracture execute object={} n_completed={} queue_len={}",
                voxel_object_id, process.fracture_objects.len(), process.dual_vertex_queue.len());

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

    pub fn handle_fracturing_impacts(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        rigid_body_manager: &RigidBodyManager,
        constraint_manager: &mut ConstraintManager,
        collision_world: &CollisionWorld,
        time_step_duration: f32,
        rng: &mut Rng,
    ) {
        let arena = ArenaPool::get_arena();

        let Some(collisions) = collision_world.cached_collisions() else {
            return;
        };

        let fracture_point_generator = ImpulseFracturePointGenerator::new();

        let mut body_manager = ConstrainedBodyManager::new_in(&arena);
        let mut fracture_points = AVec::new_in(&arena);
        let mut fractured_objects =
            HashSet::with_capacity_and_hasher_in(0, Default::default(), &arena);
        let mut fracture_collision_entities = AVec::new_in(&arena);

        let collidable_object = |id: CollidableID| {
            let object_id = VoxelObjectID::from_entity_id(id.as_entity_id());
            voxel_object_manager
                .get_voxel_object(object_id)
                .map(|object| (object_id, object.object()))
        };

        let object_can_be_fractured =
            |fracturing_manager: &VoxelObjectFracturingManager,
             object: Option<(VoxelObjectID, &VoxelObject)>| {
                // TODO: Check fracture impulse component
                object.is_some_and(|(id, _)| {
                    !fracturing_manager.object_has_initiated_fracturing_process(id)
                })
            };

        let fracture_force_threshold =
            |fracturing_manager: &VoxelObjectFracturingManager,
             object: Option<(VoxelObjectID, &VoxelObject)>| {
                if object_can_be_fractured(fracturing_manager, object) {
                    // TODO
                    5e5
                } else {
                    f32::INFINITY
                }
            };

        for collision in collisions {
            let object_a = collidable_object(collision.collidable_a_id);
            let object_b = collidable_object(collision.collidable_b_id);

            let force_threshold_a = fracture_force_threshold(self, object_a);
            let force_threshold_b = fracture_force_threshold(self, object_b);

            if force_threshold_a == f32::INFINITY && force_threshold_b == f32::INFINITY {
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

            let Some((body_a_idx, body_b_idx)) = body_manager.add_body_pair(
                rigid_body_manager,
                descriptor_a.rigid_body_id(),
                descriptor_b.rigid_body_id(),
            ) else {
                continue;
            };

            let body_a = body_manager.body(body_a_idx);
            let body_b = body_manager.body(body_b_idx);

            let mut max_impulse = f32::NEG_INFINITY;
            let mut fracture_force = Vector3::zeros();
            let mut fracture_position = Point3::origin();

            for contact in collision.contact_manifold.contacts() {
                let contact = &contact.contact;
                let geometry = &contact.geometry;

                let impulse = contact.compute_normal_impulse(body_a, body_b);

                if impulse > max_impulse {
                    let force = (impulse / time_step_duration) * geometry.surface_normal;
                    fracture_force = force;
                    fracture_position = geometry.position;
                    max_impulse = impulse;
                }
            }

            let force_magnitude = max_impulse / time_step_duration;

            let mut created_fracture = false;

            for (object, collidable, force_threshold, force) in [
                (object_a, collidable_a, force_threshold_a, fracture_force),
                (object_b, collidable_b, force_threshold_b, -fracture_force),
            ] {
                if force_magnitude < force_threshold {
                    continue;
                }
                let Some((object_id, object)) = object else {
                    continue;
                };
                let Collidable::VoxelObject(collidable) = collidable.collidable() else {
                    panic!("Unexpected collidable for voxel object");
                };

                let force_direction = UnitVector3::unchecked_from(force / force_magnitude);

                let world_to_object_transform = collidable.transform_to_object_space().aligned();
                let world_to_norm_object_transform =
                    Similarity3::from_isometry(world_to_object_transform)
                        .scaled(object.inverse_voxel_extent());

                let aabb = object.compute_normalized_chunk_grid_bounds().compact();

                fracture_points.clear();

                let world_fracture_force = FractureForce {
                    position: fracture_position,
                    direction: force_direction,
                    magnitude: force_magnitude,
                };
                let local_fracture_force =
                    world_fracture_force.transformed(&world_to_norm_object_transform);

                fracture_point_generator.add_fracture_points(
                    &mut fracture_points,
                    &aabb,
                    &local_fracture_force,
                    rng,
                );

                if fracture_points.is_empty() {
                    continue;
                }

                let processing_direction = (-force_direction).compact();

                log::debug!(
                    "Fracturing {object_id}: force magnitude {force_magnitude:.5} \
                     exceeds threshold {force_threshold:.5}, \
                     direction = [{dx:.3}, {dy:.3}, {dz:.3}], \
                     {fracture_point_count} fracture point(s) generated",
                    dx = force_direction.x(),
                    dy = force_direction.y(),
                    dz = force_direction.z(),
                    fracture_point_count = fracture_points.len(),
                );

                self.add_fracture_points_for_object(
                    voxel_object_manager,
                    object_id,
                    &fracture_points,
                    Some(&processing_direction),
                )
                .unwrap();

                fractured_objects.insert(object_id);
                created_fracture = true;
            }

            if created_fracture {
                fracture_collision_entities.push([
                    collision.collidable_a_id.as_entity_id(),
                    collision.collidable_b_id.as_entity_id(),
                ]);
            }

            // TODO: If one or both got fracture points and both are voxel objects, enable mutual absorption and disable collision response for the pair.
        }

        for object_id in fractured_objects {
            self.initiate_fracturing_process(voxel_object_manager, object_id)
                .unwrap();
        }

        for entity_ids in fracture_collision_entities {
            constraint_manager.add_collision_to_ignore_list(entity_ids);
        }
    }
}

impl Default for VoxelFracturingConfig {
    fn default() -> Self {
        Self {
            max_processing_duration_us: None,
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
            fracture_objects: Vec::new(),
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
        assert!(self.fracture_objects.is_empty());

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
        assert!(self.fracture_objects.is_empty());

        self.tetrahedralization.reconstruct(&self.fracture_points)?;

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

        let n_generated_before = self.fracture_objects.len();
        let start_time = Instant::now();

        while let Some(dual_vertex_idx) = self.dual_vertex_queue.pop_front() {
            let buffers = voxel_object_buffer_pool.take_or_create_buffers();

            let result = Self::generate_fracture_object(
                voxel_type_registry,
                &self.tetrahedralization,
                voxel_object,
                &aabb,
                dual_vertex_idx,
                buffers,
                &mut polyhedron,
            );

            match result {
                FractureObjectGenerationResult::Generated(fracture_object) => {
                    self.fracture_objects.push(fracture_object);
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

        let n_generated_before = self.fracture_objects.len();
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
                                let result = Self::generate_fracture_object(
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
                            FractureObjectGenerationResult::Generated(fracture_object) => {
                                self.fracture_objects.push(fracture_object);
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
                        FractureObjectGenerationResult::Generated(fracture_object) => {
                            self.fracture_objects.push(fracture_object);
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

    fn generate_fracture_object<A: Allocator>(
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

                FractureObjectGenerationResult::Generated(FractureObject {
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
        let original_completed_count = self.fracture_objects.len();

        self.invalidate_required_completed_objects(voxel_object_buffer_pool, arena, voxel_object);

        // If invalidation happens faster than we can keep up, we are allowed to
        // exceed the time budget. We require that at least twice the number of
        // invalidated objects must be generated during each execution so that
        // we are guaranteed to finish.
        let invalidated_count = original_completed_count - self.fracture_objects.len();
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
            log::trace!(target: "contact_gen",
                "fracture invalidate chunk=[{},{},{}]",
                invalidated_chunk_indices[0], invalidated_chunk_indices[1], invalidated_chunk_indices[2]);
            // Find each completed object whose chunk ranges in the parent
            // contain the invalidated chunk and store the object's index
            for (object_idx, fracture_object) in self.fracture_objects.iter().enumerate() {
                let chunk_ranges = fracture_object.chunk_ranges_in_parent.iter();
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
                let fracture_object = self.fracture_objects.swap_remove(object_idx);

                log::trace!(target: "contact_gen",
                    "fracture requeue idx={} dual_vtx={}",
                    object_idx, fracture_object.dual_vertex_idx);

                voxel_object_buffer_pool
                    .add_buffers(fracture_object.meshed_voxel_object.into_buffers());

                self.dual_vertex_queue
                    .push_back(fracture_object.dual_vertex_idx);
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
        let n_generated = self.fracture_objects.len() - n_generated_before;

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

        let original_position = rigid_body.position().aligned();
        let orientation = rigid_body.orientation().aligned();
        let original_linear_velocity = rigid_body.compute_velocity();
        let angular_velocity = rigid_body.compute_angular_velocity();

        let entity_ids = entity_id_manager.provide_id_vec(self.fracture_objects.len());

        for (i, obj) in self.fracture_objects.iter().enumerate() {
            log::trace!(target: "contact_gen",
                "fracture spawn slot={} dual_vtx={} offset=[{},{},{}]",
                i, obj.dual_vertex_idx,
                obj.origin_offset_in_parent[0], obj.origin_offset_in_parent[1], obj.origin_offset_in_parent[2]);
        }

        for (&entity_id, mut fracture_object) in
            entity_ids.iter().zip(self.fracture_objects.drain(..))
        {
            let voxel_object = fracture_object.meshed_voxel_object.object();

            let dynamics = interaction::determine_extracted_voxel_object_dynamics(
                voxel_object,
                fracture_object.origin_offset_in_parent,
                &mut fracture_object.inertial_property_manager,
                original_local_center_of_mass,
                original_position,
                orientation,
                original_linear_velocity,
                angular_velocity,
            );

            let anchors = interaction::get_anchors_on_extracted_voxel_object(
                anchor_manager,
                original_rigid_body_id,
                voxel_object,
                &dynamics.coordinate_changes,
            );

            let extracted_components = ExtractedComponents {
                meshed_voxel_object: fracture_object.meshed_voxel_object,
                inertial_property_manager: fracture_object.inertial_property_manager,
                rigid_body: dynamics.rigid_body,
                anchors,
            };

            interaction::spawn_extracted_voxel_object(
                voxel_object_manager,
                rigid_body_manager,
                anchor_manager,
                extracted_components,
                entity_id,
            );
        }

        context.create_extracted_voxel_object_entities(entity_ids, original_entity_id);
        context.remove_voxel_object_entity(original_entity_id);

        self.reset(voxel_object_buffer_pool);
    }

    fn reset(&mut self, voxel_object_buffer_pool: &mut VoxelObjectBufferPool) {
        self.fracture_points.clear();
        self.processing_direction = None;
        self.dual_vertex_queue.clear();
        self.reclaim_fracture_object_buffers(voxel_object_buffer_pool);
        self.state = FracturingProcessState::Idle;
    }

    fn reclaim_fracture_object_buffers(
        &mut self,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
    ) {
        for fracture_object in self.fracture_objects.drain(..) {
            let buffers = fracture_object.meshed_voxel_object.into_buffers();
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

impl ImpulseFracturePointGenerator {
    pub fn new() -> Self {
        Self {}
    }

    fn add_fracture_points<A: Allocator>(
        &self,
        points: &mut AVec<Point3C, A>,
        aabb: &AxisAlignedBoxC,
        force: &FractureForce,
        rng: &mut Rng,
    ) {
        RandomizedGridFracturePointGenerator::new(5).add_fracture_points(points, aabb, rng);
    }
}

impl FractureForce {
    fn transformed(&self, transform: &Similarity3) -> Self {
        Self {
            position: transform.transform_point(&self.position),
            direction: transform.transform_unit_vector(&self.direction),
            magnitude: self.magnitude,
        }
    }
}
