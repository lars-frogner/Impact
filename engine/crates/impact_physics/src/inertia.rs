//! Computation and representation of inertia-related properties.

use crate::quantities::Position;
use approx::{AbsDiffEq, RelativeEq};
use bytemuck::{Pod, Zeroable};
use impact_math::{
    consts::f32::PI,
    matrix::{Matrix3, Matrix3C},
    point::{Point3, Point3C},
    quaternion::UnitQuaternion,
    transform::Similarity3,
    vector::{UnitVector3, Vector3, Vector3C},
};
use roc_integration::roc;

/// The inertia-related properties of a physical body.
#[derive(Clone, Debug, PartialEq)]
pub struct InertialProperties {
    inertia_tensor: InertiaTensor,
    center_of_mass: Position,
    mass: f32,
}

/// The inertia tensor of a physical body.
///
/// The columns of the matrix and its inverse are stored in 128-bit SIMD
/// registers for efficient computation. That leads to an extra 24 bytes in size
/// (4 for each column) and 16-byte alignment. For cache-friendly storage,
/// prefer the compact 4-byte aligned [`InertiaTensorP`].
#[derive(Clone, Debug, PartialEq)]
pub struct InertiaTensor {
    matrix: Matrix3,
    inverse_matrix: Matrix3,
}

/// The inertia tensor of a physical body. This the "compact" version.
///
/// This type is primarily intended for compact storage inside other types and
/// collections. For computations, prefer the SIMD-friendly 16-byte aligned
/// [`InertiaTensor`].
#[roc(name = "InertiaTensor", parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InertiaTensorP {
    matrix: Matrix3C,
    inverse_matrix: Matrix3C,
}

