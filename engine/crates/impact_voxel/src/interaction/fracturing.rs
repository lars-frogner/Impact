//! Voxel object fracturing.

use crate::{
    VoxelObjectID, VoxelObjectManager,
    object::inertia::VoxelObjectInertialPropertyManager,
    interaction::{self, ExtractedComponents, VoxelObjectInteractionContext},
    voxel_types::VoxelTypeRegistry,
};
use anyhow::{Result, anyhow, bail};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::{HashMap, hash_map::Entry};
use impact_geometry::AxisAlignedBoxC;
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
use std::ops::Range;

/// Manages voxel object fracturing processes and state.
#[derive(Debug)]
pub struct VoxelObjectFracturingManager {
    processes: HashMap<VoxelObjectID, FracturingProcess>,
}

#[derive(Debug)]
pub struct FracturingProcess {
    voxel_object_id: VoxelObjectID,
    tetrahedralization: DelaunayTetrahedralization,
    processed_vertex_count: usize,
    parts: Vec<ExtractedComponents>,
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
    pub fn new() -> Self {
        Self {
            processes: HashMap::default(),
        }
    }

    pub fn initiate_fracturing_process(
        &mut self,
        voxel_object_manager: &VoxelObjectManager,
        voxel_object_id: VoxelObjectID,
        fracture_points: &[Point3C],
    ) -> Result<()> {
        match self.processes.entry(voxel_object_id) {
            Entry::Vacant(entry) => {
                if !voxel_object_manager.has_voxel_object(voxel_object_id) {
                    bail!(
                        "Tried to initiate fracturing for missing voxel object {voxel_object_id}"
                    );
                }
                let process = FracturingProcess::initiate(voxel_object_id, fracture_points)?;
                entry.insert(process);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Fracturing is already in progress for voxel object {voxel_object_id}"
            )),
        }
    }

    pub fn execute_fracturing_processes<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
    ) where
        C: VoxelObjectInteractionContext,
    {
        for process in self.processes.values_mut() {
            process.execute(
                voxel_type_registry,
                voxel_object_manager,
                rigid_body_manager,
                anchor_manager,
            );
        }
        self.processes.retain(|_, process| {
            if !process.is_complete() {
                return true;
            }
            process.complete(
                context,
                entity_id_manager,
                voxel_object_manager,
                rigid_body_manager,
                anchor_manager,
            );
            false
        });
    }
}

impl FracturingProcess {
    fn initiate(voxel_object_id: VoxelObjectID, fracture_points: &[Point3C]) -> Result<Self> {
        let tetrahedralization = DelaunayTetrahedralization::construct(fracture_points)?;
        Ok(Self {
            voxel_object_id,
            tetrahedralization,
            processed_vertex_count: 0,
            parts: Vec::new(),
        })
    }

    fn execute(
        &mut self,
        voxel_type_registry: &VoxelTypeRegistry,
        voxel_object_manager: &VoxelObjectManager,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) {
        if self.is_complete() {
            return;
        }

        let entity_id = self.voxel_object_id.as_entity_id();
        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);

        let Some(meshed_voxel_object) = voxel_object_manager.get_voxel_object(self.voxel_object_id)
        else {
            self.mark_complete();
            return;
        };
        let Some(physics_context) = voxel_object_manager.get_physics_context(self.voxel_object_id)
        else {
            self.mark_complete();
            return;
        };
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(rigid_body_id) else {
            self.mark_complete();
            return;
        };

        let voxel_object = meshed_voxel_object.object();

        let arena = ArenaPool::get_arena();
        let mut polyhedron = VoronoiPolyhedron::empty_in(&arena);

        let aabb = voxel_object.compute_normalized_chunk_grid_bounds();

        let original_local_center_of_mass = physics_context
            .inertial_property_manager
            .derive_center_of_mass();

        let original_position = rigid_body.position().aligned();
        let orientation = rigid_body.orientation().aligned();
        let original_linear_velocity = rigid_body.compute_velocity();
        let angular_velocity = rigid_body.compute_angular_velocity();

        for dual_vertex_idx in self.remaining_vertex_indices() {
            self.processed_vertex_count += 1;

            polyhedron.extract_from_delaunay_tetrahedra(&self.tetrahedralization, dual_vertex_idx);
            let Some(polyhedron_aabb) = polyhedron.compute_bounded_aabb(&aabb) else {
                continue;
            };

            let mut poly_inertial_property_manager = VoxelObjectInertialPropertyManager::zeroed();

            let mut inertial_property_copier = poly_inertial_property_manager.begin_computation(
                voxel_object.voxel_extent(),
                voxel_type_registry.mass_densities(),
            );

            let Some(poly_object) = voxel_object.copy_polyhedron_with_property_computer(
                &polyhedron_aabb,
                &polyhedron.face_planes,
                &mut inertial_property_copier,
            ) else {
                continue;
            };

            let dynamic_poly_object = interaction::determine_extracted_voxel_object_dynamics(
                poly_object,
                poly_inertial_property_manager,
                original_local_center_of_mass,
                original_position,
                orientation,
                original_linear_velocity,
                angular_velocity,
            );

            let anchors = interaction::get_anchors_on_extracted_voxel_object(
                anchor_manager,
                rigid_body_id,
                &dynamic_poly_object.voxel_object,
                &dynamic_poly_object.coordinate_changes,
            );

            self.parts.push(ExtractedComponents {
                object: dynamic_poly_object,
                anchors,
            });
        }
    }

    fn complete<C>(
        &mut self,
        context: &mut C,
        entity_id_manager: &mut EntityIDManager,
        voxel_object_manager: &mut VoxelObjectManager,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &mut AnchorManager,
    ) where
        C: VoxelObjectInteractionContext,
    {
        if !self.is_complete() {
            return;
        }

        let entity_id = self.voxel_object_id.as_entity_id();
        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);

        if !voxel_object_manager.has_voxel_object(self.voxel_object_id)
            || !rigid_body_manager.has_dynamic_rigid_body(rigid_body_id)
        {
            self.parts.clear();
            return;
        }

        for part in self.parts.drain(..) {
            interaction::spawn_extracted_voxel_object(
                context,
                entity_id_manager,
                voxel_object_manager,
                rigid_body_manager,
                anchor_manager,
                part,
                entity_id,
            );
        }
        context.remove_voxel_object_entity(entity_id);
    }

    fn is_complete(&mut self) -> bool {
        self.processed_vertex_count == self.tetrahedralization.internal_vertex_indices().len()
    }

    fn mark_complete(&mut self) {
        self.processed_vertex_count = self.tetrahedralization.internal_vertex_indices().len();
    }

    fn remaining_vertex_indices(&self) -> Range<VertexIdx> {
        let indices = self.tetrahedralization.internal_vertex_indices();
        let skip = self.processed_vertex_count as VertexIdx;
        (indices.start + skip)..indices.end
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
