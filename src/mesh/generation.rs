//! Generation of meshes representing geometrical objects.

use crate::{
    mesh::{FrontFaceSide, TriangleMesh},
    num::Float,
};
use approx::{abs_diff_eq, abs_diff_ne};
use nalgebra::{vector, UnitVector3, Vector3};

macro_rules! pos {
    [$x:expr, $y:expr, $z:expr] => {
        $crate::mesh::VertexPosition(nalgebra::point![$x, $y, $z])
    };
    ($point:expr) => {
        $crate::mesh::VertexPosition($point)
    };
}

macro_rules! normal {
    ($normal:expr) => {
        $crate::mesh::VertexNormalVector($normal)
    };
}

impl<F: Float> TriangleMesh<F> {
    /// Creates a mesh with vertex positions directly in clip space coordinates,
    /// consisting of two triangles at zero depth that will exactly fill the
    /// view.
    pub fn create_screen_filling_quad() -> Self {
        let positions = vec![
            pos![-F::ONE, -F::ONE, F::ZERO],
            pos![F::ONE, -F::ONE, F::ZERO],
            pos![F::ONE, F::ONE, F::ZERO],
            pos![-F::ONE, F::ONE, F::ZERO],
        ];

        let indices = vec![1, 3, 0, 2, 3, 1];

        Self::new(
            positions,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a rectangle centered at the origin with the
    /// given horizontal extents.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// If any of the given extents are negative.
    pub fn create_rectangle(extent_x: F, extent_z: F) -> Self {
        assert!(
            extent_x >= F::ZERO,
            "Tried to create rectangle mesh with negative x-extent"
        );
        assert!(
            extent_z >= F::ZERO,
            "Tried to create rectangle mesh with negative y-extent"
        );

        let hex = extent_x * F::ONE_HALF;
        let hez = extent_z * F::ONE_HALF;

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
    pub fn create_box(
        extent_x: F,
        extent_y: F,
        extent_z: F,
        front_face_side: FrontFaceSide,
    ) -> Self {
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

        let hw = extent_x * F::ONE_HALF;
        let hh = extent_y * F::ONE_HALF;
        let hd = extent_z * F::ONE_HALF;

        let mut positions = Vec::with_capacity(24);
        let mut normal_vectors = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        let mut idx = 0;

        let mut add_face_indices = || {
            match front_face_side {
                FrontFaceSide::Outside => {
                    indices.extend_from_slice(&[idx, idx + 3, idx + 1, idx + 1, idx + 3, idx + 2]);
                }
                FrontFaceSide::Inside => {
                    indices.extend_from_slice(&[idx + 1, idx + 3, idx, idx + 2, idx + 3, idx + 1]);
                }
            }
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
    pub fn create_cylinder(length: F, diameter: F, n_circumference_vertices: usize) -> Self {
        Self::create_circular_frustum(length, diameter, diameter, n_circumference_vertices)
    }

    /// Creates a mesh representing a cone with the given length and maximum
    /// diameter, centered at the origin and pointing along the positive
    /// y-direction. `n_circumference_vertices` is the number of vertices to use
    /// for representing a circular cross-section of the cone.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If any of the given extents are negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_cone(length: F, max_diameter: F, n_circumference_vertices: usize) -> Self {
        Self::create_circular_frustum(length, max_diameter, F::ZERO, n_circumference_vertices)
    }

    /// Creates a mesh representing a y-axis aligned circular frustum with the
    /// given length, bottom diameter and top diameter, centered at the origin.
    /// `n_circumference_vertices` is the number of vertices to use for
    /// representing a circular cross-section of the frustum.
    ///
    /// Using the same bottom and top diameter yields a cylinder, while setting
    /// either diameter to zero yields a cone.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If any of the given extents are negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_circular_frustum(
        length: F,
        bottom_diameter: F,
        top_diameter: F,
        n_circumference_vertices: usize,
    ) -> Self {
        assert!(
            length >= F::ZERO,
            "Tried to create circular frustum mesh with negative length"
        );
        assert!(
            bottom_diameter >= F::ZERO,
            "Tried to create circular frustum mesh with negative bottom diameter"
        );
        assert!(
            top_diameter >= F::ZERO,
            "Tried to create circular frustum mesh with negative top diameter"
        );
        assert!(
            n_circumference_vertices >= 2,
            "Tried to create circular frustum mesh with fewer than two vertices around circumference"
        );

        let half_length = length * F::ONE_HALF;
        let bottom_radius = bottom_diameter * F::ONE_HALF;
        let top_radius = top_diameter * F::ONE_HALF;

        let mut positions = Vec::with_capacity(4 * n_circumference_vertices + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(12 * n_circumference_vertices);

        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let angle_between_vertices = F::TWO_PI / F::from_u32(n_circumference_vertices).unwrap();

        let tan_slope_angle = if abs_diff_eq!(length, F::ZERO) {
            F::ZERO
        } else {
            (bottom_radius - top_radius) / length
        };
        let cos_slope_angle = F::ONE / F::sqrt(F::ONE + F::powi(tan_slope_angle, 2));
        let sin_slope_angle = cos_slope_angle * tan_slope_angle;

        // First bottom side vertex
        let bottom_pos = pos![bottom_radius, -half_length, F::ZERO];
        positions.push(bottom_pos);

        // First top side vertex
        let top_pos = pos![top_radius, half_length, F::ZERO];
        positions.push(top_pos);

        // Normal direction at first side vertices
        let normal_direction = normal!(UnitVector3::new_unchecked(vector![
            cos_slope_angle,
            sin_slope_angle,
            F::ZERO
        ]));
        normal_vectors.push(normal_direction);
        normal_vectors.push(normal_direction);

        let mut idx = 1;

        let mut polar_angle = angle_between_vertices;

        for _ in 1..n_circumference_vertices {
            let cos_polar_angle = F::cos(polar_angle);
            let sin_polar_angle = F::sin(polar_angle);

            let bottom_pos = pos![
                bottom_radius * cos_polar_angle,
                -half_length,
                bottom_radius * sin_polar_angle
            ];
            positions.push(bottom_pos);

            let top_pos = pos![
                top_radius * cos_polar_angle,
                half_length,
                top_radius * sin_polar_angle
            ];
            positions.push(top_pos);

            let normal_direction = normal!(UnitVector3::new_unchecked(vector![
                cos_polar_angle * cos_slope_angle,
                sin_slope_angle,
                sin_polar_angle * cos_slope_angle
            ]));
            normal_vectors.push(normal_direction);
            normal_vectors.push(normal_direction);

            idx += 2;

            indices.extend_from_slice(&[
                // First side triangle
                idx - 3,
                idx - 2,
                idx,
                // Second side triangle
                idx,
                idx - 1,
                idx - 3,
            ]);

            polar_angle += angle_between_vertices;
        }

        // Connect to first vertices
        indices.extend_from_slice(&[
            // First side triangle
            idx - 1,
            idx,
            1,
            // Second side triangle
            1,
            0,
            idx - 1,
        ]);

        let mut create_horizontal_disk = |radius, y, front_is_up| {
            // Center vertex
            positions.push(pos![F::ZERO, y, F::ZERO]);

            idx += 1;
            let center_idx = idx;

            // First side vertex
            positions.push(pos![radius, y, F::ZERO]);

            idx += 1;

            let mut polar_angle = angle_between_vertices;

            for _ in 1..n_circumference_vertices {
                let cos_polar_angle = F::cos(polar_angle);
                let sin_polar_angle = F::sin(polar_angle);

                positions.push(pos![radius * cos_polar_angle, y, radius * sin_polar_angle]);

                idx += 1;

                if front_is_up {
                    indices.extend_from_slice(&[center_idx, idx, idx - 1]);
                } else {
                    indices.extend_from_slice(&[center_idx, idx - 1, idx]);
                }

                polar_angle += angle_between_vertices;
            }

            if front_is_up {
                indices.extend_from_slice(&[center_idx, center_idx + 1, idx]);
            } else {
                indices.extend_from_slice(&[center_idx, idx, center_idx + 1]);
            }

            normal_vectors.extend_from_slice(&vec![
                normal!(if front_is_up {
                    Vector3::y_axis()
                } else {
                    -Vector3::y_axis()
                });
                (n_circumference_vertices + 1) as usize
            ]);
        };

        if abs_diff_ne!(bottom_diameter, F::ZERO) {
            create_horizontal_disk(bottom_radius, -half_length, false);
        }

        if abs_diff_ne!(top_diameter, F::ZERO) {
            create_horizontal_disk(top_radius, half_length, true);
        }

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

        let radius = F::ONE_HALF;

        let n_circumference_vertices = 2 * n_rings + 2;

        let mut positions = Vec::with_capacity(n_circumference_vertices * n_rings + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(6 * n_circumference_vertices * n_rings);

        let n_rings = u32::try_from(n_rings).unwrap();
        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let delta_phi = F::TWO_PI / F::from_u32(n_circumference_vertices).unwrap();
        let delta_theta = <F as Float>::PI / F::from_u32(n_rings + 1).unwrap();

        // Top vertex
        positions.push(pos![F::ZERO, radius, F::ZERO]);
        normal_vectors.push(normal!(Vector3::y_axis()));

        // Bottom vertex
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

    /// Creates a mesh representing a hemisphere with diameter 1.0, with the
    /// disk lying in the xz-plane and centered at the origin. `n_rings` is the
    /// number of horizontal circular cross-sections that vertices will be
    /// generated around. The number of vertices that will be generated around
    /// each ring increases in proportion to `n_rings` to maintain an
    /// approximately uniform resolution.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If `n_rings` is zero.
    pub fn create_hemisphere(n_rings: usize) -> Self {
        assert!(n_rings > 0, "Tried to create hemisphere mesh with no rings");

        let radius = F::ONE_HALF;

        let n_circumference_vertices = 4 * n_rings + 2;

        let mut positions = Vec::with_capacity(n_circumference_vertices * (n_rings + 1) + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(6 * n_circumference_vertices * n_rings);

        let n_rings = u32::try_from(n_rings).unwrap();
        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let delta_phi = F::TWO_PI / F::from_u32(n_circumference_vertices).unwrap();
        let delta_theta = <F as Float>::FRAC_PI_2 / F::from_u32(n_rings).unwrap();

        // Top vertex
        positions.push(pos![F::ZERO, radius, F::ZERO]);
        normal_vectors.push(normal!(Vector3::y_axis()));

        // Vertex at center of disk
        positions.push(pos![F::ZERO, F::ZERO, F::ZERO]);
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

        // Repeat positions at the equator
        positions.extend_from_within(positions.len() - n_circumference_vertices as usize..);

        // Use normal vectors appropriate for the disk for the repeated
        // equatorial positions
        normal_vectors.extend_from_slice(&vec![
            normal!(-Vector3::y_axis());
            n_circumference_vertices as usize
        ]);

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

        // Bottom disk
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