impl InertialProperties {
    /// Creates a new set of inertial properties.
    ///
    /// # Panics
    /// If the given mass does not exceed zero.
    pub fn new(mass: f32, center_of_mass: Position, inertia_tensor: InertiaTensor) -> Self {
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

    /// Computes the inertial properties of the uniformly dense body whose
    /// surface is represented by the given triangles. The surface is assumed
    /// closed, but may contain disjoint parts.
    pub fn of_uniform_triangle_mesh<'a>(
        triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
        mass_density: f32,
    ) -> Self {
        let (mass, center_of_mass, inertia_tensor) =
            compute_uniform_triangle_mesh_inertial_properties(
                triangle_vertex_positions,
                mass_density,
            );
        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense box with the
    /// given extents, with the width, height and depth axes aligned with the
    /// x-, y- and z-axis.
    ///
    /// The box corresponds to the one created by calling
    /// `impact_mesh::TriangleMesh::create_box` with the same dimensions.
    pub fn of_uniform_box(extent_x: f32, extent_y: f32, extent_z: f32, mass_density: f32) -> Self {
        let mass = compute_box_volume(extent_x, extent_y, extent_z) * mass_density;

        let center_of_mass = Position::origin();

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            (1.0 / 12.0) * mass * (extent_y.powi(2) + extent_z.powi(2)),
            (1.0 / 12.0) * mass * (extent_x.powi(2) + extent_z.powi(2)),
            (1.0 / 12.0) * mass * (extent_x.powi(2) + extent_y.powi(2)),
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense cylinder with
    /// the given length and diameter, with the length axis aligned with the
    /// y-axis.
    ///
    /// The cylinder corresponds to the one created by calling
    /// `impact_mesh::TriangleMesh::create_cylinder` with the same dimensions.
    pub fn of_uniform_cylinder(length: f32, diameter: f32, mass_density: f32) -> Self {
        let radius = diameter * 0.5;
        let mass = compute_cylinder_volume(radius, length) * mass_density;

        let center_of_mass = Point3::new(0.0, length * 0.5, 0.0);

        let moment_of_inertia_y = 0.5 * mass * radius.powi(2);
        let moment_of_inertia_xz = (1.0 / 12.0) * mass * (3.0 * radius.powi(2) + length.powi(2));
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia_xz,
            moment_of_inertia_y,
            moment_of_inertia_xz,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of the uniformly dense cone with the
    /// given length and maximum diameter, pointing along the positive
    /// y-direction.
    ///
    /// The cone corresponds to the one created by calling
    /// `impact_mesh::TriangleMesh::create_cone` with the same dimensions.
    pub fn of_uniform_cone(length: f32, max_diameter: f32, mass_density: f32) -> Self {
        let max_radius = max_diameter * 0.5;
        let mass = compute_cone_volume(max_radius, length) * mass_density;

        // The center of mass is one quarter of the way up from the center of
        // the disk toward the point
        let center_of_mass = Point3::new(0.0, length * 0.25, 0.0);

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
    /// the given diameter.
    ///
    /// With `radius = 1.0`, the sphere corresponds to the one created by
    /// calling `impact_mesh::TriangleMesh::create_sphere`.
    pub fn of_uniform_sphere(radius: f32, mass_density: f32) -> Self {
        let mass = compute_sphere_volume(radius) * mass_density;

        let center_of_mass = Position::origin();

        let moment_of_inertia = (2.0 / 5.0) * mass * radius.powi(2);
        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia,
            moment_of_inertia,
            moment_of_inertia,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of a uniform hemisphere with the
    /// given radius, with the disk lying in the xz-plane.
    ///
    /// With `radius = 0.5`, the hemisphere corresponds to the one created by
    /// calling `impact_mesh::TriangleMesh::create_hemisphere`.
    pub fn of_uniform_hemisphere(radius: f32, mass_density: f32) -> Self {
        let mass = compute_hemisphere_volume(radius) * mass_density;

        // The center of mass is (3/8) of the way up from the center of the disk
        // toward the top
        let center_of_mass = Point3::new(0.0, (3.0 / 8.0) * radius, 0.0);

        let moment_of_inertia_y = (2.0 / 5.0) * mass * radius.powi(2);
        let moment_of_inertia_xz = (83.0 / 320.0) * mass * radius.powi(2);

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia_xz,
            moment_of_inertia_y,
            moment_of_inertia_xz,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Computes the inertial properties of a uniform vertical capsule with the
    /// given segment length and radius.
    pub fn of_uniform_capsule(segment_length: f32, radius: f32, mass_density: f32) -> Self {
        let cylinder_mass = compute_cylinder_volume(radius, segment_length) * mass_density;
        let hemisphere_mass = compute_hemisphere_volume(radius) * mass_density;

        let cylinder_moment_of_inertia_y = 0.5 * cylinder_mass * radius.powi(2);
        let cylinder_moment_of_inertia_xz =
            (1.0 / 12.0) * cylinder_mass * (3.0 * radius.powi(2) + segment_length.powi(2));

        let hemisphere_moment_of_inertia_y = (2.0 / 5.0) * hemisphere_mass * radius.powi(2);
        let hemisphere_moment_of_inertia_xz_about_com =
            (83.0 / 320.0) * hemisphere_mass * radius.powi(2);

        let hemisphere_center_of_mass_y = 0.5 * segment_length + (3.0 / 8.0) * radius;
        let hemisphere_moment_of_inertia_xz = hemisphere_moment_of_inertia_xz_about_com
            + hemisphere_mass * hemisphere_center_of_mass_y.powi(2);

        let moment_of_inertia_y =
            cylinder_moment_of_inertia_y + 2.0 * hemisphere_moment_of_inertia_y;
        let moment_of_inertia_xz =
            cylinder_moment_of_inertia_xz + 2.0 * hemisphere_moment_of_inertia_xz;

        let mass = cylinder_mass + 2.0 * hemisphere_mass;

        let center_of_mass = Point3::origin();

        let inertia_tensor = InertiaTensor::from_diagonal_elements(
            moment_of_inertia_xz,
            moment_of_inertia_y,
            moment_of_inertia_xz,
        );

        Self::new(mass, center_of_mass, inertia_tensor)
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> f32 {
        self.mass
    }

    /// Returns the center of mass of the body (in the body's model space).
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
    pub fn transform(&mut self, transform: &Similarity3) {
        let mass_scaling = transform.scaling().powi(3);

        self.mass *= mass_scaling;

        self.center_of_mass = transform.transform_point(&self.center_of_mass);

        // Only the scaling and rotation affect the inertia tensor when it is
        // defined relative to the center of mass
        self.inertia_tensor = self
            .inertia_tensor
            .with_multiplied_mass(mass_scaling)
            .with_multiplied_extent(transform.scaling())
            .rotated(transform.rotation());
    }

    /// Applies the given distance scaling factor to the inertial properties of
    /// the body.
    pub fn scale(&mut self, scale: f32) {
        let mass_scaling = scale.powi(3);

        self.mass *= mass_scaling;

        self.center_of_mass = scale * self.center_of_mass;

        self.inertia_tensor = self
            .inertia_tensor
            .with_multiplied_mass(mass_scaling)
            .with_multiplied_extent(scale);
    }

    /// Modifies the inertial properties according to a change in mass by the
    /// given factor.
    pub fn multiply_mass(&mut self, factor: f32) {
        self.mass *= factor;
        self.inertia_tensor = self.inertia_tensor.with_multiplied_mass(factor);
    }
}

impl AbsDiffEq for InertialProperties {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        f32::abs_diff_eq(&self.mass, &other.mass, epsilon)
            && Point3::abs_diff_eq(&self.center_of_mass, &other.center_of_mass, epsilon)
            && InertiaTensor::abs_diff_eq(&self.inertia_tensor, &other.inertia_tensor, epsilon)
    }
}

impl RelativeEq for InertialProperties {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        f32::relative_eq(&self.mass, &other.mass, epsilon, max_relative)
            && Point3::relative_eq(
                &self.center_of_mass,
                &other.center_of_mass,
                epsilon,
                max_relative,
            )
            && InertiaTensor::relative_eq(
                &self.inertia_tensor,
                &other.inertia_tensor,
                epsilon,
                max_relative,
            )
    }
}

impl InertiaTensor {
    /// Creates a new identity inertia tensor.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_matrix_and_inverse(Matrix3::identity(), Matrix3::identity())
    }

    /// Creates a new inertia tensor corresponding to the given matrix.
    #[inline]
    pub fn from_matrix(matrix: Matrix3) -> Self {
        let inverse_matrix = matrix.inverse();

        Self::from_matrix_and_inverse(matrix, inverse_matrix)
    }

    /// Creates an inertia tensor corresponding to the given matrix and its
    /// inverse.
    #[inline]
    pub const fn from_matrix_and_inverse(matrix: Matrix3, inverse_matrix: Matrix3) -> Self {
        Self {
            matrix,
            inverse_matrix,
        }
    }

    /// Creates a new diagonal inertia tensor with the given diagonal elements.
    ///
    /// # Panics
    /// If any of the given elements does not exceed zero.
    #[inline]
    pub fn from_diagonal_elements(j_xx: f32, j_yy: f32, j_zz: f32) -> Self {
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

        let matrix = Matrix3::from_diagonal(&Vector3::new(j_xx, j_yy, j_zz));
        let inverse_matrix =
            Matrix3::from_diagonal(&Vector3::new(1.0 / j_xx, 1.0 / j_yy, 1.0 / j_zz));

        Self::from_matrix_and_inverse(matrix, inverse_matrix)
    }

    /// Returns a reference to the inertia matrix.
    #[inline]
    pub const fn matrix(&self) -> &Matrix3 {
        &self.matrix
    }

    /// Returns a reference to the inverse of the inertia matrix.
    #[inline]
    pub const fn inverse_matrix(&self) -> &Matrix3 {
        &self.inverse_matrix
    }

    /// Computes the moment of inertia about the given axis passing through the
    /// same point this inertia tensor is defined with respect to.
    #[inline]
    pub fn moment_about_axis(&self, axis: &UnitVector3) -> f32 {
        axis.dot(&(self.matrix * axis.as_vector()))
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and returns it as a matrix.
    #[inline]
    pub fn rotated_matrix(&self, rotation: &UnitQuaternion) -> Matrix3 {
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix * self.matrix * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and scaling the body's extent by the given
    /// factor (keeping the mass unchanged) and returns it as a matrix.
    ///
    /// # Panics
    /// If the given factor is negative.
    #[inline]
    pub fn rotated_matrix_with_scaled_extent(
        &self,
        rotation: &UnitQuaternion,
        factor: f32,
    ) -> Matrix3 {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );
        let rotation_matrix = rotation.to_rotation_matrix();
        factor.powi(2) * (rotation_matrix * self.matrix * rotation_matrix.transpose())
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and returns its inverse as a matrix.
    #[inline]
    pub fn inverse_rotated_matrix(&self, rotation: &UnitQuaternion) -> Matrix3 {
        let rotation_matrix = rotation.to_rotation_matrix();
        rotation_matrix * self.inverse_matrix * rotation_matrix.transpose()
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion and scaling the body's extent by the given
    /// factor (keeping the mass unchanged) and returns its inverse as a matrix.
    ///
    /// # Panics
    /// If the given factor is negative.
    #[inline]
    pub fn inverse_rotated_matrix_with_scaled_extent(
        &self,
        rotation: &UnitQuaternion,
        factor: f32,
    ) -> Matrix3 {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );
        let rotation_matrix = rotation.to_rotation_matrix();
        (1.0 / factor.powi(2))
            * (rotation_matrix * self.inverse_matrix * rotation_matrix.transpose())
    }

    /// Computes the inertia tensor corresponding to scaling the mass of the
    /// body by the given factor.
    ///
    /// # Panics
    /// If the given factor is negative.
    #[inline]
    pub fn with_multiplied_mass(&self, factor: f32) -> Self {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor mass with negative factor"
        );
        Self::from_matrix_and_inverse(factor * self.matrix, (1.0 / factor) * self.inverse_matrix)
    }

    /// Computes the inertia tensor corresponding to scaling the extent of the
    /// body by the given factor, while keeping the mass unchanged.
    ///
    /// # Panics
    /// If the given factor is negative.
    #[inline]
    pub fn with_multiplied_extent(&self, factor: f32) -> Self {
        assert!(
            factor >= 0.0,
            "Tried multiplying inertia tensor extent with negative factor"
        );

        // Moment of inertia scales as mass * distance^2
        let squared_factor = factor.powi(2);

        Self::from_matrix_and_inverse(
            squared_factor * self.matrix,
            (1.0 / squared_factor) * self.inverse_matrix,
        )
    }

    /// Computes the inertia tensor corresponding to rotating the body with the
    /// given rotation quaternion.
    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternion) -> Self {
        let rotation_matrix = rotation.to_rotation_matrix();
        let transpose_rotation_matrix = rotation_matrix.transpose();

        let rotated_inertia_matrix = rotation_matrix * self.matrix * transpose_rotation_matrix;

        let rotated_inverse_inertia_matrix =
            rotation_matrix * self.inverse_matrix * transpose_rotation_matrix;

        Self::from_matrix_and_inverse(rotated_inertia_matrix, rotated_inverse_inertia_matrix)
    }

    /// Uses the parallel axis theorem to compute the difference matrix that
    /// must be added to the inertia tensor for it to be defined with respect to
    /// when the center of mass when the center of mass has the given
    /// displacement from the point the current inertia tensor is defined with
    /// respect to.
    #[inline]
    pub fn compute_delta_to_com_inertia_matrix(
        mass: f32,
        displacement_to_com: &Vector3,
    ) -> Matrix3 {
        let (moment_of_inertia_deltas, product_of_inertia_deltas) =
            Self::compute_delta_to_com_moments_and_products_of_inertia(mass, displacement_to_com);

        let [shift_xx, shift_yy, shift_zz] = moment_of_inertia_deltas.into();
        let [shift_xy, shift_yz, shift_zx] = (-product_of_inertia_deltas).into();

        Matrix3::from_columns(
            Vector3::new(shift_xx, shift_xy, shift_zx),
            Vector3::new(shift_xy, shift_yy, shift_yz),
            Vector3::new(shift_zx, shift_yz, shift_zz),
        )
    }

    /// Uses the parallel axis theorem to compute the differences that must be
    /// added to the moments and products of inertia for them to be defined
    /// with respect to the center of mass when the center of mass has the
    /// given displacement from the point they are currently defined with
    /// respect to.
    #[inline]
    pub fn compute_delta_to_com_moments_and_products_of_inertia(
        mass: f32,
        displacement_to_com: &Vector3,
    ) -> (Vector3, Vector3) {
        let squared_displacement = displacement_to_com.component_mul(displacement_to_com);

        let moment_of_inertia_deltas =
            -mass * (squared_displacement.yzx() + squared_displacement.zxy());

        let product_of_inertia_deltas =
            -mass * (displacement_to_com.component_mul(&displacement_to_com.yzx()));

        (moment_of_inertia_deltas, product_of_inertia_deltas)
    }

    /// Uses the parallel axis theorem to compute the differences that must be
    /// added to the center-of-mass moments and products of inertia for them
    /// to be defined with respect to the point at the given displacement
    /// from the center of mass.
    #[inline]
    pub fn compute_delta_from_com_moments_and_products_of_inertia(
        mass: f32,
        displacement_from_com: &Vector3,
    ) -> (Vector3, Vector3) {
        let (moment_of_inertia_deltas, product_of_inertia_deltas) =
            Self::compute_delta_to_com_moments_and_products_of_inertia(mass, displacement_from_com);
        (-moment_of_inertia_deltas, -product_of_inertia_deltas)
    }

    /// Uses the parallel axis theorem twice to compute the differences that
    /// must be added to the moments and products of inertia for them to be
    /// defined with respect to a point at the given displacement from the
    /// point they are currently defined with respect to.
    #[inline]
    pub fn compute_delta_to_moments_and_products_of_inertia_defined_relative_to_point(
        mass: f32,
        displacement_to_com: &Vector3,
        displacement_to_point: &Vector3,
    ) -> (Vector3, Vector3) {
        let (com_moment_of_inertia_deltas, com_product_of_inertia_deltas) =
            Self::compute_delta_to_com_moments_and_products_of_inertia(mass, displacement_to_com);

        let displacement_from_com_to_point = displacement_to_point - displacement_to_com;
        let (com_to_point_moment_of_inertia_deltas, com_to_point_product_of_inertia_deltas) =
            Self::compute_delta_from_com_moments_and_products_of_inertia(
                mass,
                &displacement_from_com_to_point,
            );

        (
            com_moment_of_inertia_deltas + com_to_point_moment_of_inertia_deltas,
            com_product_of_inertia_deltas + com_to_point_product_of_inertia_deltas,
        )
    }

    /// Converts the tensor to the 4-byte aligned cache-friendly
    /// [`InertiaTensorP`].
    #[inline]
    pub fn compact(&self) -> InertiaTensorP {
        InertiaTensorP::from_matrix_and_inverse(
            self.matrix.compact(),
            self.inverse_matrix.compact(),
        )
    }

    #[cfg(test)]
    fn max_element(&self) -> f32 {
        self.matrix.max_element()
    }
}

impl AbsDiffEq for InertiaTensor {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Matrix3::abs_diff_eq(&self.matrix, &other.matrix, epsilon)
    }
}

impl RelativeEq for InertiaTensor {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        Matrix3::relative_eq(&self.matrix, &other.matrix, epsilon, max_relative)
    }
}

impl InertiaTensorP {
    /// Creates a new identity inertia tensor.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_matrix_and_inverse(Matrix3C::identity(), Matrix3C::identity())
    }

    /// Creates an inertia tensor corresponding to the given matrix and its
    /// inverse.
    #[inline]
    pub const fn from_matrix_and_inverse(matrix: Matrix3C, inverse_matrix: Matrix3C) -> Self {
        Self {
            matrix,
            inverse_matrix,
        }
    }

    /// Creates a new diagonal inertia tensor with the given diagonal elements.
    ///
    /// # Panics
    /// If any of the given elements does not exceed zero.
    #[inline]
    pub fn from_diagonal_elements(j_xx: f32, j_yy: f32, j_zz: f32) -> Self {
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

        let matrix = Matrix3C::from_diagonal(&Vector3C::new(j_xx, j_yy, j_zz));
        let inverse_matrix =
            Matrix3C::from_diagonal(&Vector3C::new(1.0 / j_xx, 1.0 / j_yy, 1.0 / j_zz));

        Self::from_matrix_and_inverse(matrix, inverse_matrix)
    }

    /// Returns a reference to the inertia matrix.
    #[inline]
    pub const fn matrix(&self) -> &Matrix3C {
        &self.matrix
    }

    /// Returns a reference to the inverse of the inertia matrix.
    #[inline]
    pub const fn inverse_matrix(&self) -> &Matrix3C {
        &self.inverse_matrix
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly
    /// [`InertiaTensor`].
    #[inline]
    pub fn aligned(&self) -> InertiaTensor {
        InertiaTensor::from_matrix_and_inverse(self.matrix.aligned(), self.inverse_matrix.aligned())
    }
}

impl AbsDiffEq for InertiaTensorP {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Matrix3C::abs_diff_eq(&self.matrix, &other.matrix, epsilon)
    }
}

impl RelativeEq for InertiaTensorP {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        Matrix3C::relative_eq(&self.matrix, &other.matrix, epsilon, max_relative)
    }
}

