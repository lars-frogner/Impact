//! Gizmo meshes.

use crate::{
    gizmo::{
        GizmoType,
        model::{
            COLLIDER_GIZMO_PLANE_MODEL_IDX, COLLIDER_GIZMO_SPHERE_MODEL_IDX,
            COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX, SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
            SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_EMPTY_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_OBSCURABLE_EMPTY_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
            VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX,
        },
    },
    resource::ResourceManager,
};
use impact_light::MAX_SHADOW_MAP_CASCADES;
use impact_mesh::{LineSegmentMesh, LineSegmentMeshDirtyMask, TriangleMesh, VertexColor};
use impact_voxel::chunks::CHUNK_SIZE;

impl GizmoType {
    fn generate_mesh_in_registry(&self, resource_manager: &mut ResourceManager) {
        match self {
            Self::ReferenceFrameAxes => {
                let mesh = LineSegmentMesh::create_reference_frame_axes();
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::BoundingSphere => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::CYAN.with_alpha(0.15),
                );
                resource_manager
                    .triangle_meshes
                    .insert(self.only_triangle_mesh_id(), mesh);
            }
            Self::LightSphere => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::YELLOW.with_alpha(0.1),
                );
                resource_manager
                    .triangle_meshes
                    .insert(self.only_triangle_mesh_id(), mesh);
            }
            Self::ShadowCubemapFaces => {
                let planes_mesh = TriangleMesh::create_cube_with_face_colors(
                    2.0,
                    &[
                        VertexColor::RED,
                        VertexColor::GREEN,
                        VertexColor::BLUE,
                        VertexColor::CYAN,
                        VertexColor::MAGENTA,
                        VertexColor::YELLOW,
                    ]
                    .map(|color| color.with_alpha(0.1)),
                );
                resource_manager.triangle_meshes.insert(
                    self.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX].triangle_mesh_id(),
                    planes_mesh,
                );

                let mut outlines_mesh = LineSegmentMesh::create_unit_cubemap_frusta();
                outlines_mesh
                    .set_same_color(VertexColor::WHITE, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager.line_segment_meshes.insert(
                    self.models()[SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX]
                        .line_segment_mesh_id(),
                    outlines_mesh,
                );
            }
            Self::ShadowMapCascades => {
                const CASCADE_COLORS: [VertexColor<f32>; 4] = [
                    VertexColor::RED,
                    VertexColor::YELLOW,
                    VertexColor::GREEN,
                    VertexColor::CYAN,
                ];
                const _: () = assert!(CASCADE_COLORS.len() == MAX_SHADOW_MAP_CASCADES as usize);

                assert_eq!(self.models().len(), CASCADE_COLORS.len());

                for (model, color) in self
                    .models()
                    .iter()
                    .zip(CASCADE_COLORS.map(|color| color.with_alpha(0.2)))
                {
                    let mesh = TriangleMesh::create_vertical_square_with_color(1.0, color);
                    resource_manager
                        .triangle_meshes
                        .insert(model.triangle_mesh_id(), mesh);
                }
            }
            Self::CenterOfMass => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::BLUE.with_alpha(0.4),
                );
                resource_manager
                    .triangle_meshes
                    .insert(self.only_triangle_mesh_id(), mesh);
            }
            Self::LinearVelocity => {
                let mut mesh = LineSegmentMesh::create_unit_arrow_y();
                mesh.set_same_color(VertexColor::RED, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::AngularVelocity => {
                let mut mesh = LineSegmentMesh::create_unit_arrow_y();
                mesh.set_same_color(VertexColor::YELLOW, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::AngularMomentum => {
                let mut mesh = LineSegmentMesh::create_unit_arrow_y();
                mesh.set_same_color(VertexColor::MAGENTA, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::Force => {
                let mut mesh = LineSegmentMesh::create_unit_arrow_y();
                mesh.set_same_color(VertexColor::GREEN, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::Torque => {
                let mut mesh = LineSegmentMesh::create_unit_arrow_y();
                mesh.set_same_color(VertexColor::CYAN, &mut LineSegmentMeshDirtyMask::empty());
                resource_manager
                    .line_segment_meshes
                    .insert(self.only_line_segment_mesh_id(), mesh);
            }
            Self::Anchors => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::MAGENTA.with_alpha(0.4),
                );
                resource_manager
                    .triangle_meshes
                    .insert(self.only_triangle_mesh_id(), mesh);
            }
            Self::DynamicCollider | Self::StaticCollider | Self::PhantomCollider => {
                let color = match self {
                    Self::DynamicCollider => VertexColor::GREEN,
                    Self::StaticCollider => VertexColor::RED,
                    Self::PhantomCollider => VertexColor::MAGENTA,
                    _ => unreachable!(),
                }
                .with_alpha(0.1);

                let sphere_mesh = TriangleMesh::create_unit_sphere_with_color(32, color);
                resource_manager.triangle_meshes.insert(
                    self.models()[COLLIDER_GIZMO_SPHERE_MODEL_IDX].triangle_mesh_id(),
                    sphere_mesh,
                );

                let plane_mesh = TriangleMesh::create_vertical_square_with_color(1.0, color);
                resource_manager.triangle_meshes.insert(
                    self.models()[COLLIDER_GIZMO_PLANE_MODEL_IDX].triangle_mesh_id(),
                    plane_mesh,
                );

                let voxel_sphere_mesh = TriangleMesh::create_unit_sphere_with_color(8, color);
                resource_manager.triangle_meshes.insert(
                    self.models()[COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX].triangle_mesh_id(),
                    voxel_sphere_mesh,
                );
            }
            Self::VoxelChunks => {
                for idx in [
                    VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX,
                    VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX,
                ] {
                    let uniform_chunk_mesh = TriangleMesh::create_voxel_chunk_cube_with_color(
                        CHUNK_SIZE as f32,
                        VertexColor::RED.with_alpha(0.05),
                    );
                    resource_manager
                        .triangle_meshes
                        .insert(self.models()[idx].triangle_mesh_id(), uniform_chunk_mesh);
                }

                for idx in [
                    VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
                    VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
                ] {
                    let non_uniform_chunk_mesh = TriangleMesh::create_voxel_chunk_cube_with_color(
                        CHUNK_SIZE as f32,
                        VertexColor::GREEN.with_alpha(0.05),
                    );
                    resource_manager.triangle_meshes.insert(
                        self.models()[idx].triangle_mesh_id(),
                        non_uniform_chunk_mesh,
                    );
                }

                for idx in [
                    VOXEL_CHUNKS_GIZMO_OBSCURABLE_EMPTY_MODEL_IDX,
                    VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_EMPTY_MODEL_IDX,
                ] {
                    let empty_chunk_mesh = TriangleMesh::create_voxel_chunk_cube_with_color(
                        CHUNK_SIZE as f32,
                        VertexColor::BLUE.with_alpha(0.05),
                    );
                    resource_manager
                        .triangle_meshes
                        .insert(self.models()[idx].triangle_mesh_id(), empty_chunk_mesh);
                }
            }
            Self::VoxelIntersections => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    8,
                    VertexColor::YELLOW.with_alpha(0.1),
                );
                resource_manager
                    .triangle_meshes
                    .insert(self.only_triangle_mesh_id(), mesh);
            }
        }
    }
}

/// Generates the mesh for each gizmo type and adds them to the appropriate
/// resource registry.
pub fn generate_gizmo_meshes(resource_manager: &mut ResourceManager) {
    for gizmo in GizmoType::all() {
        gizmo.generate_mesh_in_registry(resource_manager);
    }
}
