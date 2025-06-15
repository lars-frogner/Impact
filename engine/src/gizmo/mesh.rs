//! Gizmo meshes.

use crate::{
    gizmo::{
        GizmoType,
        model::{
            BOUNDING_SPHERE_GIZMO_MODEL_IDX, LIGHT_SPHERE_GIZMO_MODEL_IDX,
            REFERENCE_FRAME_AXES_GIZMO_MODEL_IDX, SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
            SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
        },
    },
    mesh::{MeshRepository, VertexColor, line_segment::LineSegmentMesh, triangle::TriangleMesh},
};
use anyhow::Result;

impl GizmoType {
    fn generate_mesh_in_repository(&self, mesh_repository: &mut MeshRepository) -> Result<()> {
        let models = self.models();
        match self {
            Self::ReferenceFrameAxes => {
                let mesh = LineSegmentMesh::create_reference_frame_axes();
                mesh_repository.add_line_segment_mesh(
                    models[REFERENCE_FRAME_AXES_GIZMO_MODEL_IDX].mesh_id(),
                    mesh,
                )
            }
            Self::BoundingSphere => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::CYAN.with_alpha(0.15),
                );
                mesh_repository
                    .add_triangle_mesh(models[BOUNDING_SPHERE_GIZMO_MODEL_IDX].mesh_id(), mesh)
            }
            Self::LightSphere => {
                let mesh = TriangleMesh::create_unit_sphere_with_color(
                    32,
                    VertexColor::YELLOW.with_alpha(0.1),
                );
                mesh_repository
                    .add_triangle_mesh(models[LIGHT_SPHERE_GIZMO_MODEL_IDX].mesh_id(), mesh)
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
                mesh_repository.add_triangle_mesh(
                    models[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX].mesh_id(),
                    planes_mesh,
                )?;

                let mut outlines_mesh = LineSegmentMesh::create_unit_cubemap_frusta();
                outlines_mesh.set_same_color(VertexColor::WHITE);
                mesh_repository.add_line_segment_mesh(
                    models[SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX].mesh_id(),
                    outlines_mesh,
                )
            }
        }
    }
}

/// Generates the mesh for each gizmo type and adds them to the repository.
pub fn generate_gizmo_meshes(mesh_repository: &mut MeshRepository) -> Result<()> {
    for gizmo in GizmoType::all() {
        gizmo.generate_mesh_in_repository(mesh_repository)?;
    }
    Ok(())
}
