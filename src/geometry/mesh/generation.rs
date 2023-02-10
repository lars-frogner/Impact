//! Generation of meshes representing geometrical objects.

use super::TriangleMesh;
use crate::num::Float;
use nalgebra::{UnitVector3, Vector3};

macro_rules! pos {
    [$x:expr, $y:expr, $z:expr] => {
        $crate::geometry::VertexPosition(nalgebra::point![$x, $y, $z])
    };
    ($point:expr) => {
        $crate::geometry::VertexPosition($point)
    };
}

macro_rules! normal {
    ($normal:expr) => {
        $crate::geometry::VertexNormalVector($normal)
    };
}

impl<F: Float> TriangleMesh<F> {
    /// Creates a mesh representing a box with the given extents, centered at
    /// the origin and with the width, height and depth axes aligned with the
    /// x-, y- and z-axis respectively.
    pub fn create_box(width: F, height: F, depth: F) -> Self {
        let hw = width / F::TWO;
        let hh = height / F::TWO;
        let hd = depth / F::TWO;

        let mut positions = Vec::with_capacity(24);
        let mut normal_vectors = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        let mut idx = 0;

        let mut add_face_normal = |normal: UnitVector3<F>| {
            normal_vectors.extend_from_slice(&[
                normal![normal],
                normal![normal],
                normal![normal],
                normal![normal],
            ]);
        };

        let mut add_face_indices = || {
            indices.extend_from_slice(&[idx, idx + 1, idx + 3, idx + 1, idx + 2, idx + 3]);
            idx += 4;
        };

        // Left face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![-hw, hh, -hd],
            pos![-hw, hh, hd],
            pos![-hw, -hh, hd],
        ]);
        add_face_normal(-Vector3::x_axis());
        add_face_indices();

        // Right face
        positions.extend_from_slice(&[
            pos![hw, -hh, -hd],
            pos![hw, hh, -hd],
            pos![hw, hh, hd],
            pos![hw, -hh, hd],
        ]);
        add_face_normal(Vector3::x_axis());
        add_face_indices();

        // Bottom face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![hw, -hh, -hd],
            pos![hw, -hh, hd],
            pos![-hw, -hh, hd],
        ]);
        add_face_normal(-Vector3::y_axis());
        add_face_indices();

        // Top face
        positions.extend_from_slice(&[
            pos![-hw, hh, -hd],
            pos![hw, hh, -hd],
            pos![hw, hh, hd],
            pos![-hw, hh, hd],
        ]);
        add_face_normal(Vector3::y_axis());
        add_face_indices();

        // Front face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![hw, -hh, -hd],
            pos![hw, hh, -hd],
            pos![-hw, hh, -hd],
        ]);
        add_face_normal(-Vector3::z_axis());
        add_face_indices();

        // Back face
        positions.extend_from_slice(&[
            pos![-hw, -hh, hd],
            pos![hw, -hh, hd],
            pos![hw, hh, hd],
            pos![-hw, hh, hd],
        ]);
        add_face_normal(Vector3::z_axis());
        add_face_indices();

        Self::new(positions, Vec::new(), normal_vectors, Vec::new(), indices)
    }
}
