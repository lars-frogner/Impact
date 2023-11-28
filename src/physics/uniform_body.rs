//! Representation of uniform bodies.

use crate::{
    geometry::{FrontFaceSide, TriangleMesh},
    num::Float,
};
use approx::AbsDiffEq;
use nalgebra::{point, vector, Matrix3, Point3, Similarity3, Vector3};

/// A uniform body represented by a closed [`TriangleMesh`].
#[derive(Debug)]
pub struct UniformBodyMesh<F: Float> {
    triangle_mesh: TriangleMesh<F>,
    volume: F,
    center_of_mass: Point3<F>,
}

/// The inertia tensor for a uniform body.
#[derive(Debug, PartialEq)]
pub struct InertiaTensor<F: Float> {
    matrix: Matrix3<F>,
}

impl<F: Float> UniformBodyMesh<F> {
    /// Creates a uniformly dense body represented by the given triangle mesh,
    /// which is assumed closed.
    pub fn from_triangle_mesh(triangle_mesh: TriangleMesh<F>) -> Self {
        let (volume, center_of_mass) =
            compute_triangle_mesh_volume_and_center_of_mass(&triangle_mesh);

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Creates a mesh representing a uniformly dense box with the given
    /// extents, centered at the origin and with the width, height and depth
    /// axes aligned with the x-, y- and z-axis.
    ///
    /// # Panics
    /// See [`TriangleMesh::create_box`].
    pub fn create_box(extent_x: F, extent_y: F, extent_z: F) -> Self {
        let triangle_mesh =
            TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);

        let volume = compute_box_volume(extent_x, extent_y, extent_z);

        let center_of_mass = Point3::origin();

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Creates a mesh representing a uniformly dense cylinder with the given
    /// length and diameter, centered at the origin and with the length axis
    /// aligned with the y-axis. `n_circumference_vertices` is the number of
    /// vertices to use for representing a circular cross-section of the
    /// cylinder.
    ///
    /// # Panics
    /// See [`TriangleMesh::create_cylinder`].
    pub fn create_cylinder(length: F, diameter: F, n_circumference_vertices: usize) -> Self {
        let triangle_mesh =
            TriangleMesh::create_cylinder(length, diameter, n_circumference_vertices);

        let radius = diameter * F::ONE_HALF;
        let volume = compute_cylinder_volume(radius, length);

        let center_of_mass = Point3::origin();

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Creates a mesh representing a uniformly dense cone with the given length
    /// and maximum diameter, centered at the origin and pointing along the
    /// positive y-direction. `n_circumference_vertices` is the number of
    /// vertices to use for representing a circular cross-section of the cone.
    ///
    /// # Panics
    /// See [`TriangleMesh::create_cone`].
    pub fn create_cone(length: F, max_diameter: F, n_circumference_vertices: usize) -> Self {
        let triangle_mesh =
            TriangleMesh::create_cone(length, max_diameter, n_circumference_vertices);

        let max_radius = max_diameter * F::ONE_HALF;
        let volume = compute_cone_volume(max_radius, length);

        // The center of mass is one quarter of the way up from the center of
        // the disk toward the point
        let center_of_mass = point![F::ZERO, -length * F::ONE_FOURTH, F::ZERO];

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Creates a mesh representing a uniformly dense sphere with diameter 1.0,
    /// centered at the origin. `n_rings` is the number of horizontal circular
    /// cross-sections that vertices will be generated around. The number of
    /// vertices that will be generated around each ring increases in proportion
    /// to `n_rings` to maintain an approximately uniform resolution.
    ///
    /// # Panics
    /// See [`TriangleMesh::create_sphere`].
    pub fn create_sphere(n_rings: usize) -> Self {
        let triangle_mesh = TriangleMesh::create_sphere(n_rings);

        let radius = F::ONE_HALF;
        let volume = compute_sphere_volume(radius);

        let center_of_mass = Point3::origin();

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Creates a mesh representing a hemisphere with diameter 1.0, with the
    /// disk lying in the xz-plane and centered at the origin. `n_rings` is the
    /// number of horizontal circular cross-sections that vertices will be
    /// generated around. The number of vertices that will be generated around
    /// each ring increases in proportion to `n_rings` to maintain an
    /// approximately uniform resolution.
    ///
    /// # Panics
    /// See [`TriangleMesh::create_hemisphere`].
    pub fn create_hemisphere(n_rings: usize) -> Self {
        let triangle_mesh = TriangleMesh::create_hemisphere(n_rings);

        let radius = F::ONE_HALF;
        let volume = compute_hemisphere_volume(radius);

        // The center of mass is (3/8) of the way up from the center of the disk
        // toward the top
        let center_of_mass = point![F::ZERO, (F::THREE / F::EIGHT) * radius, F::ZERO];

        Self {
            triangle_mesh,
            volume,
            center_of_mass,
        }
    }

    /// Returns a reference to the [`TriangleMesh`] representing the uniform
    /// body.
    pub fn triangle_mesh(&self) -> &TriangleMesh<F> {
        &self.triangle_mesh
    }

    /// Returns the volume of the uniform body.
    pub fn volume(&self) -> F {
        self.volume
    }

    /// Returns the center of mass of the uniform body.
    pub fn center_of_mass(&self) -> &Point3<F> {
        &self.center_of_mass
    }

    /// Applies the given similarity transform to the uniform body.
    pub fn transform(&mut self, transform: &Similarity3<F>) {
        self.triangle_mesh.transform(transform);

        self.volume *= F::powi(transform.scaling(), 3);

        self.center_of_mass = transform.transform_point(&self.center_of_mass);
    }
}

impl<F: Float> InertiaTensor<F> {
    fn from_matrix(inertia_tensor: Matrix3<F>) -> Self {
        Self {
            matrix: inertia_tensor,
        }
    }
}

impl<F: AbsDiffEq + Float> AbsDiffEq for InertiaTensor<F>
where
    F::Epsilon: Copy,
{
    type Epsilon = F::Epsilon;

    fn default_epsilon() -> F::Epsilon {
        F::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: F::Epsilon) -> bool {
        Matrix3::abs_diff_eq(&self.matrix, &other.matrix, epsilon)
    }
}

pub fn compute_box_volume<F: Float>(extent_x: F, extent_y: F, extent_z: F) -> F {
    extent_x * extent_y * extent_z
}

pub fn compute_cylinder_volume<F: Float>(radius: F, length: F) -> F {
    <F as Float>::PI * F::powi(radius, 2) * length
}

pub fn compute_cone_volume<F: Float>(max_radius: F, length: F) -> F {
    compute_cylinder_volume(max_radius, length) * F::ONE_THIRD
}

pub fn compute_sphere_volume<F: Float>(radius: F) -> F {
    (F::FOUR / F::THREE) * <F as Float>::PI * F::powi(radius, 3)
}

pub fn compute_hemisphere_volume<F: Float>(radius: F) -> F {
    compute_sphere_volume(radius) * F::ONE_HALF
}

/// Computes the volume of the given triangle mesh, using the method described
/// in Eberly (2004). The mesh is assumed closed.
pub fn compute_triangle_mesh_volume<F: Float>(mesh: &TriangleMesh<F>) -> F {
    let mut volume = F::ZERO;

    for [vertex_0, vertex_1, vertex_2] in mesh.triangle_vertex_positions() {
        volume += compute_volume_contribution_for_triangle(vertex_0, vertex_1, vertex_2);
    }

    volume *= F::ONE_HALF * F::ONE_THIRD;

    volume
}

/// Computes the volume and center of mass of a uniformly dense body represented
/// by the given triangle mesh, using the method described in Eberly (2004). The
/// mesh is assumed closed.
pub fn compute_triangle_mesh_volume_and_center_of_mass<F: Float>(
    mesh: &TriangleMesh<F>,
) -> (F, Point3<F>) {
    let mut volume = F::ZERO;
    let mut first_moments = Vector3::zeros();

    for [vertex_0, vertex_1, vertex_2] in mesh.triangle_vertex_positions() {
        let (zeroth_moment_contrib, first_moment_contrib) =
            compute_zeroth_and_first_moment_contributions_for_triangle(
                vertex_0, vertex_1, vertex_2,
            );

        volume += zeroth_moment_contrib;
        first_moments += first_moment_contrib;
    }

    volume *= F::ONE_HALF * F::ONE_THIRD;
    first_moments *= F::ONE_HALF * F::ONE_HALF * F::ONE_HALF * F::ONE_THIRD;

    let center_of_mass = Point3::from(first_moments / volume);

    (volume, center_of_mass)
}

/// Computes the center of mass of a uniformly dense body represented by the
/// given triangle mesh, using the method described in Eberly (2004). The mesh
/// is assumed closed.
pub fn compute_triangle_mesh_center_of_mass<F: Float>(mesh: &TriangleMesh<F>) -> Point3<F> {
    compute_triangle_mesh_volume_and_center_of_mass(mesh).1
}

pub fn compute_triangle_mesh_inertia_tensor<F: Float>(mesh: &TriangleMesh<F>) -> InertiaTensor<F> {
    let mut volume = F::ZERO;
    let mut first_moments = Vector3::zeros();
    let mut diagonal_second_moments = Vector3::zeros();
    let mut mixed_second_moments = Vector3::zeros();

    for [vertex_0, vertex_1, vertex_2] in mesh.triangle_vertex_positions() {
        let (
            zeroth_moment_contrib,
            first_moment_contrib,
            diagonal_second_moment_contrib,
            mixed_second_moment_contrib,
        ) = compute_zeroth_first_and_second_moment_contributions_for_triangle(
            vertex_0, vertex_1, vertex_2,
        );

        volume += zeroth_moment_contrib;
        first_moments += first_moment_contrib;
        diagonal_second_moments += diagonal_second_moment_contrib;
        mixed_second_moments += mixed_second_moment_contrib;
    }

    volume *= F::ONE_HALF * F::ONE_THIRD;
    first_moments *= F::ONE_HALF * F::ONE_FOURTH * F::ONE_THIRD;
    diagonal_second_moments *= F::ONE_THIRD * F::ONE_FOURTH * F::ONE_FIFTH;
    mixed_second_moments *= F::ONE_HALF * F::ONE_THIRD * F::ONE_FOURTH * F::ONE_FIFTH;

    let center_of_mass = Point3::from(first_moments / volume);

    let squared_center_of_mass = center_of_mass.coords.component_mul(&center_of_mass.coords);

    let j_xx = (diagonal_second_moments.y + diagonal_second_moments.z)
        - volume * (squared_center_of_mass.y + squared_center_of_mass.z);
    let j_yy = (diagonal_second_moments.z + diagonal_second_moments.x)
        - volume * (squared_center_of_mass.z + squared_center_of_mass.x);
    let j_zz = (diagonal_second_moments.x + diagonal_second_moments.y)
        - volume * (squared_center_of_mass.x + squared_center_of_mass.y);

    let j_xy = -(mixed_second_moments.x - volume * center_of_mass.x * center_of_mass.y);
    let j_yz = -(mixed_second_moments.y - volume * center_of_mass.y * center_of_mass.z);
    let j_zx = -(mixed_second_moments.z - volume * center_of_mass.z * center_of_mass.x);

    let inertia_tensor = Matrix3::from_columns(&[
        vector![j_xx, j_xy, j_zx],
        vector![j_xy, j_yy, j_yz],
        vector![j_zx, j_yz, j_zz],
    ]);

    InertiaTensor::from_matrix(inertia_tensor)
}

fn compute_volume_contribution_for_triangle<F: Float>(
    vertex_0: &Point3<F>,
    vertex_1: &Point3<F>,
    vertex_2: &Point3<F>,
) -> F {
    let edge_1_y = vertex_1.y - vertex_0.y;
    let edge_1_z = vertex_1.z - vertex_0.z;
    let edge_2_y = vertex_2.y - vertex_0.y;
    let edge_2_z = vertex_2.z - vertex_0.z;

    (edge_1_y * edge_2_z - edge_2_y * edge_1_z) * (vertex_0.x + vertex_1.x + vertex_2.x)
}

fn compute_zeroth_and_first_moment_contributions_for_triangle<F: Float>(
    vertex_0: &Point3<F>,
    vertex_1: &Point3<F>,
    vertex_2: &Point3<F>,
) -> (F, Vector3<F>) {
    let w_0 = vertex_0.coords;
    let w_1 = vertex_1.coords;
    let w_2 = vertex_2.coords;

    let tmp = w_0 + w_1;

    let f_1 = tmp + w_2;
    let f_2 = w_0.component_mul(&w_0) + w_1.component_mul(&tmp) + w_2.component_mul(&f_1);

    let edge_1 = vertex_1 - vertex_0;
    let edge_2 = vertex_2 - vertex_0;

    let edge_cross_prod = edge_1.cross(&edge_2);

    let zeroth_moment = edge_cross_prod.x * f_1.x;

    let first_moments = edge_cross_prod.component_mul(&f_2);

    (zeroth_moment, first_moments)
}

fn compute_zeroth_first_and_second_moment_contributions_for_triangle<F: Float>(
    vertex_0: &Point3<F>,
    vertex_1: &Point3<F>,
    vertex_2: &Point3<F>,
) -> (F, Vector3<F>, Vector3<F>, Vector3<F>) {
    let w_0 = vertex_0.coords;
    let w_1 = vertex_1.coords;
    let w_2 = vertex_2.coords;

    let tmp_0 = w_0 + w_1;
    let tmp_1 = w_0.component_mul(&w_0);
    let tmp_2 = tmp_1 + w_1.component_mul(&tmp_0);

    let f_1 = tmp_0 + w_2;
    let f_2 = tmp_2 + w_2.component_mul(&f_1);
    let f_3 = w_0.component_mul(&tmp_1) + w_1.component_mul(&tmp_2) + w_2.component_mul(&f_1);

    let g_0 = f_2 + w_0.component_mul(&(f_1 + w_0));
    let g_1 = f_2 + w_1.component_mul(&(f_1 + w_1));
    let g_2 = f_2 + w_2.component_mul(&(f_1 + w_2));

    let edge_1 = vertex_1 - vertex_0;
    let edge_2 = vertex_2 - vertex_0;

    let edge_cross_prod = edge_1.cross(&edge_2);

    let zeroth_moment = edge_cross_prod.x * f_1.x;

    let first_moments = edge_cross_prod.component_mul(&f_2);

    let diagonal_second_moments = edge_cross_prod.component_mul(&f_3);

    let mixed_second_moments = vector![
        edge_cross_prod.x * (w_0.y * g_0.x + w_1.y * g_1.x + w_2.y * g_2.x), // x²y
        edge_cross_prod.y * (w_0.z * g_0.y + w_1.z * g_1.y + w_2.z * g_2.y), // y²z
        edge_cross_prod.z * (w_0.x * g_0.z + w_1.x * g_1.z + w_2.x * g_2.z)  // z²x
    ];

    (
        zeroth_moment,
        first_moments,
        diagonal_second_moments,
        mixed_second_moments,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};
    use proptest::prelude::*;
    use std::{f64::consts, ops::Range};

    prop_compose! {
        fn similarity_transform_strategy(
            max_translation: f64,
            scaling_range: Range<f64>
        )(
            translation_x in -max_translation..max_translation,
            translation_y in -max_translation..max_translation,
            translation_z in -max_translation..max_translation,
            rotation_roll in 0.0..consts::TAU,
            rotation_pitch in -consts::FRAC_PI_2..consts::FRAC_PI_2,
            rotation_yaw in 0.0..consts::TAU,
            scaling in scaling_range,
        ) -> Similarity3<f64> {
            let translation = Translation3::new(translation_x, translation_y, translation_z);
            let rotation = UnitQuaternion::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw);
            Similarity3::from_parts(
                translation,
                rotation,
                scaling
            )
        }
    }

    proptest! {
        #[test]
        fn should_transform_cube_volume(transform in similarity_transform_strategy(1e6, 1e-6..1e6)) {
            let mut cube = UniformBodyMesh::create_box(1.0, 1.0, 1.0);
            let initial_volume = cube.volume();
            cube.transform(&transform);
            let volume_after_transforming = cube.volume();
            let correctly_transformed_volume = initial_volume * f64::powi(transform.scaling(), 3);
            prop_assert!(abs_diff_eq!(
                volume_after_transforming,
                correctly_transformed_volume,
                epsilon = 1e-9 * correctly_transformed_volume
            ));
        }
    }

    proptest! {
        #[test]
        fn should_transform_cube_center_of_mass(transform in similarity_transform_strategy(1e6, 1e-6..1e6)) {
            let mut cube = UniformBodyMesh::create_box(1.0, 1.0, 1.0);
            let initial_center_of_mass = *cube.center_of_mass();
            cube.transform(&transform);
            let center_of_mass_after_transforming = cube.center_of_mass();
            let correctly_transformed_center_of_mass = transform.transform_point(&initial_center_of_mass);
            prop_assert!(abs_diff_eq!(
                center_of_mass_after_transforming,
                &correctly_transformed_center_of_mass,
                epsilon = 1e-7 * correctly_transformed_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_box_mesh_volume(
            extent_x in 1e-6..1e6,
            extent_y in 1e-6..1e6,
            extent_z in 1e-6..1e6,
            transform in similarity_transform_strategy(1e6, 1e-6..1e6),
        ) {
            let mut box_ = UniformBodyMesh::create_box(extent_x, extent_y, extent_z);
            box_.transform(&transform);
            let correct_volume = box_.volume();
            let computed_volume = compute_triangle_mesh_volume(box_.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_volume,
                correct_volume,
                epsilon = 1e-9 * correct_volume
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn should_compute_cylinder_mesh_volume(
            length in 1e-6..1e6,
            diameter in 1e-6..1e6,
            transform in similarity_transform_strategy(1e6, 1e-6..1e6),
        ) {
            let mut cylinder = UniformBodyMesh::create_cylinder(length, diameter, 30);
            cylinder.transform(&transform);
            let correct_volume = cylinder.volume();
            let computed_volume = compute_triangle_mesh_volume(cylinder.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_volume,
                correct_volume,
                epsilon = 1e-2 * correct_volume
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]
        #[test]
        fn should_compute_sphere_mesh_volume(transform in similarity_transform_strategy(1e6, 1e-6..1e6)) {
            let mut sphere = UniformBodyMesh::create_sphere(20);
            sphere.transform(&transform);
            let correct_volume = sphere.volume();
            let computed_volume: f64 = compute_triangle_mesh_volume(sphere.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_volume,
                correct_volume,
                epsilon = 1e-2 * correct_volume
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_box_center_of_mass(
            extent_x in 1e-6..1e6,
            extent_y in 1e-6..1e6,
            extent_z in 1e-6..1e6,
            transform in similarity_transform_strategy(1e6, 1e-6..1e6),
        ) {
            let mut box_ = UniformBodyMesh::create_box(extent_x, extent_y, extent_z);
            box_.transform(&transform);
            let correct_center_of_mass = box_.center_of_mass();
            let computed_center_of_mass = compute_triangle_mesh_center_of_mass(box_.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-7 * correct_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_cone_center_of_mass(
            length in 1e-6..1e6,
            max_diameter in 1e-6..1e6,
            transform in similarity_transform_strategy(1e6, 1e-6..1e6),
        ) {
            let mut cone = UniformBodyMesh::create_cone(length, max_diameter, 30);
            cone.transform(&transform);
            let correct_center_of_mass = cone.center_of_mass();
            let computed_center_of_mass = compute_triangle_mesh_center_of_mass(cone.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-7 * correct_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn should_compute_hemisphere_center_of_mass(transform in similarity_transform_strategy(1e6, 1e-6..1e6)) {
            let mut hemisphere = UniformBodyMesh::create_hemisphere(15);
            hemisphere.transform(&transform);
            let correct_center_of_mass = hemisphere.center_of_mass();
            let computed_center_of_mass = compute_triangle_mesh_center_of_mass(hemisphere.triangle_mesh());
            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-3 * correct_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_determine_correct_properties_for_generic_mesh(
            length in 1e-6..1e6,
            max_diameter in 1e-6..1e6,
            transform in similarity_transform_strategy(1e6, 1e-6..1e6),
        ) {
            let mut cone_triangle_mesh = TriangleMesh::create_cone(length, max_diameter, 30);
            cone_triangle_mesh.transform(&transform);

            let mut cone = UniformBodyMesh::create_cone(length, max_diameter, 30);
            cone.transform(&transform);

            let cone_from_mesh = UniformBodyMesh::from_triangle_mesh(cone_triangle_mesh);

            prop_assert!(abs_diff_eq!(
                cone_from_mesh.volume(),
                cone.volume(),
                epsilon = 1e-2 * cone.volume()
            ));
            prop_assert!(abs_diff_eq!(
                cone_from_mesh.center_of_mass(),
                cone.center_of_mass(),
                epsilon = 1e-7 * cone.center_of_mass().coords.abs().max()
            ));
        }
    }
}