#[inline]
pub fn compute_box_volume(extent_x: f32, extent_y: f32, extent_z: f32) -> f32 {
    extent_x * extent_y * extent_z
}

#[inline]
pub fn compute_cylinder_volume(radius: f32, length: f32) -> f32 {
    PI * radius.powi(2) * length
}

#[inline]
pub fn compute_cone_volume(max_radius: f32, length: f32) -> f32 {
    compute_cylinder_volume(max_radius, length) / 3.0
}

#[inline]
pub fn compute_sphere_volume(radius: f32) -> f32 {
    (PI * 4.0 / 3.0) * radius.powi(3)
}

#[inline]
pub fn compute_hemisphere_volume(radius: f32) -> f32 {
    0.5 * compute_sphere_volume(radius)
}

/// Computes the volume inside the surface defined by the given triangles, using
/// the method described in Eberly (2004). The surface is assumed closed, but
/// may contain disjoint parts.
pub fn compute_triangle_mesh_volume<'a>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
) -> f32 {
    let mut volume = 0.0;

    for [vertex_0, vertex_1, vertex_2] in triangle_vertex_positions {
        volume += compute_volume_contribution_for_triangle(vertex_0, vertex_1, vertex_2);
    }

    volume *= 1.0 / 6.0;

    volume
}

