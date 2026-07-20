//! Implementation of [`Collidable`](collision::Collidable) that includes voxel
//! geometry.

pub mod setup;

#[cfg(feature = "ecs")]
pub mod systems;

use crate::{
    Voxel, VoxelObjectID, VoxelObjectManager, VoxelSignedDistance, VoxelSurfacePlacement,
    mesh::{
        MeshedVoxelObject, VoxelMeshIndex, VoxelMeshVertexNormalVector, VoxelMeshVertexPosition,
        VoxelObjectMesh,
    },
    object::{
        self, CHUNK_SIZE, LOG2_CHUNK_SIZE, VoxelChunk, VoxelObject,
        chunk_range_encompassing_voxel_range, inertia::VoxelObjectInertialPropertyManager, sdf,
    },
};
use impact_alloc::{AVec, Allocator, Global, arena::ArenaPool, avec};
use impact_containers::{HashMap, RangeAllocator};
use impact_geometry::{Capsule, Plane, Sphere};
use impact_id::EntityID;
use impact_math::{
    consts::f32::SQRT_3,
    point::Point3C,
    transform::{Isometry3, Isometry3C},
    vector::{UnitVector3, UnitVector3C, Vector3, Vector3C, Vector4C},
};
use impact_physics::{
    collision::{
        self, CollidableDescriptor, CollidableID, CollidableOrder, CollidableWithId,
        collidable::{
            capsule::{
                CapsuleCollidable, determine_capsule_sphere_contact_geometry,
                generate_capsule_capsule_contact_manifold, generate_capsule_plane_contact_manifold,
                generate_capsule_sphere_contact_manifold,
            },
            contact_id_from_collidable_ids_and_indices,
            plane::PlaneCollidable,
            sphere::{
                SphereCollidable, determine_sphere_plane_contact_geometry,
                determine_sphere_sphere_contact_geometry, generate_sphere_plane_contact_manifold,
                generate_sphere_sphere_contact_manifold,
            },
        },
    },
    constraint::contact::{Contact, ContactGeometry, ContactManifold, ContactWithID},
    material::ContactResponseParameters,
};
use std::ops::Range;

pub type CollisionWorld = collision::CollisionWorld<Collidable>;

