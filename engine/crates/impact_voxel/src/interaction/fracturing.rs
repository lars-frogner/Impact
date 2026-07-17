//! Voxel object fracturing.

use crate::{
    VoxelObjectBufferPool, VoxelObjectID, VoxelObjectManager,
    interaction::{self, ExtractedComponents, VoxelObjectInteractionContext},
    mesh::{MeshedVoxelObject, MeshedVoxelObjectBuffers},
    object::{
        ChunkRanges, VoxelObject, extraction::ExtractionResult,
        inertia::VoxelObjectInertialPropertyManager,
    },
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Result, anyhow, bail};
use impact_alloc::{
    AVec, Allocator,
    arena::{ArenaPool, PoolArena},
};
use impact_containers::{HashMap, hash_map::Entry};
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_id::EntityIDManager;
use impact_math::{point::Point3C, random::Rng, vector::Vector3C};
use impact_physics::{
    anchor::AnchorManager,
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
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Manages voxel object fracturing processes and state.
#[derive(Debug)]
pub struct VoxelObjectFracturingManager {
    ongoing_processes: HashMap<VoxelObjectID, FracturingProcess>,
    completed_processes: Vec<FracturingProcess>,
    config: VoxelFracturingConfig,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct VoxelFracturingConfig {}

#[derive(Debug)]
struct FracturingProcess {
    tetrahedralization: DelaunayTetrahedralization,
    dual_vertex_queue: VecDeque<VertexIdx>,
    fracture_objects: Vec<FractureObject>,
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

impl VoxelObjectFracturingManager {
    /// Creates a new empty fracturing manager with the given configuration.
    pub fn new(config: VoxelFracturingConfig) -> Self {
        Self {
            ongoing_processes: HashMap::default(),
            completed_processes: Vec::new(),
            config,
        }
    }

    /// Stages the given voxel object for fracturing based on the given fracture
    /// points. The actual processing will not happen until
    /// [`Self::execute_fracturing_processes`] is called.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The voxel object does not exist.
    /// - Fracturing has already been initiated for the object.
    pub fn initiate_fracturing_process(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_id: VoxelObjectID,
        fracture_points: &[Point3C],
    ) -> Result<()> {
        match self.ongoing_processes.entry(voxel_object_id) {
            Entry::Vacant(entry) => {
                if !voxel_object_manager.has_voxel_object(voxel_object_id) {
                    bail!(
                        "Tried to initiate fracturing for missing voxel object {voxel_object_id}"
                    );
                }
                let mut process = self
                    .completed_processes
                    .pop()
                    .unwrap_or_else(FracturingProcess::new);

                log::debug!("Initiating fracturing for voxel object: {voxel_object_id}");
                process.initiate(fracture_points)?;

                entry.insert(process);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Fracturing is already in progress for voxel object {voxel_object_id}"
            )),
        }
    }

    /// Executes all initiated fracturing processes.
    ///
    /// If a `max_duration` is given, the processing time will be attempted
    /// limited to that time. The time budget may be exceeded to spawn the
    /// fracture objects for completed processes and to make sure all processes
    /// make enough progress to counter the rate of invalidation due to objects
    /// being modified.
    pub fn execute_fracturing_processes<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        max_duration: Option<Duration>,
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
            max_duration,
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
    ///
    /// If a `max_duration` is given, the processing time will be attempted
    /// limited to that time. The time budget may be exceeded to spawn the
    /// fracture objects for completed processes and to make sure all processes
    /// make enough progress to counter the rate of invalidation due to objects
    /// being modified.
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
        max_duration: Option<Duration>,
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
            max_duration,
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

    fn execute_fracturing_processes_with_closure<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        voxel_object_buffer_pool: &mut VoxelObjectBufferPool,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
        max_duration: Option<Duration>,
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
        let mut completed_voxel_object_ids = AVec::new_in(&arena);

        let mut remaining_duration = max_duration.unwrap_or(Duration::MAX);

        for (&voxel_object_id, process) in &mut self.ongoing_processes {
            let start_time = Instant::now();

            execute_process(
                voxel_object_manager,
                voxel_object_buffer_pool,
                rigid_body_manager,
                process,
                voxel_object_id,
                remaining_duration,
            );

            if process.is_complete() {
                completed_voxel_object_ids.push(voxel_object_id);
            }

            // We don't break when the remaining duration reaches zero, because
            // we need to allow every process to regenerate enough of their
            // invalidated objects
            remaining_duration = remaining_duration.saturating_sub(start_time.elapsed());
        }

        for voxel_object_id in completed_voxel_object_ids {
            let mut process = self.ongoing_processes.remove(&voxel_object_id).unwrap();

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

            self.completed_processes.push(process);
        }
    }
}

impl Default for VoxelFracturingConfig {
    fn default() -> Self {
        Self {}
    }
}

impl FracturingProcess {
    fn new() -> Self {
        Self {
            tetrahedralization: DelaunayTetrahedralization::new(),
            dual_vertex_queue: VecDeque::new(),
            fracture_objects: Vec::new(),
        }
    }

    fn is_complete(&self) -> bool {
        self.dual_vertex_queue.is_empty()
    }

    fn initiate(&mut self, fracture_points: &[Point3C]) -> Result<()> {
        assert!(self.dual_vertex_queue.is_empty());
        assert!(self.fracture_objects.is_empty());

        self.tetrahedralization.reconstruct(fracture_points)?;

        self.dual_vertex_queue
            .extend(self.tetrahedralization.internal_vertex_indices());

        Ok(())
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
        if self.is_complete() {
            return;
        }

        let Some(voxel_object) = Self::get_voxel_object_for_execution(
            voxel_object_manager,
            rigid_body_manager,
            voxel_object_id,
        ) else {
            self.reset(voxel_object_buffer_pool);
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
        if self.is_complete() {
            return;
        }

        let Some(voxel_object) = Self::get_voxel_object_for_execution(
            voxel_object_manager,
            rigid_body_manager,
            voxel_object_id,
        ) else {
            self.reset(voxel_object_buffer_pool);
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
            self.reset(voxel_object_buffer_pool);
            return;
        };
        let Some(physics_context) =
            voxel_object_manager.get_physics_context(original_voxel_object_id)
        else {
            log::warn!(
                "Tried to execute fracturing for voxel object {original_voxel_object_id} \
                 with missing physics context"
            );
            self.reset(voxel_object_buffer_pool);
            return;
        };
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(original_rigid_body_id)
        else {
            log::warn!(
                "Tried to execute fracturing for voxel object {original_voxel_object_id} \
                 with missing rigid body"
            );
            self.reset(voxel_object_buffer_pool);
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
        self.dual_vertex_queue.clear();
        self.reclaim_fracture_object_buffers(voxel_object_buffer_pool);
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
    pub fn generate_fracture_points<A: Allocator>(
        &self,
        alloc: A,
        aabb: &AxisAlignedBoxC,
        seed: u64,
    ) -> AVec<Point3C, A> {
        let mut rng = Rng::with_seed(seed);
        match self {
            Self::RandomizedGrid(seeder) => seeder.generate_fracture_points(alloc, aabb, &mut rng),
        }
    }
}

impl RandomizedGridFracturePointGenerator {
    pub fn new(points_per_dim: usize) -> Self {
        assert_ne!(points_per_dim, 0);
        Self { points_per_dim }
    }

    pub fn generate_fracture_points<A: Allocator>(
        &self,
        alloc: A,
        aabb: &AxisAlignedBoxC,
        rng: &mut Rng,
    ) -> AVec<Point3C, A> {
        let start = aabb.lower_corner();
        let scale = aabb.extents() / (self.points_per_dim as f32);

        let mut points = AVec::with_capacity_in(self.points_per_dim.pow(3), alloc);

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

        points
    }
}
