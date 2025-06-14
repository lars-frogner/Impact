//! Gizmo meshes.

use super::GizmoType;
use crate::mesh::{
    MeshRepository, VertexColor, line_segment::LineSegmentMesh, triangle::TriangleMesh,
};
use anyhow::Result;

impl GizmoType {
    fn generate_mesh_in_repository(&self, mesh_repository: &mut MeshRepository) -> Result<()> {
        let mesh_id = self.model_id().mesh_id();
        match self {
            Self::ReferenceFrameAxes => {
                let mesh = LineSegmentMesh::create_reference_frame_axes();
                mesh_repository.add_line_segment_mesh(mesh_id, mesh)
            }
            Self::BoundingSphere => {
                let mesh = TriangleMesh::create_colored_unit_sphere(
                    32,
                    VertexColor::CYAN.with_alpha(0.15),
                );
                mesh_repository.add_triangle_mesh(mesh_id, mesh)
            }
            Self::LightSphere => {
                let mesh = TriangleMesh::create_colored_unit_sphere(
                    32,
                    VertexColor::YELLOW.with_alpha(0.1),
                );
                mesh_repository.add_triangle_mesh(mesh_id, mesh)
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