#[derive(Clone, Debug)]
pub enum Collidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
    VoxelObject(VoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub enum LocalCollidable {
    Sphere(SphereCollidable),
    Plane(PlaneCollidable),
    Capsule(CapsuleCollidable),
    VoxelObject(LocalVoxelObjectCollidable),
}

#[derive(Clone, Debug)]
pub struct LocalVoxelObjectCollidable {
    entity_id: EntityID,
    response_params: ContactResponseParameters,
    origin_offset: Vector3C,
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidable {
    entity_id: EntityID,
    response_params: ContactResponseParameters,
    transform_to_object_space: Isometry3C,
}

/// Specially selected points on the voxel object surface to use for probing the
/// other object's SDF during mutual voxel object collision detection and
/// contact generation.
///
/// The points are a subset of the object's mesh vertices and are determined for
/// each chunk separately. The vertices for a chunk are spatially grouped by
/// dividing the chunk into blocks of 4³ voxels (or 2³ or 1³ if the object is
/// small). Within each block, the vertex with the largest convex curvature (as
/// determined by a cheap heuristic) is selected as the probe. This approach is
/// intended to yield a relatively small number of distributed points focused on
/// the protruding parts of the mesh.
#[derive(Clone, Debug)]
pub struct VoxelObjectCollisionProbes {
    chunk_point_ranges: HashMap<[usize; 3], Range<usize>>,
    probe_points: AVec<Point3C, Global>,
    point_range_allocator: RangeAllocator,
}

impl collision::Collidable for Collidable {
    type Local = LocalCollidable;
    type Context = VoxelObjectManager;

    fn from_descriptor(
        descriptor: &CollidableDescriptor<Self>,
        transform_to_world_space: &Isometry3,
    ) -> Self {
        match descriptor.local_collidable() {
            Self::Local::Sphere(sphere) => {
                Self::Sphere(sphere.transformed(transform_to_world_space))
            }
            Self::Local::Plane(plane) => Self::Plane(plane.transformed(transform_to_world_space)),
            Self::Local::Capsule(capsule) => {
                Self::Capsule(capsule.transformed(transform_to_world_space))
            }
            Self::Local::VoxelObject(voxel_object) => {
                Self::VoxelObject(VoxelObjectCollidable::new(
                    voxel_object.entity_id,
                    voxel_object.response_params,
                    voxel_object.origin_offset.aligned(),
                    transform_to_world_space,
                ))
            }
        }
    }

    fn generate_contact_manifold(
        voxel_object_manager: &VoxelObjectManager,
        collidable_a: &CollidableWithId<Self>,
        collidable_b: &CollidableWithId<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder {
        use Collidable::{Capsule, Plane, Sphere, VoxelObject};

        match (collidable_a.collidable(), collidable_b.collidable()) {
            (VoxelObject(voxel_object_a), VoxelObject(voxel_object_b)) => {
                generate_mutual_voxel_object_contact_manifold(
                    voxel_object_manager,
                    voxel_object_a,
                    voxel_object_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Capsule(capsule), VoxelObject(voxel_object)) => {
                generate_capsule_voxel_object_contact_manifold(
                    voxel_object_manager,
                    capsule,
                    voxel_object,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (VoxelObject(voxel_object), Capsule(capsule)) => {
                generate_capsule_voxel_object_contact_manifold(
                    voxel_object_manager,
                    capsule,
                    voxel_object,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Sphere(sphere), VoxelObject(voxel_object)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (VoxelObject(voxel_object), Sphere(sphere)) => {
                generate_sphere_voxel_object_contact_manifold(
                    voxel_object_manager,
                    sphere,
                    voxel_object,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (VoxelObject(voxel_object), Plane(plane)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), VoxelObject(voxel_object)) => {
                generate_voxel_object_plane_contact_manifold(
                    voxel_object_manager,
                    voxel_object,
                    plane,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Capsule(capsule_a), Capsule(capsule_b)) => {
                generate_capsule_capsule_contact_manifold(
                    capsule_a,
                    capsule_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Capsule(capsule), Sphere(sphere)) => {
                generate_capsule_sphere_contact_manifold(
                    capsule,
                    sphere,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), Capsule(capsule)) => {
                generate_capsule_sphere_contact_manifold(
                    capsule,
                    sphere,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Capsule(capsule), Plane(plane)) => {
                generate_capsule_plane_contact_manifold(
                    capsule,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), Capsule(capsule)) => {
                generate_capsule_plane_contact_manifold(
                    capsule,
                    plane,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Sphere(sphere_a), Sphere(sphere_b)) => {
                generate_sphere_sphere_contact_manifold(
                    sphere_a,
                    sphere_b,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Sphere(sphere), Plane(plane)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_a.id(),
                    collidable_b.id(),
                    contact_manifold,
                );
                CollidableOrder::Original
            }
            (Plane(plane), Sphere(sphere)) => {
                generate_sphere_plane_contact_manifold(
                    sphere,
                    plane,
                    collidable_b.id(),
                    collidable_a.id(),
                    contact_manifold,
                );
                CollidableOrder::Swapped
            }
            (Plane(_), Plane(_)) => {
                // Not useful
                CollidableOrder::Original
            }
        }
    }
}

impl LocalVoxelObjectCollidable {
    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

impl VoxelObjectCollidable {
    pub fn new(
        entity_id: EntityID,
        response_params: ContactResponseParameters,
        origin_offset: Vector3,
        transform_to_world_space: &Isometry3,
    ) -> Self {
        let transform_from_object_to_world_space =
            transform_to_world_space.applied_to_translation(&(-origin_offset));

        let transform_to_object_space = transform_from_object_to_world_space.inverted();

        Self {
            entity_id,
            response_params,
            transform_to_object_space: transform_to_object_space.compact(),
        }
    }

    pub fn entity_id(&self) -> EntityID {
        self.entity_id
    }

    pub fn transform_to_object_space(&self) -> &Isometry3C {
        &self.transform_to_object_space
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockSize {
    One,
    Two,
    Four,
    Eight,
}

impl VoxelObjectCollisionProbes {
    pub fn new() -> Self {
        Self {
            chunk_point_ranges: HashMap::default(),
            probe_points: AVec::new(),
            point_range_allocator: RangeAllocator::fully_occupied(),
        }
    }

    pub fn compute_for_all_chunks(object: &VoxelObject, mesh: &VoxelObjectMesh) -> Self {
        let mut probes = Self::new();
        probes.recompute_for_all_chunks(object, mesh);
        probes
    }

    pub fn recompute_for_all_chunks(&mut self, object: &VoxelObject, mesh: &VoxelObjectMesh) {
        match Self::determine_log2_block_size_for_object(object) {
            BlockSize::One => {
                const LOG2_BLOCK_SIZE: usize = 0;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.recompute_for_all_chunks_with_block_size::<LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                    object, mesh,
                );
            }
            BlockSize::Two => {
                const LOG2_BLOCK_SIZE: usize = 1;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.recompute_for_all_chunks_with_block_size::<LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                    object, mesh,
                );
            }
            BlockSize::Four => {
                const LOG2_BLOCK_SIZE: usize = 2;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.recompute_for_all_chunks_with_block_size::<LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                    object, mesh,
                );
            }
            BlockSize::Eight => {
                const LOG2_BLOCK_SIZE: usize = 3;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.recompute_for_all_chunks_with_block_size::<LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                    object, mesh,
                );
            }
        }
    }

    pub fn sync_with_voxel_object_and_mesh(
        &mut self,
        object: &VoxelObject,
        mesh: &VoxelObjectMesh,
    ) {
        match Self::determine_log2_block_size_for_object(object) {
            BlockSize::One => {
                const LOG2_BLOCK_SIZE: usize = 0;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.sync_with_voxel_object_and_mesh_with_block_size::<
                    LOG2_BLOCK_SIZE,
                    CHUNK_BLOCK_COUNT,
                >(object, mesh);
            }
            BlockSize::Two => {
                const LOG2_BLOCK_SIZE: usize = 1;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.sync_with_voxel_object_and_mesh_with_block_size::<
                    LOG2_BLOCK_SIZE,
                    CHUNK_BLOCK_COUNT,
                >(object, mesh);
            }
            BlockSize::Four => {
                const LOG2_BLOCK_SIZE: usize = 2;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.sync_with_voxel_object_and_mesh_with_block_size::<
                    LOG2_BLOCK_SIZE,
                    CHUNK_BLOCK_COUNT,
                >(object, mesh);
            }
            BlockSize::Eight => {
                const LOG2_BLOCK_SIZE: usize = 3;
                const CHUNK_BLOCK_COUNT: usize = chunk_block_count(LOG2_BLOCK_SIZE);
                self.sync_with_voxel_object_and_mesh_with_block_size::<
                    LOG2_BLOCK_SIZE,
                    CHUNK_BLOCK_COUNT,
                >(object, mesh);
            }
        }
    }

    pub fn clear(&mut self) {
        self.chunk_point_ranges.clear();
        self.probe_points.clear();
        self.point_range_allocator.mark_all_ranges_occupied();
    }

    pub fn chunk_point_ranges(&self) -> impl ExactSizeIterator<Item = (&[usize; 3], Range<usize>)> {
        self.chunk_point_ranges
            .iter()
            .map(|(chunk_indices, range)| (chunk_indices, range.clone()))
    }

    pub fn probe_points(&self) -> &[Point3C] {
        &self.probe_points
    }

    fn determine_log2_block_size_for_object(object: &VoxelObject) -> BlockSize {
        let min_extent_in_voxels = object
            .occupied_voxel_ranges()
            .iter()
            .map(Range::len)
            .min()
            .unwrap();

        // We want the largest block size we can get away with without missing
        // mesh features. The worst outcome if the block size is too large is
        // that only one side of a thin object gets collision probes.
        if min_extent_in_voxels >= 2 * 8 {
            BlockSize::Eight
        } else if min_extent_in_voxels >= 2 * 4 {
            BlockSize::Four
        } else if min_extent_in_voxels >= 2 * 2 {
            BlockSize::Two
        } else {
            BlockSize::One
        }
    }

    fn recompute_for_all_chunks_with_block_size<
        const LOG2_BLOCK_SIZE: usize,
        const CHUNK_BLOCK_COUNT: usize,
    >(
        &mut self,
        object: &VoxelObject,
        mesh: &VoxelObjectMesh,
    ) {
        self.clear();

        self.chunk_point_ranges.reserve(mesh.n_chunks());
        self.probe_points
            .reserve(mesh.n_chunks() * CHUNK_BLOCK_COUNT);

        for (submesh, vertex_range) in mesh
            .chunk_submeshes()
            .iter()
            .zip(mesh.chunk_vertex_ranges())
        {
            let point_range_start = self.probe_points.len();

            let index_range = submesh.index_range();

            let chunk_indices = submesh.chunk_indices().map(|idx| idx as usize);

            let vertex_positions = &mesh.positions()[vertex_range.clone()];
            let vertex_normals = &mesh.normal_vectors()[vertex_range.clone()];
            let start_index = vertex_range.start as u32;
            let indices = &mesh.indices()[index_range];

            Self::add_points_for_vertices_in_blocks::<_, LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                &mut self.probe_points,
                &chunk_indices,
                vertex_positions,
                vertex_normals,
                indices,
                start_index,
                object.inverse_voxel_extent(),
            );

            let point_range_end = self.probe_points.len();
            let point_range = point_range_start..point_range_end;

            if point_range.is_empty() {
                continue;
            }

            self.chunk_point_ranges.insert(chunk_indices, point_range);
        }
    }

    fn sync_with_voxel_object_and_mesh_with_block_size<
        const LOG2_BLOCK_SIZE: usize,
        const CHUNK_BLOCK_COUNT: usize,
    >(
        &mut self,
        object: &VoxelObject,
        mesh: &VoxelObjectMesh,
    ) {
        for chunk_indices in object.invalidated_mesh_chunk_indices() {
            self.update_for_chunk::<LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
                object,
                mesh,
                *chunk_indices,
            );
        }
        self.point_range_allocator.merge_consecutive_ranges();
    }

    fn update_for_chunk<const LOG2_BLOCK_SIZE: usize, const CHUNK_BLOCK_COUNT: usize>(
        &mut self,
        object: &VoxelObject,
        mesh: &VoxelObjectMesh,
        chunk_indices: [usize; 3],
    ) {
        let Some((vertex_range, index_range)) =
            mesh.vertex_and_index_range_for_chunk_at_indices(chunk_indices)
        else {
            // If the chunk doesn't have vertices, remove it
            if let Some(removed_range) = self.chunk_point_ranges.remove(&chunk_indices) {
                // Free its point range if it was present
                self.point_range_allocator.free_range(&removed_range);
            }
            return;
        };

        let arena = ArenaPool::get_arena();
        let mut point_buffer = AVec::with_capacity_in(CHUNK_BLOCK_COUNT, &arena);

        let vertex_positions = &mesh.positions()[vertex_range.clone()];
        let vertex_normals = &mesh.normal_vectors()[vertex_range.clone()];
        let start_index = vertex_range.start as u32;
        let indices = &mesh.indices()[index_range];

        Self::add_points_for_vertices_in_blocks::<_, LOG2_BLOCK_SIZE, CHUNK_BLOCK_COUNT>(
            &mut point_buffer,
            &chunk_indices,
            vertex_positions,
            vertex_normals,
            indices,
            start_index,
            object.inverse_voxel_extent(),
        );

        // If there were no points, abort after removing the chunk and freeing
        // its range
        if point_buffer.is_empty() {
            if let Some(old_point_range) = self.chunk_point_ranges.remove(&chunk_indices) {
                self.point_range_allocator.free_range(&old_point_range);
            }
            return;
        }

        // Free the old point range if we had points for the chunk
        if let Some(old_point_range) = self.chunk_point_ranges.get(&chunk_indices) {
            self.point_range_allocator.free_range(old_point_range);
        }

        // Look for a free range for the new points
        if let Some(point_range) = self
            .point_range_allocator
            .allocate_range(point_buffer.len())
        {
            // If we found a free range, store it for the chunk and copy in the
            // new points
            self.chunk_point_ranges
                .insert(chunk_indices, point_range.clone());

            self.probe_points[point_range].copy_from_slice(&point_buffer);
        } else {
            // If there was no free range, append the new points to the end
            let point_range =
                self.probe_points.len()..(self.probe_points.len() + point_buffer.len());

            self.chunk_point_ranges
                .insert(chunk_indices, point_range.clone());

            self.probe_points.extend_from_slice(&point_buffer);
        }
    }

    fn add_points_for_vertices_in_blocks<
        A: Allocator,
        const LOG2_BLOCK_SIZE: usize,
        const CHUNK_BLOCK_COUNT: usize,
    >(
        points: &mut AVec<Point3C, A>,
        chunk_indices: &[usize; 3],
        vertex_positions: &[VoxelMeshVertexPosition],
        vertex_normals: &[VoxelMeshVertexNormalVector],
        indices: &[VoxelMeshIndex],
        start_index: u32,
        inverse_voxel_extent: f32,
    ) {
        let arena = ArenaPool::get_arena();

        // Running sum and count of curvature samples for each vertex
        let mut vertex_curvatures = avec![in &arena; [0.0, 0.0]; vertex_positions.len()];

        let (triangles, []) = indices.as_chunks::<3>() else {
            panic!("Indices were not divisible into triangles");
        };

        // For each triangle, add a curvature sample for each of the three
        // vertices. We produce a curvature sample for a vertex by projecting
        // its normal along an outgoing edge. If negative, the edge has a
        // component opposite the normal direction, so the curvature is convex.
        // Each vertex in the triangle get two samples, since they are connected
        // to two triangle edges.
        //
        // Note that since we treat each triangle in isolation, edges shared
        // between triangles will yield two equal samples to each connected
        // vertex. This will make interior vertices biased differently from
        // boundary vertices, but the extra cost of detecting shared edges is
        // not worth it. This is only supposed to be a cheap heuristic.
        for &[i0, i1, i2] in triangles {
            let i0 = (i0.0 - start_index) as usize;
            let i1 = (i1.0 - start_index) as usize;
            let i2 = (i2.0 - start_index) as usize;

            let v0 = Point3C::from(vertex_positions[i0].0);
            let v1 = Point3C::from(vertex_positions[i1].0);
            let v2 = Point3C::from(vertex_positions[i2].0);

            let n0 = UnitVector3C::unchecked_from(vertex_normals[i0].0.into());
            let n1 = UnitVector3C::unchecked_from(vertex_normals[i1].0.into());
            let n2 = UnitVector3C::unchecked_from(vertex_normals[i2].0.into());

            let edge_01 = v1 - v0;
            let edge_12 = v2 - v1;
            let edge_20 = v0 - v2;

            let [sum, count] = &mut vertex_curvatures[i0];
            *sum += n0.dot(&edge_01) - n0.dot(&edge_20);
            *count += 2.0;

            let [sum, count] = &mut vertex_curvatures[i1];
            *sum += n1.dot(&edge_12) - n1.dot(&edge_01);
            *count += 2.0;

            let [sum, count] = &mut vertex_curvatures[i2];
            *sum += n2.dot(&edge_20) - n2.dot(&edge_12);
            *count += 2.0;
        }

        let norm_chunk_aabb =
            object::normalized_chunk_aabb_from_chunk_indices(chunk_indices).compact();

        // Running best point and associated minimum (most convex) curvature for
        // each block in the chunk. The point and curvature are packed into a
        // Vector4 to keep them adjacent in memory.
        let mut best_points_and_curvatures =
            avec![in &arena; Vector4C::same(f32::INFINITY); CHUNK_BLOCK_COUNT];

        for (vertex_position, [curvature_sum, curvature_count]) in
            vertex_positions.iter().zip(vertex_curvatures)
        {
            // There may be vertices not connected to any edges. We ignore
            // those.
            if curvature_count == 0.0 {
                continue;
            }

            let position = Point3C::from(vertex_position.0);
            let norm_position = position * inverse_voxel_extent;

            // Vertices may be slightly outside the chunk, so we clamp the
            // position to the chunk for the purpose of determining the block
            let clamped_norm_position = norm_position
                .max_with(norm_chunk_aabb.lower_corner())
                .min_with(norm_chunk_aabb.upper_corner());

            let object_voxel_indices =
                <[f32; 3]>::from(clamped_norm_position).map(|idx| idx as usize);

            let block_idx = Self::linear_block_idx_from_object_voxel_indices(
                LOG2_BLOCK_SIZE,
                object_voxel_indices,
            );

            let curvature = curvature_sum / curvature_count;

            let min_curvature = best_points_and_curvatures[block_idx].w();

            // Keep the point with the minimum curvature
            if curvature < min_curvature {
                best_points_and_curvatures[block_idx] =
                    Vector4C::new(position.x(), position.y(), position.z(), curvature);
            }
        }

        // Gather the best vertex (if present) from each block
        for best_point_and_curvature in best_points_and_curvatures {
            if best_point_and_curvature.w() != f32::INFINITY {
                points.push(best_point_and_curvature.xyz().into());
            }
        }
    }

    #[inline]
    fn linear_block_idx_from_object_voxel_indices(
        log2_block_size: usize,
        object_voxel_indices: [usize; 3],
    ) -> usize {
        let block_indices =
            Self::block_indices_from_object_voxel_indices(log2_block_size, object_voxel_indices);
        Self::linear_block_idx_within_chunk(log2_block_size, block_indices)
    }

    #[inline]
    fn block_indices_from_object_voxel_indices(
        log2_block_size: usize,
        [i, j, k]: [usize; 3],
    ) -> [usize; 3] {
        let indices_within_chunk =
            object::voxel_indices_within_chunk_from_object_voxel_indices(i, j, k);
        Self::block_indices_from_voxel_indices_within_chunk(log2_block_size, indices_within_chunk)
    }

    #[inline]
    fn block_indices_from_voxel_indices_within_chunk(
        log2_block_size: usize,
        voxel_indices: [usize; 3],
    ) -> [usize; 3] {
        [
            Self::shift_from_voxel_idx_within_chunk_to_block_idx(log2_block_size, voxel_indices[0]),
            Self::shift_from_voxel_idx_within_chunk_to_block_idx(log2_block_size, voxel_indices[1]),
            Self::shift_from_voxel_idx_within_chunk_to_block_idx(log2_block_size, voxel_indices[2]),
        ]
    }

    #[inline]
    fn linear_block_idx_within_chunk(log2_block_size: usize, block_indices: [usize; 3]) -> usize {
        let log2_chunk_size_in_blocks = log2_chunk_size_in_blocks(log2_block_size);
        (block_indices[0] << (2 * log2_chunk_size_in_blocks))
            + (block_indices[1] << log2_chunk_size_in_blocks)
            + block_indices[2]
    }

    #[inline]
    fn shift_from_voxel_idx_within_chunk_to_block_idx(
        log2_block_size: usize,
        voxel_idx: usize,
    ) -> usize {
        voxel_idx >> log2_block_size
    }
}

#[inline]
const fn log2_chunk_size_in_blocks(log2_block_size: usize) -> usize {
    LOG2_CHUNK_SIZE - log2_block_size
}

#[inline]
const fn chunk_block_count(log2_block_size: usize) -> usize {
    1 << (3 * log2_chunk_size_in_blocks(log2_block_size))
}

fn generate_mutual_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object_a: &VoxelObjectCollidable,
    voxel_object_b: &VoxelObjectCollidable,
    voxel_object_a_collidable_id: CollidableID,
    voxel_object_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        entity_id: entity_a_id,
        response_params: response_params_a,
        transform_to_object_space: transform_from_world_to_a,
    } = voxel_object_a;

    let VoxelObjectCollidable {
        entity_id: entity_b_id,
        response_params: response_params_b,
        transform_to_object_space: transform_from_world_to_b,
    } = voxel_object_b;

    let object_a_id = VoxelObjectID::from_entity_id(*entity_a_id);
    let Some(object_a) = voxel_object_manager.get_voxel_object(object_a_id) else {
        return;
    };
    let Some(physics_context_a) = voxel_object_manager.get_physics_context(object_a_id) else {
        return;
    };
    let inertial_properties_a = &physics_context_a.inertial_property_manager;

    let object_b_id = VoxelObjectID::from_entity_id(*entity_b_id);
    let Some(object_b) = voxel_object_manager.get_voxel_object(object_b_id) else {
        return;
    };
    let Some(physics_context_b) = voxel_object_manager.get_physics_context(object_b_id) else {
        return;
    };
    let inertial_properties_b = &physics_context_b.inertial_property_manager;

    let transform_from_world_to_a = transform_from_world_to_a.aligned();
    let transform_from_world_to_b = transform_from_world_to_b.aligned();

    let response_params = ContactResponseParameters::combined(response_params_a, response_params_b);

    for_each_mutual_voxel_object_contact(
        object_a,
        inertial_properties_a,
        object_b,
        inertial_properties_b,
        &transform_from_world_to_a,
        &transform_from_world_to_b,
        &mut |indices_for_id, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                voxel_object_a_collidable_id,
                voxel_object_b_collidable_id,
                indices_for_id,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_mutual_voxel_object_contact<'a>(
    meshed_voxel_object_a: &'a MeshedVoxelObject,
    inertial_properties_a: &'a VoxelObjectInertialPropertyManager,
    meshed_voxel_object_b: &'a MeshedVoxelObject,
    inertial_properties_b: &'a VoxelObjectInertialPropertyManager,
    transform_from_world_to_a: &'a Isometry3,
    transform_from_world_to_b: &'a Isometry3,
    f: &mut impl FnMut([usize; 4], ContactGeometry),
) {
    let object_a = meshed_voxel_object_a.object();
    let object_b = meshed_voxel_object_b.object();

    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverted();

    let Some((intersection_voxel_ranges_in_a, intersection_voxel_ranges_in_b)) =
        VoxelObject::determine_voxel_ranges_encompassing_intersection(
            object_a,
            object_b,
            &transform_from_b_to_a,
        )
    else {
        return;
    };

    // There might be cases where checking one against the other is enough, but
    // until we have a good heuristic for that, we check both
    let check_a_against_b = true;
    let check_b_against_a = true;

    if check_a_against_b {
        let grid_dimensions_for_b = object_b.chunk_counts().map(|count| count * CHUNK_SIZE);

        // Use the center of mass as the object center
        let norm_object_center_for_b = (inertial_properties_b.derive_center_of_mass()
            * object_b.inverse_voxel_extent())
        .compact();

        // We expand the intersected AABB by one voxel because the surface can
        // poke slightly outside the occupied voxel range (a voxel is only
        // considered occuped if the surface crosses its center)
        let intersection_aabb_in_a = object::aabb_from_voxel_ranges(
            object_a.voxel_extent(),
            &intersection_voxel_ranges_in_a,
        )
        .expanded_about_center(object_a.voxel_extent());

        let intersection_chunk_ranges_in_a = intersection_voxel_ranges_in_a
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let collision_probes_for_a = meshed_voxel_object_a.collision_probes();

        for (chunk_indices, probe_point_range) in collision_probes_for_a.chunk_point_ranges() {
            if !intersection_chunk_ranges_in_a[0].contains(&chunk_indices[0])
                || !intersection_chunk_ranges_in_a[1].contains(&chunk_indices[1])
                || !intersection_chunk_ranges_in_a[2].contains(&chunk_indices[2])
            {
                continue;
            }

            for probe_point in &collision_probes_for_a.probe_points()[probe_point_range] {
                let point_in_a = probe_point.aligned();

                if !intersection_aabb_in_a.contains_point(&point_in_a) {
                    continue;
                }

                let point = transform_from_world_to_a.inverse_transform_point(&point_in_a);

                let norm_point_in_b = transform_from_world_to_b.transform_point(&point)
                    * object_b.inverse_voxel_extent();
                let norm_point_in_b = norm_point_in_b.as_vector();

                // Required to ensure safety in
                // `determine_sdf_value_and_normal_at_point_if_intersecting`. We
                // make the assertion here to avoid pressuring that function
                // with more instructions.
                assert!(!norm_point_in_b.has_nan_component());

                let Some((signed_distance_in_b, normal_vector_in_b)) =
                    determine_sdf_value_and_normal_at_point_if_intersecting(
                        object_b,
                        &grid_dimensions_for_b,
                        &norm_object_center_for_b,
                        &norm_point_in_b.compact(),
                    )
                else {
                    continue;
                };

                let surface_normal = transform_from_world_to_b
                    .rotation()
                    .inverse()
                    .rotate_unit_vector(&normal_vector_in_b);

                let penetration_depth = -signed_distance_in_b * object_b.voxel_extent();

                let norm_point_in_a = point_in_a * object_a.inverse_voxel_extent();

                let [i_a, j_a, k_a] = <[f32; 3]>::from(norm_point_in_a).map(|idx| idx as usize);

                f(
                    [0, i_a, j_a, k_a],
                    ContactGeometry {
                        position: point,
                        surface_normal,
                        penetration_depth,
                    },
                );
            }
        }
    }

    if check_b_against_a {
        let grid_dimensions_for_a = object_a.chunk_counts().map(|count| count * CHUNK_SIZE);

        let norm_object_center_for_a = (inertial_properties_a.derive_center_of_mass()
            * object_a.inverse_voxel_extent())
        .compact();

        let intersection_aabb_in_b = object::aabb_from_voxel_ranges(
            object_b.voxel_extent(),
            &intersection_voxel_ranges_in_b,
        )
        .expanded_about_center(object_a.voxel_extent());

        let intersection_chunk_ranges_in_b = intersection_voxel_ranges_in_b
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let collision_probes_for_b = meshed_voxel_object_b.collision_probes();

        for (chunk_indices, probe_point_range) in collision_probes_for_b.chunk_point_ranges() {
            if !intersection_chunk_ranges_in_b[0].contains(&chunk_indices[0])
                || !intersection_chunk_ranges_in_b[1].contains(&chunk_indices[1])
                || !intersection_chunk_ranges_in_b[2].contains(&chunk_indices[2])
            {
                continue;
            }

            for probe_point in &collision_probes_for_b.probe_points()[probe_point_range] {
                let point_in_b = probe_point.aligned();

                if !intersection_aabb_in_b.contains_point(&point_in_b) {
                    continue;
                }

                let point = transform_from_world_to_b.inverse_transform_point(&point_in_b);

                let norm_point_in_a = transform_from_world_to_a.transform_point(&point)
                    * object_a.inverse_voxel_extent();
                let norm_point_in_a = norm_point_in_a.as_vector();

                assert!(!norm_point_in_a.has_nan_component());

                let Some((signed_distance_in_a, normal_vector_in_a)) =
                    determine_sdf_value_and_normal_at_point_if_intersecting(
                        object_a,
                        &grid_dimensions_for_a,
                        &norm_object_center_for_a,
                        &norm_point_in_a.compact(),
                    )
                else {
                    continue;
                };

                let normal_vector = transform_from_world_to_a
                    .rotation()
                    .inverse()
                    .rotate_unit_vector(&normal_vector_in_a);

                let surface_normal = -normal_vector;

                let penetration_depth = -signed_distance_in_a * object_a.voxel_extent();

                let norm_point_in_b = point_in_b * object_b.inverse_voxel_extent();

                let [i_b, j_b, k_b] = <[f32; 3]>::from(norm_point_in_b).map(|idx| idx as usize);

                f(
                    [0, i_b, j_b, k_b],
                    ContactGeometry {
                        position: point,
                        surface_normal,
                        penetration_depth,
                    },
                );
            }
        }
    }
}

fn generate_sphere_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    sphere: &SphereCollidable,
    voxel_object: &VoxelObjectCollidable,
    sphere_collidable_id: CollidableID,
    voxel_object_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, sphere.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let sphere = sphere.sphere().aligned();

    for_each_sphere_voxel_object_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &sphere,
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                sphere_collidable_id,
                voxel_object_collidable_id,
                indices,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_sphere_voxel_object_contact(
    voxel_object: &VoxelObject,
    transform_to_object_space: &Isometry3,
    sphere: &Sphere,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let sphere_in_object_space = sphere.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_sphere(
        &sphere_in_object_space,
        &mut |[i, j, k], voxel, _| {
            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            let voxel_sphere = Sphere::new(voxel_center, voxel_radius);

            let Some(contact_geometry) =
                determine_sphere_sphere_contact_geometry(sphere, &voxel_sphere)
            else {
                return;
            };

            f([i, j, k], contact_geometry);
        },
    );
}

fn generate_voxel_object_plane_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object: &VoxelObjectCollidable,
    plane: &PlaneCollidable,
    voxel_object_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, plane.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let plane = plane.plane().aligned();

    for_each_voxel_object_plane_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &plane,
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                plane_collidable_id,
                voxel_object_collidable_id,
                indices,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_voxel_object_plane_contact(
    voxel_object: &VoxelObject,
    transform_to_object_space: &Isometry3,
    plane: &Plane,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let plane_in_object_space = plane.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
        &plane_in_object_space,
        &mut |[i, j, k], voxel, placement| {
            // In the case of a plane, we only need contacts for the corner
            // voxels
            if placement != VoxelSurfacePlacement::Corner {
                return;
            }

            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            if let Some(contact_geometry) = determine_sphere_plane_contact_geometry(
                &Sphere::new(voxel_center, voxel_radius),
                plane,
            ) {
                f([i, j, k], contact_geometry);
            }
        },
    );
}

fn generate_capsule_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    capsule: &CapsuleCollidable,
    voxel_object: &VoxelObjectCollidable,
    capsule_collidable_id: CollidableID,
    voxel_object_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    let VoxelObjectCollidable {
        entity_id,
        response_params,
        transform_to_object_space,
    } = voxel_object;

    let object_id = VoxelObjectID::from_entity_id(*entity_id);
    let Some(voxel_object) = voxel_object_manager.get_voxel_object(object_id) else {
        return;
    };

    let response_params =
        ContactResponseParameters::combined(response_params, capsule.response_params());

    let transform_to_object_space = transform_to_object_space.aligned();
    let capsule = capsule.capsule().aligned();

    for_each_capsule_voxel_object_contact(
        voxel_object.object(),
        &transform_to_object_space,
        &capsule,
        &mut |indices, geometry| {
            let id = contact_id_from_collidable_ids_and_indices(
                capsule_collidable_id,
                voxel_object_collidable_id,
                indices,
            );

            contact_manifold.add_contact(ContactWithID {
                id,
                contact: Contact {
                    geometry,
                    response_params,
                },
            });
        },
    );
}

pub fn for_each_capsule_voxel_object_contact(
    voxel_object: &VoxelObject,
    transform_to_object_space: &Isometry3,
    capsule: &Capsule,
    f: &mut impl FnMut([usize; 3], ContactGeometry),
) {
    let capsule_in_object_space = capsule.iso_transformed(transform_to_object_space);

    voxel_object.for_each_surface_voxel_maybe_intersecting_capsule(
        &capsule_in_object_space,
        &mut |[i, j, k], voxel, _| {
            let voxel_center_in_object_space =
                voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

            let voxel_center =
                transform_to_object_space.inverse_transform_point(&voxel_center_in_object_space);
            let voxel_radius = compute_voxel_radius(voxel, voxel_object.voxel_extent());

            let voxel_sphere = Sphere::new(voxel_center, voxel_radius);

            let Some(contact_geometry) =
                determine_capsule_sphere_contact_geometry(capsule, &voxel_sphere)
            else {
                return;
            };

            f([i, j, k], contact_geometry);
        },
    );
}

fn determine_sdf_value_and_normal_at_point_if_intersecting(
    object: &VoxelObject,
    grid_dimensions: &[usize; 3],
    norm_object_center: &Vector3C,
    norm_point: &Vector3C,
) -> Option<(f32, UnitVector3)> {
    const HALF_VOXEL_DIAGONAL: f32 = 0.5 * SQRT_3;

    // Compute the point that, when floored, gives the indices of the lower cell
    // of the 2x2x2 region closest to the point.
    let lower_point = norm_point - Vector3C::same(0.5);

    // Avoid sampling outside the lower bounds of the SDF grid
    if lower_point.has_negative_component() {
        return None;
    }

    let [li, lj, lk] = <[f32; 3]>::from(lower_point).map(|idx| idx as usize);

    // Avoid sampling outside the upper bounds of the SDF grid. We use bitwise
    // ORs to avoid branches due to short-circuiting.
    if (li + 1 >= grid_dimensions[0])
        | (lj + 1 >= grid_dimensions[1])
        | (lk + 1 >= grid_dimensions[2])
    {
        return None;
    }

    // Compute the indices of the cell containing the point.
    //
    // SAFETY: The point is within the 2x2x2 region, so we know it is
    // non-negative and fits in `usize`. The caller asserted that it is not NaN.
    let [ci, cj, ck] = index_vector_to_ints_maybe_unchecked(*norm_point);

    let [chunk_i, chunk_j, chunk_k] = object::chunk_indices_from_object_voxel_indices(ci, cj, ck);
    let chunk_idx = object.linear_chunk_idx(&[chunk_i, chunk_j, chunk_k]);
    let chunk = object.chunk_at_idx_maybe_unchecked(chunk_idx);

    let chunk = match chunk {
        VoxelChunk::NonUniform(chunk) => chunk,
        VoxelChunk::Uniform(_) => {
            return estimate_sdf_value_and_normal_at_point_deep_inside(
                norm_object_center,
                norm_point,
            );
        }
        // If the point lies inside a void chunk, it's outside
        VoxelChunk::Void => {
            return None;
        }
    };

    let chunk_start_voxel_idx = chunk.start_voxel_idx();

    let containing_voxel_idx = chunk_start_voxel_idx
        + object::linear_voxel_idx_within_chunk_from_object_voxel_indices(ci, cj, ck);

    let containing_signed_dist = object
        .voxel_at_idx_maybe_unchecked(containing_voxel_idx)
        .signed_distance()
        .to_f32();

    // If the signed distance at the center of the voxel containing the point
    // exceeds half a voxel diagonal, the surface doesn't reach the voxel, so
    // the point can't cross the surface
    if containing_signed_dist > HALF_VOXEL_DIAGONAL {
        return None;
    }

    let [cli, clj, clk] = object::voxel_indices_within_chunk_from_object_voxel_indices(li, lj, lk);

    let all_in_same_chunk =
        (cli != CHUNK_SIZE - 1) & (clj != CHUNK_SIZE - 1) & (clk != CHUNK_SIZE - 1);

    let signed_distances = if all_in_same_chunk {
        let sample_dist = |i, j, k| {
            let voxel_idx =
                chunk_start_voxel_idx + object::linear_voxel_idx_within_chunk(&[i, j, k]);
            let voxel = object.voxel_at_idx_maybe_unchecked(voxel_idx);
            voxel.signed_distance().to_f32()
        };

        [
            sample_dist(cli, clj, clk),
            sample_dist(cli, clj, clk + 1),
            sample_dist(cli, clj + 1, clk),
            sample_dist(cli, clj + 1, clk + 1),
            sample_dist(cli + 1, clj, clk),
            sample_dist(cli + 1, clj, clk + 1),
            sample_dist(cli + 1, clj + 1, clk),
            sample_dist(cli + 1, clj + 1, clk + 1),
        ]
    } else {
        let sample_dist = |i, j, k| {
            object
                .voxel_maybe_unchecked(i, j, k)
                .signed_distance()
                .to_f32()
        };
        [
            sample_dist(li, lj, lk),
            sample_dist(li, lj, lk + 1),
            sample_dist(li, lj + 1, lk),
            sample_dist(li, lj + 1, lk + 1),
            sample_dist(li + 1, lj, lk),
            sample_dist(li + 1, lj, lk + 1),
            sample_dist(li + 1, lj + 1, lk),
            sample_dist(li + 1, lj + 1, lk + 1),
        ]
    };

    let lower_indices = lower_point.component_floor();
    let fractional_offset = lower_point - lower_indices;
    let signed_distance =
        sdf::evaluate_sdf_from_corner_samples(&signed_distances, &fractional_offset);

    // Check if the point is outside of the surface
    if signed_distance > 0.0 {
        return None;
    }

    // Check if the point is deep enough inside that the signed distance is capped
    if (signed_distance - VoxelSignedDistance::MIN_F32).abs() < 1e-3 {
        return estimate_sdf_value_and_normal_at_point_deep_inside(norm_object_center, norm_point);
    }

    let sdf_gradient = sdf::compute_sdf_gradient_from_corner_samples(
        &signed_distances,
        &fractional_offset.aligned(),
    );

    let normal_vector = UnitVector3::normalized_from_if_above(sdf_gradient, 1e-8)?;

    Some((signed_distance, normal_vector))
}

fn estimate_sdf_value_and_normal_at_point_deep_inside(
    norm_object_center: &Vector3C,
    norm_point: &Vector3C,
) -> Option<(f32, UnitVector3)> {
    // If the point is deep enough in the SDF interior that the local signed
    // distance is capped, we would get a zero gradient, so we can't determine
    // the normal vector from that. As a rough fallback, we use the direction
    // from the object center to the point as the normal vector and the capped
    // signed distance as a conservative estimate for calculating the
    // penetration depth.
    let signed_distance = VoxelSignedDistance::MIN_F32;
    let displacement = (norm_point - norm_object_center).aligned();
    let normal_vector = UnitVector3::normalized_from_if_above(displacement, 1e-8)?;
    Some((signed_distance, normal_vector))
}

#[cfg(not(feature = "unchecked"))]
#[inline]
fn index_vector_to_ints_maybe_unchecked(indices: Vector3C) -> [usize; 3] {
    <[f32; 3]>::from(indices).map(|idx| idx as usize)
}

#[cfg(feature = "unchecked")]
#[inline]
fn index_vector_to_ints_maybe_unchecked(indices: Vector3C) -> [usize; 3] {
    <[f32; 3]>::from(indices).map(|idx| unsafe { idx.to_int_unchecked::<usize>() })
}

#[inline]
fn compute_voxel_radius(voxel: &Voxel, voxel_extent: f32) -> f32 {
    -voxel.signed_distance().to_f32() * voxel_extent
}