/// Computes the mass, center of mass and inertia tensor of a uniformly dense
/// body whose surface represented by the given triangles, using the method
/// described in Eberly (2004). The inertia tensor is defined relative to the
/// center of mass. The mesh is assumed closed, but may contain disjoint parts.
pub fn compute_uniform_triangle_mesh_inertial_properties<'a>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
    mass_density: f32,
) -> (f32, Position, InertiaTensor) {
    let mut mass = 0.0;
    let mut first_moments = Vector3::zeros();
    let mut diagonal_second_moments = Vector3::zeros();
    let mut mixed_second_moments = Vector3::zeros();

    for [vertex_0, vertex_1, vertex_2] in triangle_vertex_positions {
        let (
            zeroth_moment_contrib,
            first_moment_contrib,
            diagonal_second_moment_contrib,
            mixed_second_moment_contrib,
        ) = compute_zeroth_first_and_second_moment_contributions_for_triangle(
            &vertex_0.aligned(),
            &vertex_1.aligned(),
            &vertex_2.aligned(),
        );

        mass += zeroth_moment_contrib;
        first_moments += first_moment_contrib;
        diagonal_second_moments += diagonal_second_moment_contrib;
        mixed_second_moments += mixed_second_moment_contrib;
    }

    mass *= (1.0 / 6.0) * mass_density;
    first_moments *= (1.0 / 24.0) * mass_density;
    diagonal_second_moments *= (1.0 / 60.0) * mass_density;
    mixed_second_moments *= (1.0 / 120.0) * mass_density;

    let center_of_mass = Point3::from(first_moments / mass);

    let j_diag = diagonal_second_moments.yzx() + diagonal_second_moments.zxy();

    let j_xy = -mixed_second_moments.x();
    let j_yz = -mixed_second_moments.y();
    let j_zx = -mixed_second_moments.z();

    let inertia_matrix =
        Matrix3::from_columns(
            Vector3::new(j_diag.x(), j_xy, j_zx),
            Vector3::new(j_xy, j_diag.y(), j_yz),
            Vector3::new(j_zx, j_yz, j_diag.z()),
        ) + InertiaTensor::compute_delta_to_com_inertia_matrix(mass, center_of_mass.as_vector());

    let inertia_tensor = InertiaTensor::from_matrix(inertia_matrix);

    (mass, center_of_mass, inertia_tensor)
}

