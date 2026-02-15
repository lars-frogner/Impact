//! Generation of meshes representing geometrical objects.

use crate::{
    FrontFaceSide, LineSegmentMesh, LineSegmentMeshDirtyMask, TriangleMesh, TriangleMeshDirtyMask,
    VertexColor,
};
use approx::{abs_diff_eq, abs_diff_ne};
use impact_math::{
    consts::f32::{FRAC_PI_2, PI, TWO_PI},
    point::Point3C,
    quaternion::UnitQuaternion,
    transform::Similarity3,
    vector::{UnitVector3, UnitVector3C, Vector3, Vector3C},
};

macro_rules! pos {
    [$x:expr, $y:expr, $z:expr] => {
        $crate::VertexPosition(Point3C::new($x, $y, $z))
    };
    ($point:expr) => {
        $crate::VertexPosition($point)
    };
}

macro_rules! normal {
    ($normal:expr) => {
        $crate::VertexNormalVector($normal)
    };
}

impl TriangleMesh {
    /// Creates a mesh with vertex positions directly in clip space coordinates,
    /// consisting of two triangles at zero depth that will exactly fill the
    /// view.
    pub fn create_screen_filling_quad() -> Self {
        let positions = vec![
            pos![-1.0, -1.0, 0.0],
            pos![1.0, -1.0, 0.0],
            pos![1.0, 1.0, 0.0],
            pos![-1.0, 1.0, 0.0],
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
    pub fn create_rectangle(extent_x: f32, extent_z: f32) -> Self {
        assert!(
            extent_x >= 0.0,
            "Tried to create rectangle mesh with negative x-extent"
        );
        assert!(
            extent_z >= 0.0,
            "Tried to create rectangle mesh with negative y-extent"
        );

        let hex = extent_x * 0.5;
        let hez = extent_z * 0.5;

        let positions = vec![
            pos![-hex, 0.0, -hez],
            pos![hex, 0.0, -hez],
            pos![hex, 0.0, hez],
            pos![-hex, 0.0, hez],
        ];

        let normal_vectors = vec![normal![UnitVector3C::unit_y()]; 4];

        let indices = vec![0, 3, 1, 1, 3, 2];

        Self::new(
            positions,
            normal_vectors,
            Vec::new(),
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
        extent_x: f32,
        extent_y: f32,
        extent_z: f32,
        front_face_side: FrontFaceSide,
    ) -> Self {
        assert!(
            extent_x >= 0.0,
            "Tried to create box mesh with negative x-extent"
        );
        assert!(
            extent_y >= 0.0,
            "Tried to create box mesh with negative y-extent"
        );
        assert!(
            extent_z >= 0.0,
            "Tried to create box mesh with negative z-extent"
        );

        let hw = extent_x * 0.5;
        let hh = extent_y * 0.5;
        let hd = extent_z * 0.5;

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
        normal_vectors.extend_from_slice(&[normal![-UnitVector3C::unit_x()]; 4]);
        add_face_indices();

        // Right face
        positions.extend_from_slice(&[
            pos![hw, -hh, -hd],
            pos![hw, -hh, hd],
            pos![hw, hh, hd],
            pos![hw, hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![UnitVector3C::unit_x()]; 4]);
        add_face_indices();

        // Bottom face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![-hw, -hh, hd],
            pos![hw, -hh, hd],
            pos![hw, -hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![-UnitVector3C::unit_y()]; 4]);
        add_face_indices();

        // Top face
        positions.extend_from_slice(&[
            pos![-hw, hh, -hd],
            pos![hw, hh, -hd],
            pos![hw, hh, hd],
            pos![-hw, hh, hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![UnitVector3C::unit_y()]; 4]);
        add_face_indices();

        // Front face
        positions.extend_from_slice(&[
            pos![-hw, -hh, -hd],
            pos![hw, -hh, -hd],
            pos![hw, hh, -hd],
            pos![-hw, hh, -hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![-UnitVector3C::unit_z()]; 4]);
        add_face_indices();

        // Back face
        positions.extend_from_slice(&[
            pos![-hw, -hh, hd],
            pos![-hw, hh, hd],
            pos![hw, hh, hd],
            pos![hw, -hh, hd],
        ]);
        normal_vectors.extend_from_slice(&[normal![UnitVector3C::unit_z()]; 4]);
        add_face_indices();

        Self::new(
            positions,
            normal_vectors,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a cylinder with the given length and
    /// diameter, with the length axis aligned with the y-axis and with the
    /// bottom centered at the origin. `n_circumference_vertices` is the number
    /// of vertices to use for representing a circular cross-section of the
    /// cylinder.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If any of the given extents are negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_cylinder(length: f32, diameter: f32, n_circumference_vertices: usize) -> Self {
        Self::create_circular_frustum(length, diameter, diameter, n_circumference_vertices)
    }

    /// Creates a mesh representing a cone with the given length and maximum
    /// diameter, pointing along the positive y-direction and with the bottom
    /// centered at the origin. `n_circumference_vertices` is the number of
    /// vertices to use for representing a circular cross-section of the
    /// cone.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If any of the given extents are negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_cone(length: f32, max_diameter: f32, n_circumference_vertices: usize) -> Self {
        Self::create_circular_frustum(length, max_diameter, 0.0, n_circumference_vertices)
    }

    /// Creates a mesh representing a y-axis aligned circular frustum with the
    /// given length, bottom diameter and top diameter, with the bottom centered
    /// at the origin. `n_circumference_vertices` is the number of vertices
    /// to use for representing a circular cross-section of the frustum.
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
        length: f32,
        bottom_diameter: f32,
        top_diameter: f32,
        n_circumference_vertices: usize,
    ) -> Self {
        assert!(
            length >= 0.0,
            "Tried to create circular frustum mesh with negative length"
        );
        assert!(
            bottom_diameter >= 0.0,
            "Tried to create circular frustum mesh with negative bottom diameter"
        );
        assert!(
            top_diameter >= 0.0,
            "Tried to create circular frustum mesh with negative top diameter"
        );
        assert!(
            n_circumference_vertices >= 2,
            "Tried to create circular frustum mesh with fewer than two vertices around circumference"
        );

        let bottom_radius = bottom_diameter * 0.5;
        let top_radius = top_diameter * 0.5;

        let mut positions = Vec::with_capacity(4 * n_circumference_vertices + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(12 * n_circumference_vertices);

        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let angle_between_vertices = TWO_PI / n_circumference_vertices as f32;

        let tan_slope_angle = if abs_diff_eq!(length, 0.0) {
            0.0
        } else {
            (bottom_radius - top_radius) / length
        };
        let cos_slope_angle = 1.0 / f32::sqrt(1.0 + f32::powi(tan_slope_angle, 2));
        let sin_slope_angle = cos_slope_angle * tan_slope_angle;

        // First bottom side vertex
        let bottom_pos = pos![bottom_radius, 0.0, 0.0];
        positions.push(bottom_pos);

        // First top side vertex
        let top_pos = pos![top_radius, length, 0.0];
        positions.push(top_pos);

        // Normal direction at first side vertices
        let normal_direction = normal!(UnitVector3C::unchecked_from(Vector3C::new(
            cos_slope_angle,
            sin_slope_angle,
            0.0
        )));
        normal_vectors.push(normal_direction);
        normal_vectors.push(normal_direction);

        let mut idx = 1;

        let mut polar_angle = angle_between_vertices;

        for _ in 1..n_circumference_vertices {
            let cos_polar_angle = f32::cos(polar_angle);
            let sin_polar_angle = f32::sin(polar_angle);

            let bottom_pos = pos![
                bottom_radius * cos_polar_angle,
                0.0,
                bottom_radius * sin_polar_angle
            ];
            positions.push(bottom_pos);

            let top_pos = pos![
                top_radius * cos_polar_angle,
                length,
                top_radius * sin_polar_angle
            ];
            positions.push(top_pos);

            let normal_direction = normal!(UnitVector3C::unchecked_from(Vector3C::new(
                cos_polar_angle * cos_slope_angle,
                sin_slope_angle,
                sin_polar_angle * cos_slope_angle
            )));
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
            positions.push(pos![0.0, y, 0.0]);

            idx += 1;
            let center_idx = idx;

            // First side vertex
            positions.push(pos![radius, y, 0.0]);

            idx += 1;

            let mut polar_angle = angle_between_vertices;

            for _ in 1..n_circumference_vertices {
                let cos_polar_angle = f32::cos(polar_angle);
                let sin_polar_angle = f32::sin(polar_angle);

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
                    UnitVector3C::unit_y()
                } else {
                    -UnitVector3C::unit_y()
                });
                (n_circumference_vertices + 1) as usize
            ]);
        };

        if abs_diff_ne!(bottom_diameter, 0.0) {
            create_horizontal_disk(bottom_radius, 0.0, false);
        }

        if abs_diff_ne!(top_diameter, 0.0) {
            create_horizontal_disk(top_radius, length, true);
        }

        Self::new(
            positions,
            normal_vectors,
            Vec::new(),
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

        let radius = 0.5;

        let n_circumference_vertices = 2 * n_rings + 2;

        let mut positions = Vec::with_capacity(n_circumference_vertices * n_rings + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(6 * n_circumference_vertices * n_rings);

        let n_rings = u32::try_from(n_rings).unwrap();
        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let delta_phi = TWO_PI / n_circumference_vertices as f32;
        let delta_theta = PI / (n_rings + 1) as f32;

        // Top vertex
        positions.push(pos![0.0, radius, 0.0]);
        normal_vectors.push(normal!(UnitVector3C::unit_y()));

        // Bottom vertex
        positions.push(pos![0.0, -radius, 0.0]);
        normal_vectors.push(normal!(-UnitVector3C::unit_y()));

        let mut theta = delta_theta;

        for _ in 0..n_rings {
            let sin_theta = f32::sin(theta);
            let cos_theta = f32::cos(theta);
            let y = radius * cos_theta;

            let mut phi = 0.0;

            for _ in 0..n_circumference_vertices {
                let cos_phi_sin_theta = f32::cos(phi) * sin_theta;
                let sin_phi_sin_theta = f32::sin(phi) * sin_theta;

                positions.push(pos![
                    radius * cos_phi_sin_theta,
                    y,
                    radius * sin_phi_sin_theta
                ]);
                normal_vectors.push(normal!(UnitVector3C::unchecked_from(Vector3C::new(
                    cos_phi_sin_theta,
                    cos_theta,
                    sin_phi_sin_theta
                ))));

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
            normal_vectors,
            Vec::new(),
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

        let radius = 0.5;

        let n_circumference_vertices = 4 * n_rings + 2;

        let mut positions = Vec::with_capacity(n_circumference_vertices * (n_rings + 1) + 2);
        let mut normal_vectors = Vec::with_capacity(positions.capacity());
        let mut indices = Vec::with_capacity(6 * n_circumference_vertices * n_rings);

        let n_rings = u32::try_from(n_rings).unwrap();
        let n_circumference_vertices = u32::try_from(n_circumference_vertices).unwrap();

        let delta_phi = TWO_PI / n_circumference_vertices as f32;
        let delta_theta = FRAC_PI_2 / n_rings as f32;

        // Top vertex
        positions.push(pos![0.0, radius, 0.0]);
        normal_vectors.push(normal!(UnitVector3C::unit_y()));

        // Vertex at center of disk
        positions.push(pos![0.0, 0.0, 0.0]);
        normal_vectors.push(normal!(-UnitVector3C::unit_y()));

        let mut theta = delta_theta;

        for _ in 0..n_rings {
            let sin_theta = f32::sin(theta);
            let cos_theta = f32::cos(theta);
            let y = radius * cos_theta;

            let mut phi = 0.0;

            for _ in 0..n_circumference_vertices {
                let cos_phi_sin_theta = f32::cos(phi) * sin_theta;
                let sin_phi_sin_theta = f32::sin(phi) * sin_theta;

                positions.push(pos![
                    radius * cos_phi_sin_theta,
                    y,
                    radius * sin_phi_sin_theta
                ]);
                normal_vectors.push(normal!(UnitVector3C::unchecked_from(Vector3C::new(
                    cos_phi_sin_theta,
                    cos_theta,
                    sin_phi_sin_theta
                ))));

                phi += delta_phi;
            }

            theta += delta_theta;
        }

        // Repeat positions at the equator
        positions.extend_from_within(positions.len() - n_circumference_vertices as usize..);

        // Use normal vectors appropriate for the disk for the repeated
        // equatorial positions
        normal_vectors.extend_from_slice(&vec![
            normal!(-UnitVector3C::unit_y());
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
            normal_vectors,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            indices,
        )
    }

    /// Creates a mesh representing a capsule with the given segment length
    /// (distance between cap sphere centers) and radius, with the axis aligned
    /// with the y-axis and centered on the origin. diameter 1.0, with the disk
    /// lying in the xz-plane and centered at the origin.
    /// `n_circumference_vertices` is the number of vertices to use for
    /// representing a circular cross-section of the capsule's cylinder. The
    /// number of horizontal circular cross-sections that vertices will be
    /// generated around for the spherical caps increases in proportion to
    /// `n_circumference_vertices` to maintain an approximately uniform
    /// resolution.
    ///
    /// The generated mesh will contain positions and normal vectors.
    ///
    /// # Panics
    /// - If the segment length or radius is negative.
    /// - If `n_circumference_vertices` is smaller than 2.
    pub fn create_capsule(
        segment_length: f32,
        radius: f32,
        n_circumference_vertices: usize,
    ) -> Self {
        let diameter = 2.0 * radius;

        let mut mesh = Self::create_cylinder(segment_length, diameter, n_circumference_vertices);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        mesh.translate(
            &Vector3::new(0.0, -0.5 * segment_length, 0.0),
            &mut dirty_mask,
        );

        let n_rings = ((n_circumference_vertices - 2) / 4).max(4);

        let mut cap_mesh = Self::create_hemisphere(n_rings);
        cap_mesh.scale(diameter, &mut dirty_mask);
        cap_mesh.translate(
            &Vector3::new(0.0, 0.5 * segment_length, 0.0),
            &mut dirty_mask,
        );

        mesh.merge_with(&cap_mesh, &mut dirty_mask);

        cap_mesh.rotate(
            &UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI),
            &mut dirty_mask,
        );

        mesh.merge_with(&cap_mesh, &mut dirty_mask);

        mesh
    }

    /// Creates a mesh representing a sphere with radius 1.0, centered at the
    /// origin, with triangle front faces pointing inward. `n_rings` is the
    /// number of horizontal circular cross-sections that vertices will be
    /// generated around. The number of vertices that will be generated
    /// around each ring increases in proportion to `n_rings` to maintain an
    /// approximately uniform resolution.
    ///
    /// The generated mesh will only contain positions.
    ///
    /// # Panics
    /// - If `n_rings` is zero.
    pub fn create_spherical_light_volume(n_rings: usize) -> TriangleMesh {
        let mut mesh = Self::create_sphere(n_rings);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        // Normal vectors are not needed for light volumes
        mesh.remove_normal_vectors(&mut dirty_mask);

        // Scale to unit radius
        mesh.scale(2.0, &mut dirty_mask);

        // Flip triangle winding order to make the front faces point inward
        mesh.flip_triangle_winding_order(&mut dirty_mask);

        mesh
    }

    /// Creates a mesh representing a vertical square with the given extent
    /// along the x- and y-axis, the front face pointing in the z-direction,
    /// centered on the origin and with all vertices having the given color.
    ///
    /// The generated mesh will only contain positions and colors.
    pub fn create_vertical_square_with_color(extent: f32, color: VertexColor) -> Self {
        let mut square = Self::create_rectangle(extent, extent);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        square.remove_normal_vectors(&mut dirty_mask);
        square.rotate(
            &UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), FRAC_PI_2),
            &mut dirty_mask,
        );
        square.set_same_color(color, &mut dirty_mask);

        square
    }

    /// Creates a mesh representing a cube with the given extent, centered on
    /// the origin and with the width, height and depth axes aligned with the
    /// x-, y- and z-axis. The six given colors will be assigned to the left,
    /// right, bottom, top, front and back face respectively.
    ///
    /// The generated mesh will only contain positions and colors.
    pub fn create_cube_with_face_colors(extent: f32, face_colors: &[VertexColor; 6]) -> Self {
        let mut cube = Self::create_box(extent, extent, extent, FrontFaceSide::Outside);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        cube.remove_normal_vectors(&mut dirty_mask);

        let mut colors = Vec::with_capacity(cube.n_vertices());
        for face_color in face_colors {
            colors.extend_from_slice(&[*face_color; 4]);
        }
        cube.set_colors(colors, &mut dirty_mask);

        cube
    }

    /// Creates a mesh representing a cube with extent 1.0, centered at the
    /// origin, with all vertices having the given color.
    ///
    /// The generated mesh will only contain positions and colors.
    pub fn create_unit_cube_with_color(color: VertexColor) -> Self {
        let mut cube = Self::create_box(1.0, 1.0, 1.0, FrontFaceSide::Outside);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        cube.remove_normal_vectors(&mut dirty_mask);
        cube.set_same_color(color, &mut dirty_mask);

        cube
    }

    /// Creates a mesh representing a sphere with radius 1.0, centered at the
    /// origin, with all vertices having the given color.
    ///
    /// The generated mesh will only contain positions and colors.
    ///
    /// See [`Self::create_sphere`] for an explanation of the `n_rings`
    /// argument.
    ///
    /// # Panics
    /// - If `n_rings` is zero.
    pub fn create_unit_sphere_with_color(n_rings: usize, color: VertexColor) -> Self {
        let mut sphere = Self::create_sphere(n_rings);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        sphere.remove_normal_vectors(&mut dirty_mask);
        sphere.scale(2.0, &mut dirty_mask);
        sphere.set_same_color(color, &mut dirty_mask);

        sphere
    }

    /// Creates a mesh representing the boundary of a cubic voxel chunk with the
    /// given extent. The lower corner of the cube is at the origin, and the
    /// width, height and depth axes are aligned with the x-, y- and z-axis.
    ///
    /// The generated mesh will only contain positions and colors.
    pub fn create_voxel_chunk_cube_with_color(extent: f32, color: VertexColor) -> Self {
        let mut cube = Self::create_box(extent, extent, extent, FrontFaceSide::Outside);
        let mut dirty_mask = TriangleMeshDirtyMask::empty();

        cube.remove_normal_vectors(&mut dirty_mask);
        cube.translate(&Vector3::same(0.5 * extent), &mut dirty_mask);
        cube.set_same_color(color, &mut dirty_mask);

        cube
    }
}

impl LineSegmentMesh {
    /// Creates a mesh containing an arrow going from the origin to (0, 1, 0).
    /// The two line segments making up the arrow head lie in the xy-plane.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_unit_arrow_y() -> Self {
        let arrow_length = 0.1;
        let arrow_width = 0.05;

        let positions = vec![
            pos![0.0, 0.0, 0.0],
            pos![0.0, 1.0, 0.0],
            pos![0.0, 1.0, 0.0],
            pos![-arrow_width, 1.0 - arrow_length, 0.0],
            pos![0.0, 1.0, 0.0],
            pos![arrow_width, 1.0 - arrow_length, 0.0],
        ];

        Self::new(positions, Vec::new())
    }

    /// Creates a mesh with three line segments corresponding to the x, y and z
    /// unit vectors rooted at the origin, respectively colored red, green and
    /// blue.
    pub fn create_reference_frame_axes() -> Self {
        let positions = vec![
            pos![0.0, 0.0, 0.0],
            pos![1.0, 0.0, 0.0],
            pos![0.0, 0.0, 0.0],
            pos![0.0, 1.0, 0.0],
            pos![0.0, 0.0, 0.0],
            pos![0.0, 0.0, 1.0],
        ];

        let colors = vec![
            VertexColor::RED,
            VertexColor::RED,
            VertexColor::GREEN,
            VertexColor::GREEN,
            VertexColor::BLUE,
            VertexColor::BLUE,
        ];

        Self::new(positions, colors)
    }

    /// Creates a mesh containing the edges of the six frusta of a cubemap. The
    /// frusta are aligned with the Cartesian axes, the far planes are at
    /// distance one and the near planes are at distance zero.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_unit_cubemap_frusta() -> Self {
        let mut down_diagonals = Self::create_baseless_unit_pyramid();
        let mut dirty_mask = LineSegmentMeshDirtyMask::empty();
        down_diagonals.translate(&Vector3::new(0.0, -1.0, 0.0), &mut dirty_mask);

        let mut up_diagonals = Self::create_baseless_unit_pyramid();
        up_diagonals.transform(
            &Similarity3::from_parts(
                Vector3::new(0.0, 1.0, 0.0),
                UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI),
                1.0,
            ),
            &mut LineSegmentMeshDirtyMask::empty(),
        );

        let mut far_plane_edges = Self::create_unit_cube();
        far_plane_edges.scale(2.0, &mut LineSegmentMeshDirtyMask::empty());

        let mut frusta = down_diagonals;
        frusta.merge_with(&up_diagonals, &mut dirty_mask);
        frusta.merge_with(&far_plane_edges, &mut dirty_mask);

        frusta
    }

    /// Creates a mesh containing the edges of the unit cube centered on the
    /// origin.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_unit_cube() -> Self {
        let corners = [
            [
                [pos![-0.5, -0.5, -0.5], pos![-0.5, -0.5, 0.5]],
                [pos![-0.5, 0.5, -0.5], pos![-0.5, 0.5, 0.5]],
            ],
            [
                [pos![0.5, -0.5, -0.5], pos![0.5, -0.5, 0.5]],
                [pos![0.5, 0.5, -0.5], pos![0.5, 0.5, 0.5]],
            ],
        ];

        let positions = vec![
            // Bottom face edges
            corners[0][0][0],
            corners[0][0][1],
            corners[0][0][1],
            corners[1][0][1],
            corners[1][0][1],
            corners[1][0][0],
            corners[1][0][0],
            corners[0][0][0],
            // Top face edges
            corners[0][1][0],
            corners[0][1][1],
            corners[0][1][1],
            corners[1][1][1],
            corners[1][1][1],
            corners[1][1][0],
            corners[1][1][0],
            corners[0][1][0],
            // Vertical edges connecting bottom to top
            corners[0][0][0],
            corners[0][1][0],
            corners[0][0][1],
            corners[0][1][1],
            corners[1][0][1],
            corners[1][1][1],
            corners[1][0][0],
            corners[1][1][0],
        ];

        Self::new(positions, Vec::new())
    }

