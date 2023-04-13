//! Generation of meshes representing geometrical objects.

use super::TriangleMesh;
use crate::num::Float;
use nalgebra::{vector, UnitVector3, Vector3};

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
    /// Creates a mesh representing a flat plane centered at the origin with the
    /// given horizontal extents.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// If any of the given extents are negative.
    pub fn create_plane(extent_x: F, extent_z: F) -> Self {
        assert!(
            extent_x >= F::ZERO,
            "Tried to create plane mesh with negative x-extent"
        );
        assert!(
            extent_z >= F::ZERO,
            "Tried to create plane mesh with negative y-extent"
        );

        let hex = extent_x / F::TWO;
        let hez = extent_z / F::TWO;

        let positions = vec![
            pos![-hex, F::ZERO, -hez],
            pos![hex, F::ZERO, -hez],
            pos![hex, F::ZERO, hez],
            pos![-hex, F::ZERO, hez],
        ];

        let normal_vectors = vec![normal![Vector3::y_axis()]; 4];

        let indices = vec![0, 3, 1, 1, 3, 2];

        Self::new(
            positions,
            Vec::new(),
            normal_vectors,
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a box with the given extents, centered at
    /// the origin and with the width, height and depth axes aligned with the
    /// x-, y- and z-axis.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// If any of the given extents are negative.
    pub fn create_box(extent_x: F, extent_y: F, extent_z: F) -> Self {
        assert!(
            extent_x >= F::ZERO,
            "Tried to create box mesh with negative x-extent"
        );
        assert!(
            extent_y >= F::ZERO,
            "Tried to create box mesh with negative y-extent"
        );
        assert!(
            extent_z >= F::ZERO,
            "Tried to create box mesh with negative z-extent"
        );

        let hw = extent_x / F::TWO;
        let hh = extent_y / F::TWO;
        let hd = extent_z / F::TWO;

        let mut positions = Vec::with_capacity(24);
        let mut normal_vectors = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        let mut idx = 0;

        let mut add_face_indices = || {
            indices.extend_from_slice(&[idx, idx + 3, idx + 1, idx + 1, idx + 3, idx + 2]);
            idx += 4;
        };

        // Left face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![-hw, hh, -hd],
            pos![-hw, hh, hd],
            pos![-hw, -hh, hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![-Vector3::x_axis()]; 4]);
        add_face_indices();

        // Right face
        positions.extend_from_slice(&[
            pos![hw, -hh, -hd],
            pos![hw, -hh, hd],
            pos![hw, hh, hd],
            pos![hw, hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![Vector3::x_axis()]; 4]);
        add_face_indices();

        // Bottom face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![-hw, -hh, hd],
            pos![hw, -hh, hd],
            pos![hw, -hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![-Vector3::y_axis()]; 4]);
        add_face_indices();

        // Top face
        positions.extend_from_slice(&[
            pos![-hw, hh, -hd],
            pos![hw, hh, -hd],
            pos![hw, hh, hd],
            pos![-hw, hh, hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![Vector3::y_axis()]; 4]);
        add_face_indices();

        // Front face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![hw, -hh, -hd],
            pos![hw, hh, -hd],
            pos![-hw, hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![-Vector3::z_axis()]; 4]);
        add_face_indices();

        // Back face
        positions.extend_from_slice(&[
            pos![-hw, -hh, hd],
            pos![-hw, hh, hd],
            pos![hw, hh, hd],
            pos![hw, -hh, hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![Vector3::z_axis()]; 4]);
        add_face_indices();

        Self::new(
            positions,
            Vec::new(),
            normal_vectors,
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a cylinder with the given length and
    /// diameter, centered at the origin and with the length axis aligned with
    /// the y-axis. `n_circumference_vertices` is the number of vertices to use
    /// for representing a circular cross-section of the cylinder.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If any of the given extents are negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_cylinder(extent_y: F, diameter: F, n_circumference_vertices: usize) -> Self {
        assert!(
            extent_y >= F::ZERO,
            "Tried to create cylinder mesh with negative y-extent"
        );
        assert!(
            diameter >= F::ZERO,
            "Tried to create cylinder mesh with negative diameter"
        );
        assert!(
            n_circumference_vertices >= 2,
            "Tried to create cylinder mesh with fewer than two vertices around circumference"
        );

        let half_length = extent_y / F::TWO;
        let radius = diameter / F::TWO;

        let mut positions = Vec::with_capacity(4 * n_circumference_vertices + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(12 * n_circumference_vertices);

        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let angle_between_vertices = F::TWO_PI / F::from_u32(n_circumference_vertices).unwrap();

        // Bottom and top center vertices
        positions.push(pos![F::ZERO, -half_length, F::ZERO]);
        positions.push(pos![F::ZERO, half_length, F::ZERO]);
        normal_vectors.push(normal!(-Vector3::y_axis()));
        normal_vectors.push(normal!(Vector3::y_axis()));

        // First bottom and top side vertices
        let bottom_pos = pos![radius, -half_length, F::ZERO];
        let top_pos = pos![radius, half_length, F::ZERO];
        positions.push(bottom_pos);
        positions.push(top_pos);
        normal_vectors.push(normal!(Vector3::x_axis()));
        normal_vectors.push(normal!(Vector3::x_axis()));

        // Duplicate positions and use vertical instead of radial normal vectors
        positions.push(bottom_pos);
        positions.push(top_pos);
        normal_vectors.push(normal!(-Vector3::y_axis()));
        normal_vectors.push(normal!(Vector3::y_axis()));

        let mut angle = angle_between_vertices;

        for i in 1..n_circumference_vertices {
            let cos_angle = F::cos(angle);
            let sin_angle = F::sin(angle);

            let x = radius * cos_angle;
            let z = radius * sin_angle;

            let bottom_pos = pos![x, -half_length, z];
            let top_pos = pos![x, half_length, z];

            positions.push(bottom_pos);
            positions.push(top_pos);

            let radial_direction =
                UnitVector3::new_unchecked(vector![cos_angle, F::ZERO, sin_angle]);
            normal_vectors.push(normal!(radial_direction));
            normal_vectors.push(normal!(radial_direction));

            // Duplicate positions and use vertical instead of radial normal vectors
            positions.push(bottom_pos);
            positions.push(top_pos);
            normal_vectors.push(normal!(-Vector3::y_axis()));
            normal_vectors.push(normal!(Vector3::y_axis()));

            let current_idx = 4 * i + 2;
            indices.extend_from_slice(&[
                // First side triangle
                current_idx - 4,
                current_idx - 3,
                current_idx,
                // Second side triangle
                current_idx - 3,
                current_idx + 1,
                current_idx,
                // Bottom lid triangle
                current_idx + 2,
                0,
                current_idx - 2,
                // Top lid triangle
                current_idx - 1,
                1,
                current_idx + 3,
            ]);

            angle += angle_between_vertices;
        }

        // Connect to first vertices
        let current_idx = 4 * n_circumference_vertices + 2;
        indices.extend_from_slice(&[
            current_idx - 4,
            current_idx - 3,
            2,
            current_idx - 3,
            3,
            2,
            4,
            0,
            current_idx - 2,
            current_idx - 1,
            1,
            5,
        ]);

        Self::new(
            positions,
            Vec::new(),
            normal_vectors,
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a sphere with diameter 1.0, centered at the
    /// origin. `n_rings` is the number of horizontal circular cross-sections
    /// that vertices will be generated around. The number of vertices that will
    /// be generated around each ring increases in proportion to `n_rings` to
    /// maintain an approximately uniform resolution.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If `n_rings` is zero.
    pub fn create_sphere(n_rings: usize) -> Self {
        assert!(n_rings > 0, "Tried to create sphere mesh with no rings");

        let radius = F::ONE / F::TWO;

        let n_circumference_vertices = 2 * n_rings + 2;

        let mut positions = Vec::with_capacity(n_circumference_vertices * n_rings + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(6 * n_circumference_vertices * n_rings);

        let n_rings = u32::try_from(n_rings).unwrap();
        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let delta_phi = F::TWO_PI / F::from_u32(n_circumference_vertices).unwrap();
        let delta_theta = <F as Float>::PI / F::from_u32(n_rings + 1).unwrap();

        positions.push(pos![F::ZERO, radius, F::ZERO]);
        normal_vectors.push(normal!(Vector3::y_axis()));

        positions.push(pos![F::ZERO, -radius, F::ZERO]);
        normal_vectors.push(normal!(-Vector3::y_axis()));

        let mut theta = delta_theta;

        for _ in 0..n_rings {
            let sin_theta = F::sin(theta);
            let cos_theta = F::cos(theta);
            let y = radius * cos_theta;

            let mut phi = F::ZERO;

            for _ in 0..n_circumference_vertices {
                let cos_phi_sin_theta = F::cos(phi) * sin_theta;
                let sin_phi_sin_theta = F::sin(phi) * sin_theta;

                positions.push(pos![
                    radius * cos_phi_sin_theta,
                    y,
                    radius * sin_phi_sin_theta
                ]);
                normal_vectors.push(normal!(UnitVector3::new_unchecked(vector![
                    cos_phi_sin_theta,
                    cos_theta,
                    sin_phi_sin_theta
                ])));

                phi += delta_phi;
            }

            theta += delta_theta;
        }

        let mut idx = 2;

        // Top cap
        for _ in 0..n_circumference_vertices - 1 {
            indices.extend_from_slice(&[idx, 0, idx + 1]);
            idx += 1;
        }
        indices.extend_from_slice(&[idx, 0, idx - n_circumference_vertices + 1]);
        idx += 1;

        for _ in 1..n_rings {
            for _ in 0..n_circumference_vertices - 1 {
                indices.extend_from_slice(&[
                    idx,
                    idx - n_circumference_vertices,
                    idx + 1,
                    idx - n_circumference_vertices,
                    idx - n_circumference_vertices + 1,
                    idx + 1,
                ]);
                idx += 1;
            }
            indices.extend_from_slice(&[
                idx,
                idx - n_circumference_vertices,
                idx - n_circumference_vertices + 1,
                idx - n_circumference_vertices,
                idx - 2 * n_circumference_vertices + 1,
                idx - n_circumference_vertices + 1,
            ]);
            idx += 1;
        }

        idx -= n_circumference_vertices;

        // Bottom cap
        for _ in 0..n_circumference_vertices - 1 {
            indices.extend_from_slice(&[1, idx, idx + 1]);
            idx += 1;
        }
        indices.extend_from_slice(&[1, idx, idx - n_circumference_vertices + 1]);

        Self::new(
            positions,
            Vec::new(),
            normal_vectors,
            Vec::new(),
            Vec::new(),
            indices,
        )
    }
}