/// Computes the mass of the unform body whose surface is represented by the
/// given triangles, using the method described in Eberly (2004). The surface is
/// assumed closed, but may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_mass<'a>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
    mass_density: f32,
) -> f32 {
    compute_triangle_mesh_volume(triangle_vertex_positions) * mass_density
}

/// Computes the center of mass of the uniformly dense body whose surface is
/// represented by the given triangles, using the method described in Eberly
/// (2004). The surface is assumed closed, but may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_center_of_mass<'a>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
) -> Position {
    compute_uniform_triangle_mesh_inertial_properties(triangle_vertex_positions, 1.0).1
}

/// Computes the inertia tensor of the uniformly dense body whose surface is
/// represented by the given triangles, using the method described in Eberly
/// (2004). The inertia tensor is defined relative to the center of mass. The
/// surface is assumed closed, but may contain disjoint parts.
#[cfg(test)]
pub fn compute_uniform_triangle_mesh_inertia_tensor<'a>(
    triangle_vertex_positions: impl IntoIterator<Item = [&'a Point3C; 3]>,
    mass_density: f32,
) -> InertiaTensor {
    compute_uniform_triangle_mesh_inertial_properties(triangle_vertex_positions, mass_density).2
}

fn compute_volume_contribution_for_triangle(
    vertex_0: &Point3C,
    vertex_1: &Point3C,
    vertex_2: &Point3C,
) -> f32 {
    let edge_1_y = vertex_1.y() - vertex_0.y();
    let edge_1_z = vertex_1.z() - vertex_0.z();
    let edge_2_y = vertex_2.y() - vertex_0.y();
    let edge_2_z = vertex_2.z() - vertex_0.z();

    (edge_1_y * edge_2_z - edge_2_y * edge_1_z) * (vertex_0.x() + vertex_1.x() + vertex_2.x())
}