    /// Creates a mesh containing the edges of a vertical pyramid whose base has
    /// unit extents and is centered on the origin.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_unit_pyramid() -> Self {
        let positions = vec![
            pos![0.0, 1.0, 0.0],
            pos![-1.0, 0.0, -1.0],
            pos![0.0, 1.0, 0.0],
            pos![-1.0, 0.0, 1.0],
            pos![0.0, 1.0, 0.0],
            pos![1.0, 0.0, -1.0],
            pos![0.0, 1.0, 0.0],
            pos![1.0, 0.0, 1.0],
            pos![-1.0, 0.0, -1.0],
            pos![-1.0, 0.0, 1.0],
            pos![-1.0, 0.0, 1.0],
            pos![1.0, 0.0, 1.0],
            pos![1.0, 0.0, 1.0],
            pos![1.0, 0.0, -1.0],
            pos![1.0, 0.0, -1.0],
            pos![-1.0, 0.0, -1.0],
        ];

        Self::new(positions, Vec::new())
    }

    /// Creates a mesh containing the diagonal edges of a vertical pyramid
    /// whose base has unit extents and is centered on the origin. The base
    /// edges are not included.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_baseless_unit_pyramid() -> Self {
        let positions = vec![
            pos![0.0, 1.0, 0.0],
            pos![-1.0, 0.0, -1.0],
            pos![0.0, 1.0, 0.0],
            pos![-1.0, 0.0, 1.0],
            pos![0.0, 1.0, 0.0],
            pos![1.0, 0.0, -1.0],
            pos![0.0, 1.0, 0.0],
            pos![1.0, 0.0, 1.0],
        ];

        Self::new(positions, Vec::new())
    }

    /// Creates a mesh containing the three circles formed by the intersection
    /// of the three Cartesian coordinate planes with the unit radius circle
    /// centered on the origin. Each circle will consist of the given number of
    /// line segments.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_unit_sphere_great_circles(n_circumference_segments: usize) -> Self {
        let xz_circle = Self::create_horizontal_unit_circle(n_circumference_segments);
        let mut dirty_mask = LineSegmentMeshDirtyMask::empty();

        let mut xy_circle = Self::new(xz_circle.positions().to_vec(), Vec::new());
        xy_circle.rotate(
            &UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), FRAC_PI_2),
            &mut dirty_mask,
        );

        let mut yz_circle = Self::new(xz_circle.positions().to_vec(), Vec::new());
        yz_circle.rotate(
            &UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), FRAC_PI_2),
            &mut dirty_mask,
        );

        let mut sphere = xz_circle;
        sphere.merge_with(&xy_circle, &mut dirty_mask);
        sphere.merge_with(&yz_circle, &mut dirty_mask);

        sphere
    }

    /// Creates a mesh corresponding to a unit radius circle centered on the
    /// origin in the xz-plane, with the given number of line segment.
    ///
    /// The generated mesh will only contain positions.
    pub fn create_horizontal_unit_circle(n_segments: usize) -> Self {
        let mut positions = Vec::with_capacity(2 * n_segments);

        let angle_between_vertices = TWO_PI / n_segments as f32;

        positions.push(pos![1.0, 0.0, 0.0]);

        let mut polar_angle = angle_between_vertices;

        for _ in 1..n_segments {
            let cos_polar_angle = f32::cos(polar_angle);
            let sin_polar_angle = f32::sin(polar_angle);

            let position = pos![cos_polar_angle, 0.0, sin_polar_angle];
            positions.push(position);
            positions.push(position);

            polar_angle += angle_between_vertices;
        }

        positions.push(positions[0]);

        Self::new(positions, Vec::new())
    }
}
