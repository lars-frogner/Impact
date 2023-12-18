//! Computation and representation of inertia-related properties.

use crate::{
    geometry::TriangleMesh,
    num::Float,
    physics::{fph, Position},
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use nalgebra::{point, vector, Matrix3, Point3, Similarity3, UnitQuaternion, Vector3};
use simba::scalar::SubsetOf;

/// The inertia-related properties of a physical body.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InertialProperties {
    inertia_tensor: InertiaTensor,
    center_of_mass: Position,
    mass: fph,
}

/// The inertia tensor of a physical body.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InertiaTensor {
    matrix: Matrix3<fph>,
    inverse_matrix: Matrix3<fph>,
}

impl InertialProperties {
    /// Creates a new set of inertial properties.
    ///
    /// # Panics
    /// If the given mass does not exceed zero.
    pub fn new(mass: fph, center_of_mass: Position, inertia_tensor: InertiaTensor) -> Self {
        assert!(
            mass > 0.0,
            "Tried creating body with mass not exceeding zero"
        );
        Self {
            mass,
            center_of_mass,
            inertia_tensor,
        }
    }

    /// Computes the inertial properties of the uniformly dense body represented
    /// by the given triangle mesh, which is assumed closed, but may contain
    /// disjoint parts.
    pub fn of_uniform_triangle_mesh<F: Float + SubsetOf<fph>>(
        triangle_mesh: &TriangleMesh<F>,
        mass_density: fph,
    ) -> Self {
        let (mass, center_of_mass, inertia_tensor) =
            compute_uniform_triangle_mesh_inertial_properties(triangle_mesh, mass_density);
        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense box with the
    /// given extents, centered at the origin and with the width, height and
    /// depth axes aligned with the x-, y- and z-axis.
    ///
    /// The box corresponds to the one created by calling
    /// [`TriangleMesh::create_box`] with the same dimensions.
    pub fn of_uniform_box(extent_x: fph, extent_y: fph, extent_z: fph, mass_density: fph) -> Self {
        let mass = compute_box_volume(extent_x, extent_y, extent_z) * mass_density;

        let center_of_mass = Position::origin();

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            (0.25 * fph::ONE_THIRD) * mass * (extent_y.powi(2) + extent_z.powi(2)),
            (0.25 * fph::ONE_THIRD) * mass * (extent_x.powi(2) + extent_z.powi(2)),
            (0.25 * fph::ONE_THIRD) * mass * (extent_x.powi(2) + extent_y.powi(2)),
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense cylinder with
    /// the given length and diameter, centered at the origin and with the
    /// length axis aligned with the y-axis.
    ///
    /// The cylinder corresponds to the one created by calling
    /// [`TriangleMesh::create_cylinder`] with the same dimensions.
    pub fn of_uniform_cylinder(length: fph, diameter: fph, mass_density: fph) -> Self {
        let radius = diameter * 0.5;
        let mass = compute_cylinder_volume(radius, length) * mass_density;

        let center_of_mass = Position::origin();

        let moment_of_inertia_y = 0.5 * mass * radius.powi(2);
        let moment_of_inertia_xz =
            (0.25 * fph::ONE_THIRD) * mass * (3.0 * radius.powi(2) + length.powi(2));
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
    pub fn of_uniform_cone(length: fph, max_diameter: fph, mass_density: fph) -> Self {
        let max_radius = max_diameter * 0.5;
        let mass = compute_cone_volume(max_radius, length) * mass_density;

        // The center of mass is one quarter of the way up from the center of
        // the disk toward the point
        let center_of_mass = point![0.0, -length * 0.25, 0.0];

        let moment_of_inertia_y = (3.0 * 0.5 * 0.2) * mass * max_radius.powi(2);
        let moment_of_inertia_xz =
            0.5 * moment_of_inertia_y + (3.0 * 0.25 * 0.25 * 0.2) * mass * length.powi(2);
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
    pub fn of_uniform_sphere(mass_density: fph) -> Self {
        let radius = 0.5;
        let mass = compute_sphere_volume(radius) * mass_density;

        let center_of_mass = Position::origin();

        let moment_of_inertia = (2.0 * 0.2) * mass * radius.powi(2);
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
    pub fn of_uniform_hemisphere(mass_density: fph) -> Self {
        let radius = 0.5;
        let mass = compute_hemisphere_volume(radius) * mass_density;

        // The center of mass is (3/8) of the way up from the center of the disk
        // toward the top
        let center_of_mass = point![0.0, (3.0 / 8.0) * radius, 0.0];

        let moment_of_inertia = (2.0 * 0.2) * mass * radius.powi(2);

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia - mass * center_of_mass.y.powi(2),
            moment_of_inertia,
            moment_of_inertia - mass * center_of_mass.y.powi(2),
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> fph {
        self.mass
    }

    /// Returns the center of mass of the body (in the body's reference frame).
    pub fn center_of_mass(&self) -> &Position {
        &self.center_of_mass
    }

    /// Returns the inertia tensor of the body, defined with respect to the
    /// center of mass.
    pub fn inertia_tensor(&self) -> &InertiaTensor {
        &self.inertia_tensor
    }

    /// Applies the given similarity transform to the inertial properties of the
    /// body.
    pub fn transform(&mut self, transform: &Similarity3<fph>) {
        let mass_scaling = transform.scaling().powi(3);

        self.mass *= mass_scaling;

        self.center_of_mass = transform.transform_point(&self.center_of_mass);

        // Only the scaling and rotation affect the inertia tensor when it is
        // defined relative to the center of mass
        self.inertia_tensor = self
            .inertia_tensor
            .with_multiplied_mass(mass_scaling)
            .with_multiplied_extent(transform.scaling())
            .rotated(&transform.isometry.rotation);
    }

    /// Modifies the inertial properties according to a change in mass by the
    /// given factor.
    pub fn multiply_mass(&mut self, factor: fph) {
        self.mass *= factor;
        self.inertia_tensor = self.inertia_tensor.with_multiplied_mass(factor);
    }
}

impl AbsDiffEq for InertialProperties {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        fph::abs_diff_eq(&self.mass, &other.mass, epsilon)
            && Point3::abs_diff_eq(&self.center_of_mass, &other.center_of_mass, epsilon)
            && InertiaTensor::abs_diff_eq(&self.inertia_tensor, &other.inertia_tensor, epsilon)
    }
}

impl InertiaTensor {
    /// Creates a new inertia tensor corresponding to the given matrix.
    pub fn from_matrix(matrix: Matrix3<fph>) -> Self {
        let inverse_matrix = matrix
            .try_inverse()
            .expect("Could not invert inertia tensor");

        Self::from_matrix_and_inverse(matrix, inverse_matrix)
    }

    /// Creates a new diagonal inertia tensor with the given diagonal elements.
    ///
    /// # Panics
    /// If any of the given elements does not exceed zero.
    pub fn from_diagonal_elements(j_xx: fph, j_yy: fph, j_zz: fph) -> Self {
        assert!(
            j_xx > 0.0,
            "Tried creating inertia tensor with diagonal element not exceeding zero"
        );
        assert!(
            j_yy > 0.0,
            "Tried creating inertia tensor with diagonal element not exceeding zero"
        );
        assert!(
            j_zz > 0.0,
            "Tried creating inertia tensor with diagonal element not exceeding zero"
        );

        let matrix = Matrix3::from_diagonal(&vector![j_xx, j_yy, j_zz]);
        let inverse_matrix = Matrix3::from_diagonal(&vector![1.0 / j_xx, 1.0 / j_yy, 1.0 / j_zz]);

        Self::from_matrix_and_inverse(matrix, inverse_matrix)
    }

    /// Creates a new identity inertia tensor.
    pub fn identity() -> Self {
        Self::from_matrix_and_inverse(Matrix3::identity(), Matrix3::identity())
    }

    /// Returns a reference to the inertia matrix.
    pub fn matrix(&self) -> &Matrix3<fph> {
        &self.matrix
    }

    /// Returns a reference to the inverse of the inertia matrix.
    pub fn inverse_matrix(&self) -> &Matrix3<fph> {
        &self.inverse_matrix
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and returns it as a matrix.
    pub fn rotated_matrix(&self, rotation: &UnitQuaternion<fph>) -> Matrix3<fph> {
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix * self.matrix * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and scaling the body's extent by the given
    /// factor (keeping the mass unchanged) and returns it as a matrix.
    ///
    /// # Panics
    /// If the given factor is negative.
    pub fn rotated_matrix_with_scaled_extent(
        &self,
        rotation: &UnitQuaternion<fph>,
        factor: fph,
    ) -> Matrix3<fph> {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix * self.matrix.scale(factor.powi(2)) * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and returns its inverse as a matrix.
    pub fn inverse_rotated_matrix(&self, rotation: &UnitQuaternion<fph>) -> Matrix3<fph> {
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix * self.inverse_matrix * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and scaling the body's extent by the given
    /// factor (keeping the mass unchanged) and returns its inverse as a matrix.
    ///
    /// # Panics
    /// If the given factor is negative.
    pub fn inverse_rotated_matrix_with_scaled_extent(
        &self,
        rotation: &UnitQuaternion<fph>,
        factor: fph,
    ) -> Matrix3<fph> {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix
            * self.inverse_matrix.scale(1.0 / factor.powi(2))
            * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to scaling the mass of the
    /// body by the given factor.
    ///
    /// # Panics
    /// If the given factor is negative.
    pub fn with_multiplied_mass(&self, factor: fph) -> Self {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor mass with negative factor"
        );
        Self::from_matrix_and_inverse(
            self.matrix.scale(factor),
            self.inverse_matrix.scale(1.0 / factor),
        )
    }

    /// Computes the inertia tensor corresponding to scaling the extent of the
    /// body by the given factor, while keeping the mass unchanged.
    ///
    /// # Panics
    /// If the given factor is negative.
    pub fn with_multiplied_extent(&self, factor: fph) -> Self {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );

        // Moment of inertia scales as mass * distance^2
        let squared_factor = factor.powi(2);

        Self::from_matrix_and_inverse(
            self.matrix.scale(squared_factor),
            self.inverse_matrix.scale(1.0 / squared_factor),
        )
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<fph>) -> Self {
        let rotation_matrix = rotation.to_rotation_matrix();
        let transpose_rotation_matrix = rotation_matrix.transpose();

        let rotated_inertia_matrix = rotation_matrix * self.matrix * transpose_rotation_matrix;

        let rotated_inverse_inertia_matrix =
            rotation_matrix * self.inverse_matrix * transpose_rotation_matrix;

        Self::from_matrix_and_inverse(rotated_inertia_matrix, rotated_inverse_inertia_matrix)
    }

    /// Computes the difference matrix between the inertia tensor with respect
    /// to the center of mass and the inertia tensor with respect to a point at
    /// the given displacement from the center of mass. Adding this difference
    /// to the latter yields the center of mass inertia tensor.
    pub fn compute_parallel_axis_inertia_matrix_difference(
        mass: fph,
        displacement_from_com: &Vector3<fph>,
    ) -> Matrix3<fph> {
        let squared_displacement = displacement_from_com.component_mul(displacement_from_com);

        let shift_xx = -mass * (squared_displacement.y + squared_displacement.z);
        let shift_yy = -mass * (squared_displacement.z + squared_displacement.x);
        let shift_zz = -mass * (squared_displacement.x + squared_displacement.y);

        let shift_xy = mass * displacement_from_com.x * displacement_from_com.y;
        let shift_yz = mass * displacement_from_com.y * displacement_from_com.z;
        let shift_zx = mass * displacement_from_com.z * displacement_from_com.x;

        Matrix3::from_columns(&[
            vector![shift_xx, shift_xy, shift_zx],
            vector![shift_xy, shift_yy, shift_yz],
            vector![shift_zx, shift_yz, shift_zz],
        ])
    }

    fn from_matrix_and_inverse(matrix: Matrix3<fph>, inverse_matrix: Matrix3<fph>) -> Self {
        Self {
            matrix,
            inverse_matrix,
        }
    }

    #[cfg(test)]
    fn max_element(&self) -> fph {
        self.matrix.max()
    }
}

impl AbsDiffEq for InertiaTensor {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
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
    (F::FOUR * F::ONE_THIRD) * <F as Float>::PI * radius.powi(3)
}

pub fn compute_hemisphere_volume<F: Float>(radius: F) -> F {
    compute_sphere_volume(radius) * F::ONE_HALF
}

/// Computes the volume of the given triangle mesh, using the method described
/// in Eberly (2004). The mesh is assumed closed, but may contain disjoint
/// parts.
pub fn compute_triangle_mesh_volume<F: Float + SubsetOf<fph>>(mesh: &TriangleMesh<F>) -> fph {
    let mut volume = 0.0;

    for [vertex_0, vertex_1, vertex_2] in mesh.triangle_vertex_positions() {
        volume += compute_volume_contribution_for_triangle(
            &vertex_0.cast::<fph>(),
            &vertex_1.cast::<fph>(),
            &vertex_2.cast::<fph>(),
        );
    }

    volume *= 0.5 * fph::ONE_THIRD;

    volume
}

/// Computes the mass, center of mass and inertia tensor of a uniformly dense
/// body represented by the given triangle mesh, using the method described in
/// Eberly (2004). The inertia tensor is defined relative to the center of mass.
/// The mesh is assumed closed, but may contain disjoint parts.
pub fn compute_uniform_triangle_mesh_inertial_properties<F: Float + SubsetOf<fph>>(
    mesh: &TriangleMesh<F>,
    mass_density: fph,
) -> (fph, Position, InertiaTensor) {
    let mut mass = 0.0;
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
            &vertex_0.cast::<fph>(),
            &vertex_1.cast::<fph>(),
            &vertex_2.cast::<fph>(),
        );

        mass += zeroth_moment_contrib;
        first_moments += first_moment_contrib;
        diagonal_second_moments += diagonal_second_moment_contrib;
        mixed_second_moments += mixed_second_moment_contrib;
    }

    mass *= (0.5 * fph::ONE_THIRD) * mass_density;
    first_moments *= (0.5 * 0.25 * fph::ONE_THIRD) * mass_density;
    diagonal_second_moments *= (fph::ONE_THIRD * 0.25 * 0.2) * mass_density;
    mixed_second_moments *= (0.5 * fph::ONE_THIRD * 0.25 * 0.2) * mass_density;

    let center_of_mass = Point3::from(first_moments / mass);

    let j_xx = diagonal_second_moments.y + diagonal_second_moments.z;
    let j_yy = diagonal_second_moments.z + diagonal_second_moments.x;
    let j_zz = diagonal_second_moments.x + diagonal_second_moments.y;

    let j_xy = -mixed_second_moments.x;
    let j_yz = -mixed_second_moments.y;
    let j_zx = -mixed_second_moments.z;

    let inertia_tensor = InertiaTensor::from_matrix(
        Matrix3::from_columns(&[
            vector![j_xx, j_xy, j_zx],
            vector![j_xy, j_yy, j_yz],
            vector![j_zx, j_yz, j_zz],
        ]) + InertiaTensor::compute_parallel_axis_inertia_matrix_difference(
            mass,
            &(-center_of_mass.coords),
        ),
    );

    (mass, center_of_mass, inertia_tensor)
}

/// Computes the mass of the unform body represented by the given triangle mesh,
/// using the method described in Eberly (2004). The mesh is assumed closed, but
/// may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_mass<F: Float + SubsetOf<fph>>(
    mesh: &TriangleMesh<F>,
    mass_density: fph,
) -> fph {
    compute_triangle_mesh_volume(mesh) * mass_density
}

/// Computes the center of mass of the uniformly dense body represented by the
/// given triangle mesh, using the method described in Eberly (2004). The mesh
/// is assumed closed, but may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_center_of_mass<F: Float + SubsetOf<fph>>(
    mesh: &TriangleMesh<F>,
) -> Position {
    compute_uniform_triangle_mesh_inertial_properties(mesh, 1.0).1
}

/// Computes the inertia tensor of the uniformly dense body represented by the
/// given triangle mesh, using the method described in Eberly (2004). The
/// inertia tensor is defined relative to the center of mass. The mesh is
/// assumed closed, but may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_inertia_tensor<F: Float + SubsetOf<fph>>(
    mesh: &TriangleMesh<F>,
    mass_density: fph,
) -> InertiaTensor {
    compute_uniform_triangle_mesh_inertial_properties(mesh, mass_density).2
}

fn compute_volume_contribution_for_triangle(
    vertex_0: &Point3<fph>,
    vertex_1: &Point3<fph>,
    vertex_2: &Point3<fph>,
) -> fph {
    let edge_1_y = vertex_1.y - vertex_0.y;
    let edge_1_z = vertex_1.z - vertex_0.z;
    let edge_2_y = vertex_2.y - vertex_0.y;
    let edge_2_z = vertex_2.z - vertex_0.z;

    (edge_1_y * edge_2_z - edge_2_y * edge_1_z) * (vertex_0.x + vertex_1.x + vertex_2.x)
}

fn compute_zeroth_first_and_second_moment_contributions_for_triangle(
    vertex_0: &Point3<fph>,
    vertex_1: &Point3<fph>,
    vertex_2: &Point3<fph>,
) -> (fph, Vector3<fph>, Vector3<fph>, Vector3<fph>) {
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
    use approx::abs_diff_eq;
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};
    use proptest::prelude::*;
    use std::ops::Range;

    prop_compose! {
        fn rotation_strategy()(
            rotation_roll in 0.0..fph::TWO_PI,
            rotation_pitch in -fph::FRAC_PI_2..fph::FRAC_PI_2,
            rotation_yaw in 0.0..fph::TWO_PI,
        ) -> UnitQuaternion<fph> {
            UnitQuaternion::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw)
        }
    }

    prop_compose! {
        fn similarity_transform_strategy(
            max_translation: fph,
            scaling_range: Range<fph>
        )(
            translation_x in -max_translation..max_translation,
            translation_y in -max_translation..max_translation,
            translation_z in -max_translation..max_translation,
            rotation in rotation_strategy(),
            scaling in scaling_range,
        ) -> Similarity3<fph> {
            let translation = Translation3::new(translation_x, translation_y, translation_z);
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
            let mut cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_mass = cube_properties.mass();

            cube_properties.transform(&transform);

            let correctly_transformed_mass = initial_mass * transform.scaling().powi(3);

            prop_assert!(abs_diff_eq!(
                cube_properties.mass(),
                correctly_transformed_mass,
                epsilon = 1e-9 * correctly_transformed_mass
            ));
        }
    }

    proptest! {
        #[test]
        fn should_transform_uniform_cube_center_of_mass(transform in similarity_transform_strategy(1e4, 1e-4..1e4)) {
            let mut cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_center_of_mass = *cube_properties.center_of_mass();

            cube_properties.transform(&transform);

            let correctly_transformed_center_of_mass = transform.transform_point(&initial_center_of_mass);

            prop_assert!(abs_diff_eq!(
                cube_properties.center_of_mass(),
                &correctly_transformed_center_of_mass,
                epsilon = 1e-7 * correctly_transformed_center_of_mass.coords.abs().max()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_transform_uniform_cube_inertia_tensor(transform in similarity_transform_strategy(1e4, 1e-4..1e4)) {
            let mut cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_inertia_tensor = *cube_properties.inertia_tensor();

            cube_properties.transform(&transform);

            let correctly_transformed_inertia_tensor = initial_inertia_tensor
                .with_multiplied_mass(transform.scaling().powi(3))
                .with_multiplied_extent(transform.scaling())
                .rotated(&transform.isometry.rotation);

            prop_assert!(abs_diff_eq!(
                cube_properties.inertia_tensor(),
                &correctly_transformed_inertia_tensor,
                epsilon = 1e-7 * correctly_transformed_inertia_tensor.max_element()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_invert_rotated_inertia_tensor(rotation in rotation_strategy()) {
            let cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let rotated_inertia_tensor = cube_properties.inertia_tensor().rotated(&rotation);
            prop_assert!(abs_diff_eq!(
                rotated_inertia_tensor.inverse_matrix(),
                &rotated_inertia_tensor.matrix().try_inverse().unwrap(),
                epsilon = 1e-7
            ));
            prop_assert!(abs_diff_eq!(
                cube_properties.inertia_tensor().inverse_rotated_matrix(&rotation),
                rotated_inertia_tensor.matrix().try_inverse().unwrap(),
                epsilon = 1e-7
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

            let computed_mass = compute_uniform_triangle_mesh_mass(&sphere_mesh, mass_density);
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
        #![proptest_config(ProptestConfig::with_cases(15))]
        #[test]
        fn should_compute_uniform_hemisphere_mesh_center_of_mass(
            mass_density in 1e-4..1e4,
            transform in similarity_transform_strategy(1e4, 1e-4..1e4),
        ) {
            let mut hemisphere_mesh = TriangleMesh::create_hemisphere(20);
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

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            ));
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

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            ));
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

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            ));
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

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            ));
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

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor,
                epsilon = 1e-2 * correct_inertia_tensor.max_element()
            ));
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

            let cone_properties_from_mesh = InertialProperties::of_uniform_triangle_mesh(&cone_mesh, mass_density);

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