fn compute_zeroth_first_and_second_moment_contributions_for_triangle(
    vertex_0: &Point3,
    vertex_1: &Point3,
    vertex_2: &Point3,
) -> (f32, Vector3, Vector3, Vector3) {
    let w_0 = vertex_0.as_vector();
    let w_1 = vertex_1.as_vector();
    let w_2 = vertex_2.as_vector();

    let tmp_0 = w_0 + w_1;
    let tmp_1 = w_0.component_mul(w_0);
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

    let zeroth_moment = edge_cross_prod.x() * f_1.x();

    let first_moments = edge_cross_prod.component_mul(&f_2);

    let diagonal_second_moments = edge_cross_prod.component_mul(&f_3);

    let mixed_second_moments = edge_cross_prod.component_mul(
        &(w_0.yzx().component_mul(&g_0)
            + w_1.yzx().component_mul(&g_1)
            + w_2.yzx().component_mul(&g_2)),
    );

    (
        zeroth_moment,
        first_moments,
        diagonal_second_moments,
        mixed_second_moments,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::abs_diff_eq;
    use impact_math::consts::f32::{FRAC_PI_2, TWO_PI};
    use impact_mesh::{FrontFaceSide, TriangleMesh, TriangleMeshDirtyMask};
    use proptest::prelude::*;
    use std::ops::Range;

    prop_compose! {
        fn rotation_strategy()(
            rotation_y in 0.0..TWO_PI,
            rotation_x in -FRAC_PI_2..FRAC_PI_2,
            rotation_z in 0.0..TWO_PI,
        ) -> UnitQuaternion {
            UnitQuaternion::from_euler_angles_extrinsic(rotation_y, rotation_x, rotation_z)
        }
    }

    prop_compose! {
        fn similarity_transform_strategy(
            max_translation: f32,
            scaling_range: Range<f32>
        )(
            translation_x in -max_translation..max_translation,
            translation_y in -max_translation..max_translation,
            translation_z in -max_translation..max_translation,
            rotation in rotation_strategy(),
            scaling in scaling_range,
        ) -> Similarity3 {
            let translation = Vector3::new(translation_x, translation_y, translation_z);
            Similarity3::from_parts(
                translation,
                rotation,
                scaling
            )
        }
    }

    proptest! {
        #[test]
        fn should_transform_uniform_cube_mass(transform in similarity_transform_strategy(1e3, 1e-3..1e3)) {
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
        fn should_transform_uniform_cube_center_of_mass(transform in similarity_transform_strategy(1e3, 1e-3..1e3)) {
            let mut cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_center_of_mass = *cube_properties.center_of_mass();

            cube_properties.transform(&transform);

            let correctly_transformed_center_of_mass = transform.transform_point(&initial_center_of_mass);

            prop_assert!(abs_diff_eq!(
                cube_properties.center_of_mass(),
                &correctly_transformed_center_of_mass,
                epsilon = 1e-7 * correctly_transformed_center_of_mass.as_vector().component_abs().max_component()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_transform_uniform_cube_inertia_tensor(transform in similarity_transform_strategy(1e3, 1e-3..1e3)) {
            let mut cube_properties = InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0);
            let initial_inertia_tensor = cube_properties.inertia_tensor().clone();

            cube_properties.transform(&transform);

            let correctly_transformed_inertia_tensor = initial_inertia_tensor
                .with_multiplied_mass(transform.scaling().powi(3))
                .with_multiplied_extent(transform.scaling())
                .rotated(transform.rotation());

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
                &rotated_inertia_tensor.matrix().inverse(),
                epsilon = 1e-4
            ));
            prop_assert!(abs_diff_eq!(
                cube_properties.inertia_tensor().inverse_rotated_matrix(&rotation),
                rotated_inertia_tensor.matrix().inverse(),
                epsilon = 1e-4
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_box_mesh_mass(
            extent_x in 1e-3..1e3_f32,
            extent_y in 1e-3..1e3_f32,
            extent_z in 1e-3..1e3_f32,
            mass_density in 1e-3..1e3_f32,
            transform in similarity_transform_strategy(1e3, 1e-3..1e3),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform, &mut TriangleMeshDirtyMask::empty());
            box_properties.transform(&transform);

            let computed_mass = compute_uniform_triangle_mesh_mass(box_mesh.triangle_vertex_positions(), mass_density);
            let correct_mass = box_properties.mass();

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
            extent_x in 1e-3..1e3_f32,
            extent_y in 1e-3..1e3_f32,
            extent_z in 1e-3..1e3_f32,
            mass_density in 1e-3..1e3_f32,
            transform in similarity_transform_strategy(1e3, 1e-3..1e3),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform, &mut TriangleMeshDirtyMask::empty());
            box_properties.transform(&transform);

            let computed_center_of_mass = compute_uniform_triangle_mesh_center_of_mass(box_mesh.triangle_vertex_positions());
            let correct_center_of_mass = box_properties.center_of_mass();

            prop_assert!(abs_diff_eq!(
                computed_center_of_mass,
                correct_center_of_mass,
                epsilon = 1e-1 * correct_center_of_mass.as_vector().component_abs().max_component()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_compute_uniform_box_mesh_inertia_tensor(
            extent_x in 1e-1..1e2_f32,
            extent_y in 1e-1..1e2_f32,
            extent_z in 1e-1..1e2_f32,
            mass_density in 1e-1..1e2_f32,
            transform in similarity_transform_strategy(1e2, 1e-1..1e2),
        ) {
            let mut box_mesh = TriangleMesh::create_box(extent_x, extent_y, extent_z, FrontFaceSide::Outside);
            let mut box_properties = InertialProperties::of_uniform_box(extent_x, extent_y, extent_z, mass_density);

            box_mesh.transform(&transform, &mut TriangleMeshDirtyMask::empty());
            box_properties.transform(&transform);

            let computed_inertia_tensor =
                compute_uniform_triangle_mesh_inertia_tensor(box_mesh.triangle_vertex_positions(), mass_density);
            let correct_inertia_tensor = box_properties.inertia_tensor();

            prop_assert!(abs_diff_eq!(
                computed_inertia_tensor,
                correct_inertia_tensor.clone(),
                epsilon = 1e-1 * correct_inertia_tensor.max_element()
            ));
        }
    }
}
