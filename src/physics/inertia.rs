//! Computation and representation of inertia-related properties.

use crate::{geometry::TriangleMesh, num::Float};
use approx::AbsDiffEq;
use nalgebra::{point, vector, Matrix3, Point3, Similarity3, UnitQuaternion, Vector3};

/// The inertia-related properties of a physical body.
#[derive(Debug)]
pub struct InertialProperties<F: Float> {
    mass: F,
    center_of_mass: Point3<F>,
    inertia_tensor: InertiaTensor<F>,
}

/// The inertia tensor of a physical body.
#[derive(Clone, Debug, PartialEq)]
pub struct InertiaTensor<F: Float> {
    matrix: Matrix3<F>,
}

impl<F: Float> InertialProperties<F> {
    /// Creates a new set of inertial properties.
    pub fn new(mass: F, center_of_mass: Point3<F>, inertia_tensor: InertiaTensor<F>) -> Self {
        Self {
            mass,
            center_of_mass,
            inertia_tensor,
        }
    }

    /// Computes the inertial properties of the uniformly dense body represented
    /// by the given triangle mesh, which is assumed closed.
    pub fn of_uniform_triangle_mesh(triangle_mesh: TriangleMesh<F>, mass_density: F) -> Self {
        let (mass, center_of_mass, inertia_tensor) =
            compute_uniform_triangle_mesh_inertial_properties(&triangle_mesh, mass_density);
        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense box with the
    /// given extents, centered at the origin and with the width, height and
    /// depth axes aligned with the x-, y- and z-axis.
    ///
    /// The box corresponds to the one created by calling
    /// [`TriangleMesh::create_box`] with the same dimensions.
    pub fn of_uniform_box(extent_x: F, extent_y: F, extent_z: F, mass_density: F) -> Self {
        let mass = compute_box_volume(extent_x, extent_y, extent_z) * mass_density;

        let center_of_mass = Point3::origin();

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            (F::ONE_FOURTH * F::ONE_THIRD) * mass * (extent_y.powi(2) + extent_z.powi(2)),
            (F::ONE_FOURTH * F::ONE_THIRD) * mass * (extent_x.powi(2) + extent_z.powi(2)),
            (F::ONE_FOURTH * F::ONE_THIRD) * mass * (extent_x.powi(2) + extent_y.powi(2)),
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense cylinder with
    /// the given length and diameter, centered at the origin and with the
    /// length axis aligned with the y-axis.
    ///
    /// The cylinder corresponds to the one created by calling
    /// [`TriangleMesh::create_cylinder`] with the same dimensions.
    pub fn of_uniform_cylinder(length: F, diameter: F, mass_density: F) -> Self {
        let radius = diameter * F::ONE_HALF;
        let mass = compute_cylinder_volume(radius, length) * mass_density;

        let center_of_mass = Point3::origin();

        let moment_of_inertia_y = F::ONE_HALF * mass * radius.powi(2);
        let moment_of_inertia_xz =
            (F::ONE_FOURTH * F::ONE_THIRD) * mass * (F::THREE * radius.powi(2) + length.powi(2));
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia_xz,
            moment_of_inertia_y,
            moment_of_inertia_xz,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense cone with the
    /// given length and maximum diameter, centered at the origin and pointing
    /// along the positive y-direction.
    ///
    /// The cone corresponds to the one created by calling
    /// [`TriangleMesh::create_cone`] with the same dimensions.
    pub fn of_uniform_cone(length: F, max_diameter: F, mass_density: F) -> Self {
        let max_radius = max_diameter * F::ONE_HALF;
        let mass = compute_cone_volume(max_radius, length) * mass_density;

        // The center of mass is one quarter of the way up from the center of
        // the disk toward the point
        let center_of_mass = point![F::ZERO, -length * F::ONE_FOURTH, F::ZERO];

        let moment_of_inertia_y =
            (F::THREE * F::ONE_HALF * F::ONE_FIFTH) * mass * max_radius.powi(2);
        let moment_of_inertia_xz = F::ONE_HALF * moment_of_inertia_y
            + (F::THREE * F::ONE_FOURTH * F::ONE_FOURTH * F::ONE_FIFTH) * mass * length.powi(2);
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia_xz,
            moment_of_inertia_y,
            moment_of_inertia_xz,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense sphere with
    /// diameter 1.0, centered at the origin.
    ///
    /// The sphere corresponds to the one created by calling
    /// [`TriangleMesh::create_sphere`].
    pub fn of_uniform_sphere(mass_density: F) -> Self {
        let radius = F::ONE_HALF;
        let mass = compute_sphere_volume(radius) * mass_density;

        let center_of_mass = Point3::origin();

        let moment_of_inertia = (F::TWO * F::ONE_FIFTH) * mass * radius.powi(2);
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia,
            moment_of_inertia,
            moment_of_inertia,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniform hemisphere with diameter
    /// 1.0, with the disk lying in the xz-plane and centered at the origin.
    ///
    /// The hemisphere corresponds to the one created by calling
    /// [`TriangleMesh::create_hemisphere`].
    pub fn of_uniform_hemisphere(mass_density: F) -> Self {
        let radius = F::ONE_HALF;
        let mass = compute_hemisphere_volume(radius) * mass_density;

        // The center of mass is (3/8) of the way up from the center of the disk
        // toward the top
        let center_of_mass = point![F::ZERO, (F::THREE / F::EIGHT) * radius, F::ZERO];

        let moment_of_inertia = (F::TWO * F::ONE_FIFTH) * mass * radius.powi(2);
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia,
            moment_of_inertia,
            moment_of_inertia,
        )
        .with_displaced_axis(mass, &center_of_mass.coords);

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> F {
        self.mass
    }

    /// Returns the center of mass of the body (in model space).
    pub fn center_of_mass(&self) -> &Point3<F> {
        &self.center_of_mass
    }

    /// Returns the inertia tensor of the body, defined with respect to the
    /// center of mass.
    pub fn inertia_tensor(&self) -> &InertiaTensor<F> {
        &self.inertia_tensor
    }

    /// Applies the given similarity transform to the inertial properties of the
    /// body.
    pub fn transform(&mut self, transform: &Similarity3<F>) {
        self.mass *= transform.scaling().powi(3);

        self.center_of_mass = transform.transform_point(&self.center_of_mass);

        // Only the scaling and rotation affect the inertia tensor when it is
        // defined relative to the center of mass
        self.inertia_tensor = self
            .inertia_tensor
            .scaled(transform.scaling())
            .rotated(&transform.isometry.rotation);
    }
}

impl<F: Float> InertiaTensor<F> {
    fn from_matrix(inertia_tensor: Matrix3<F>) -> Self {
        Self {
            matrix: inertia_tensor,
        }
    }

    fn from_diagonal_elements(j_xx: F, j_yy: F, j_zz: F) -> Self {
        Self::from_matrix(Matrix3::from_diagonal(&vector![j_xx, j_yy, j_zz]))
    }

    /// Computes the inertia tensor corresponding to scaling the body uniformly
    /// by the given factor.
    pub fn scaled(&self, scaling: F) -> Self {
        assert!(scaling >= F::ZERO);
        // Moment of inertia scales as mass * distance^2 = distance^5
        Self::from_matrix(self.matrix.scale(scaling.powi(5)))
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        let rotation_matrix = rotation.to_rotation_matrix();
        let rotated_inertia_matrix = rotation_matrix * self.matrix * rotation_matrix.transpose();
        Self::from_matrix(rotated_inertia_matrix)
    }

    /// Computes the inertia tensor with respect to the point at the given
    /// displacement from the current point.
    pub fn with_displaced_axis(&self, mass: F, displacement: &Vector3<F>) -> Self {
        let squared_displacement = displacement.component_mul(displacement);

        let shift_xx = -mass * (squared_displacement.y + squared_displacement.z);
        let shift_yy = -mass * (squared_displacement.z + squared_displacement.x);
        let shift_zz = -mass * (squared_displacement.x + squared_displacement.y);

        let shift_xy = mass * displacement.x * displacement.y;
        let shift_yz = mass * displacement.y * displacement.z;
        let shift_zx = mass * displacement.z * displacement.x;

        Self::from_matrix(
            self.matrix
                + Matrix3::from_columns(&[
                    vector![shift_xx, shift_xy, shift_zx],
                    vector![shift_xy, shift_yy, shift_yz],
                    vector![shift_zx, shift_yz, shift_zz],
                ]),
        )
    }

    #[cfg(test)]
    fn max_element(&self) -> F {
        self.matrix.max()
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
    <F as Float>::PI * radius.powi(2) * length
}

pub fn compute_cone_volume<F: Float>(max_radius: F, length: F) -> F {
    compute_cylinder_volume(max_radius, length) * F::ONE_THIRD
}

pub fn compute_sphere_volume<F: Float>(radius: F) -> F {
    (F::FOUR / F::THREE) * <F as Float>::PI * radius.powi(3)
}

pub fn compute_hemisphere_volume<F: Float>(radius: F) -> F {
    compute_sphere_volume(radius) * F::ONE_HALF
}

/// Computes the mass of the unform body represented by the given triangle mesh,
/// using the method described in Eberly (2004). The mesh is assumed closed.
pub fn compute_uniform_triangle_mesh_mass<F: Float>(mesh: &TriangleMesh<F>, mass_density: F) -> F {
    compute_triangle_mesh_volume(mesh) * mass_density
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

/// Computes the volume and center of mass of the uniformly dense body
/// represented by the given triangle mesh, using the method described in Eberly
/// (2004). The mesh is assumed closed.
pub fn compute_uniform_triangle_mesh_volume_and_center_of_mass<F: Float>(
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

/// Computes the center of mass of the uniformly dense body represented by the
/// given triangle mesh, using the method described in Eberly (2004). The mesh
/// is assumed closed.
pub fn compute_uniform_triangle_mesh_center_of_mass<F: Float>(mesh: &TriangleMesh<F>) -> Point3<F> {
    compute_uniform_triangle_mesh_volume_and_center_of_mass(mesh).1
}

/// Computes the inertia tensor of the uniformly dense body represented by the
/// given triangle mesh, using the method described in Eberly (2004). The
/// inertia tensor is defined relative to the center of mass. The mesh is
/// assumed closed.
pub fn compute_uniform_triangle_mesh_inertia_tensor<F: Float>(
    mesh: &TriangleMesh<F>,
    mass_density: F,
) -> InertiaTensor<F> {
    compute_uniform_triangle_mesh_inertial_properties(mesh, mass_density).2
}

/// Computes the mass, center of mass and inertia tensor of a uniformly dense
/// body represented by the given triangle mesh, using the method described in
/// Eberly (2004). The inertia tensor is defined relative to the center of mass.
/// The mesh is assumed closed.
pub fn compute_uniform_triangle_mesh_inertial_properties<F: Float>(
    mesh: &TriangleMesh<F>,
    mass_density: F,
) -> (F, Point3<F>, InertiaTensor<F>) {
    let mut mass = F::ZERO;
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

        mass += zeroth_moment_contrib;
        first_moments += first_moment_contrib;
        diagonal_second_moments += diagonal_second_moment_contrib;
        mixed_second_moments += mixed_second_moment_contrib;
    }

    mass *= F::ONE_HALF * F::ONE_THIRD * mass_density;
    first_moments *= F::ONE_HALF * F::ONE_FOURTH * F::ONE_THIRD * mass_density;
    diagonal_second_moments *= F::ONE_THIRD * F::ONE_FOURTH * F::ONE_FIFTH * mass_density;
    mixed_second_moments *=
        F::ONE_HALF * F::ONE_THIRD * F::ONE_FOURTH * F::ONE_FIFTH * mass_density;

    let center_of_mass = Point3::from(first_moments / mass);

    let j_xx = diagonal_second_moments.y + diagonal_second_moments.z;
    let j_yy = diagonal_second_moments.z + diagonal_second_moments.x;
    let j_zz = diagonal_second_moments.x + diagonal_second_moments.y;

    let j_xy = -mixed_second_moments.x;
    let j_yz = -mixed_second_moments.y;
    let j_zx = -mixed_second_moments.z;

    let inertia_tensor = InertiaTensor::from_matrix(Matrix3::from_columns(&[
        vector![j_xx, j_xy, j_zx],
        vector![j_xy, j_yy, j_yz],
        vector![j_zx, j_yz, j_zz],
    ]))
    .with_displaced_axis(mass, &center_of_mass.coords);

    (mass, center_of_mass, inertia_tensor)
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
    let f_3 = w_0.component_mul(&tmp_1) + w_1.component_mul(&tmp_2) + w_2.component_mul(&f_2);

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
    use crate::geometry::FrontFaceSide;
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
        fn should_transform_uniform_cube_mass(transform in similarity_transform_strategy(1e4, 1e-4..1e4)) {
            let mut cube = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_mass = cube.mass();
            cube.transform(&transform);
            let mass_after_transforming = cube.mass();
            let correctly_transformed_mass = initial_mass * f64::powi(transform.scaling(), 3);
            prop_assert!(abs_diff_eq!(
                mass_after_transforming,
                correctly_transformed_mass,
                epsilon = 1e-9 * correctly_transformed_mass
            ));
        }
    }

    proptest! {
        #[test]
        fn should_transform_uniform_cube_center_of_mass(transform in similarity_transform_strategy(1e4, 1e-4..1e4)) {
            let mut cube = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
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
        fn should_transform_uniform_cube_inertia_tensor(transform in similarity_transform_strategy(1e4, 1e-4..1e4)) {
            let mut cube = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_inertia_tensor = cube.inertia_tensor().clone();
            cube.transform(&transform);
            let inertia_tensor_after_transforming = cube.inertia_tensor().clone();
            let correctly_transformed_inertia_tensor = initial_inertia_tensor
                .scaled(transform.scaling())
                .rotated(&transform.isometry.rotation);
            prop_assert!(abs_diff_eq!(
                inertia_tensor_after_transforming,
                &correctly_transformed_inertia_tensor,
                epsilon = 1e-7 * correctly_transformed_inertia_tensor.max_element()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_box_mesh_mass(
            extent_x in 1e-4..1e4,
            extent_y in 1e-4..1e4,
            extent_z in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform);
            box_properties.transform(&transform);

            let computed_mass = compute_uniform_triangle_mesh_mass(&box_mesh, mass_density);
            let correct_mass = box_properties.mass();

            prop_assert!(abs_diff_eq!(
                computed_mass,
                correct_mass,
                epsilon = 1e-9 * correct_mass
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn should_compute_uniform_cylinder_mesh_mass(
            length in 1e-4..1e4,
            diameter in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut cylinder_mesh = TriangleMesh::create_cylinder(length, diameter, 30);
            let mut cylinder_properties = InertialProperties::of_uniform_cylinder(length, diameter, mass_density);

            cylinder_mesh.transform(&transform);
            cylinder_properties.transform(&transform);

            let computed_mass = compute_uniform_triangle_mesh_mass(&cylinder_mesh, mass_density);
            let correct_mass = cylinder_properties.mass();

            prop_assert!(abs_diff_eq!(
                computed_mass,
                correct_mass,
                epsilon = 1e-2 * correct_mass
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]
        #[test]
        fn should_compute_uniform_sphere_mesh_mass(
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut sphere_mesh = TriangleMesh::create_sphere(20);
            let mut sphere_properties = InertialProperties::of_uniform_sphere(mass_density);

            sphere_mesh.transform(&transform);
            sphere_properties.transform(&transform);

            let computed_mass: f64 = compute_uniform_triangle_mesh_mass(&sphere_mesh, mass_density);
            let correct_mass = sphere_properties.mass();

            prop_assert!(abs_diff_eq!(
                computed_mass,
                correct_mass,
                epsilon = 1e-2 * correct_mass
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_box_mesh_center_of_mass(
            extent_x in 1e-4..1e4,
            extent_y in 1e-4..1e4,
            extent_z in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform);
            box_properties.transform(&transform);

            let computed_center_of_mass = compute_uniform_triangle_mesh_center_of_mass(&box_mesh);
            let correct_center_of_mass = box_properties.center_of_mass();

            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-7 * correct_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_cone_mesh_center_of_mass(
            length in 1e-4..1e4,
            max_diameter in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut cone_mesh = TriangleMesh::create_cone(length, max_diameter, 30);
            let mut cone_properties = InertialProperties::of_uniform_cone(length, max_diameter, mass_density);

            cone_mesh.transform(&transform);
            cone_properties.transform(&transform);

            let computed_center_of_mass = compute_uniform_triangle_mesh_center_of_mass(&cone_mesh);
            let correct_center_of_mass = cone_properties.center_of_mass();

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
        fn should_compute_uniform_hemisphere_mesh_center_of_mass(
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut hemisphere_mesh = TriangleMesh::create_hemisphere(15);
            let mut hemisphere_properties = InertialProperties::of_uniform_hemisphere(mass_density);

            hemisphere_mesh.transform(&transform);
            hemisphere_properties.transform(&transform);

            let computed_center_of_mass = compute_uniform_triangle_mesh_center_of_mass(&hemisphere_mesh);
            let correct_center_of_mass = hemisphere_properties.center_of_mass();

            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-3 * correct_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn should_compute_uniform_sphere_mesh_inertia_tensor(
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut sphere_mesh = TriangleMesh::create_sphere(30);
            let mut sphere_properties = InertialProperties::of_uniform_sphere(mass_density);

            sphere_mesh.transform(&transform);
            sphere_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(&sphere_mesh, mass_density);
            let correct_inertia_tensor = sphere_properties.inertia_tensor();

            assert_abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn should_compute_uniform_hemisphere_mesh_inertia_tensor(
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut hemisphere_mesh = TriangleMesh::create_hemisphere(15);
            let mut hemisphere_properties = InertialProperties::of_uniform_hemisphere(mass_density);

            hemisphere_mesh.transform(&transform);
            hemisphere_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(&hemisphere_mesh, mass_density);
            let correct_inertia_tensor = hemisphere_properties.inertia_tensor();

            assert_abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn should_compute_uniform_cone_mesh_inertia_tensor(
            length in 1e-4..1e4,
            max_diameter in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut cone_mesh = TriangleMesh::create_cone(length, max_diameter, 40);
            let mut cone_properties = InertialProperties::of_uniform_cone(length, max_diameter, mass_density);

            cone_mesh.transform(&transform);
            cone_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(&cone_mesh, mass_density);
            let correct_inertia_tensor = cone_properties.inertia_tensor();

            assert_abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn should_compute_uniform_cylinder_mesh_inertia_tensor(
            length in 1e-4..1e4,
            diameter in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut cylinder_mesh = TriangleMesh::create_cylinder(length, diameter, 40);
            let mut cylinder_properties = InertialProperties::of_uniform_cylinder(length, diameter, mass_density);

            cylinder_mesh.transform(&transform);
            cylinder_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(&cylinder_mesh, mass_density);
            let correct_inertia_tensor = cylinder_properties.inertia_tensor();

            assert_abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            );
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_box_mesh_inertia_tensor(
            extent_x in 1e-4..1e4,
            extent_y in 1e-4..1e4,
            extent_z in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform);
            box_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(&box_mesh, mass_density);
            let correct_inertia_tensor = box_properties.inertia_tensor();

            assert_abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            );
        }
    }

    proptest! {
        #[test]
        fn should_determine_correct_properties_for_generic_mesh(
            length in 1e-4..1e4,
            max_diameter in 1e-4..1e4,
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut cone_mesh = TriangleMesh::create_cone(length, max_diameter, 40);
            let mut cone_properties = InertialProperties::of_uniform_cone(length, max_diameter, mass_density);

            cone_mesh.transform(&transform);
            cone_properties.transform(&transform);

            let cone_properties_from_mesh = InertialProperties::of_uniform_triangle_mesh(cone_mesh, mass_density);

            prop_assert!(abs_diff_eq!(
                cone_properties_from_mesh.mass(),
                cone_properties.mass(),
                epsilon = 1e-2 * cone_properties.mass()
            ));
            prop_assert!(abs_diff_eq!(
                cone_properties_from_mesh.center_of_mass(),
                cone_properties.center_of_mass(),
                epsilon = 1e-7 * cone_properties.center_of_mass().coords.abs().max()
            ));
            prop_assert!(abs_diff_eq!(
                cone_properties_from_mesh.inertia_tensor(),
                cone_properties.inertia_tensor(),
                epsilon = 1e-2 * cone_properties.inertia_tensor().max_element()
            ));
        }
    }
}
