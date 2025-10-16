//! Inertial properties of voxel objects.

use super::chunk_voxels;
use crate::{
    Voxel,
    chunks::{CHUNK_SIZE, ChunkedVoxelObject, VoxelChunk, disconnection},
};
use approx::{AbsDiffEq, RelativeEq};
use impact_math::Float;
use impact_physics::{
    fph,
    inertia::{InertiaTensor, InertialProperties},
};
use nalgebra::{Matrix3, Point3, Vector3, vector};
use std::ops::Range;

/// Keeps track of the inertial properties (mass, center of mass and inertia
/// tensor) of a voxel object. Specifically, stores the mass (`m`), moments
/// (`m * x`, `m * y`, `m * z`), moments of inertia (diagonal elements of the
/// inertia tensor: `m * (y^2 + z^2)`, `m * (x^2 + z^2)`, `m * (x^2 + y^2)`) and
/// products of inertia (negative off-diagonal elements of the inertia tensor:
/// `m * x * y`, `m * y * z`, `m * z * x`) integrated over all voxels.
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelObjectInertialPropertyManager {
    mass: fph,
    moments: Vector3<fph>,
    moments_of_inertia: Vector3<fph>,
    products_of_inertia: Vector3<fph>,
}

/// Helper for updating the inertial properties of a voxel object.
#[derive(Debug)]
pub struct VoxelObjectInertialPropertyUpdater<'a, 'b> {
    parent: &'a mut VoxelObjectInertialPropertyManager,
    voxel_type_densities: &'b [f32],
    voxel_extent: f64,
    voxel_extent_pow_2: f64,
    voxel_extent_pow_3: f64,
}

/// Helper for updating the inertial properties of two voxel objects when voxels
/// are transferred between them.
#[derive(Debug)]
pub struct VoxelObjectInertialPropertyTransferrer<'a, 'b> {
    source: &'a mut VoxelObjectInertialPropertyManager,
    destination: &'a mut VoxelObjectInertialPropertyManager,
    voxel_type_densities: &'b [f32],
    voxel_extent: f64,
    voxel_extent_pow_2: f64,
    voxel_extent_pow_3: f64,
}

impl VoxelChunk {
    fn accumulate_moments(
        &self,
        voxel_extent: f64,
        voxels: &[Voxel],
        voxel_type_densities: &[f32],
        chunk_indices: &[usize; 3],
        mass: &mut fph,
        moments: &mut Vector3<fph>,
        moments_of_inertia: &mut Vector3<fph>,
        products_of_inertia: &mut Vector3<fph>,
    ) {
        match self {
            Self::NonUniform(chunk) => {
                let chunk_voxels = chunk_voxels(voxels, chunk.data_offset);
                let (
                    chunk_mass,
                    chunk_moments,
                    chunk_moments_of_inertia,
                    chunk_products_of_inertia,
                ) = compute_moments_for_non_uniform_chunk(
                    voxel_extent,
                    chunk_voxels,
                    voxel_type_densities,
                    chunk_indices,
                );
                *mass += chunk_mass;
                *moments += chunk_moments;
                *moments_of_inertia += chunk_moments_of_inertia;
                *products_of_inertia += chunk_products_of_inertia;
            }
            Self::Uniform(chunk) => {
                let (
                    chunk_mass,
                    chunk_moments,
                    chunk_moments_of_inertia,
                    chunk_products_of_inertia,
                ) = compute_moments_for_uniform_chunk(
                    voxel_extent,
                    voxel_type_densities,
                    chunk.voxel,
                    chunk_indices,
                );
                *mass += chunk_mass;
                *moments += chunk_moments;
                *moments_of_inertia += chunk_moments_of_inertia;
                *products_of_inertia += chunk_products_of_inertia;
            }
            Self::Empty => {}
        }
    }
}

impl VoxelObjectInertialPropertyManager {
    /// Creates a new manager with all inertial properties set to zero.
    pub fn zeroed() -> Self {
        Self::new(0.0, Vector3::zeros(), Vector3::zeros(), Vector3::zeros())
    }

    /// Integrates up the inertial properties of the given voxel object and
    /// returns the result in a new inertial property manager. The inertial
    /// properties will be defined with respect to the origin of the voxel grid.
    ///
    /// The mass density of each voxel type will be looked up in the given
    /// slice.
    pub fn initialized_from(object: &ChunkedVoxelObject, voxel_type_densities: &[f32]) -> Self {
        let (mass, moments, moments_of_inertia, products_of_inertia) =
            compute_inertial_property_moments_for_object(
                object.voxel_extent(),
                object.occupied_chunk_ranges(),
                object.chunk_idx_strides(),
                object.chunks(),
                object.voxels(),
                voxel_type_densities,
            );
        Self::new(mass, moments, moments_of_inertia, products_of_inertia)
    }

    fn new(
        mass: fph,
        moments: Vector3<fph>,
        moments_of_inertia: Vector3<fph>,
        products_of_inertia: Vector3<fph>,
    ) -> Self {
        Self {
            mass,
            moments,
            moments_of_inertia,
            products_of_inertia,
        }
    }

    /// Computes the [`InertialProperties`] (containing the mass, center of mass
    /// and inertia tensor) corresponding to the current inertial properties
    /// in the manager. The inertia tensor will be defined with respect to the
    /// center of mass.
    pub fn derive_inertial_properties(&self) -> InertialProperties {
        Self::compute_inertial_properties_from_moments(
            self.mass,
            &self.moments,
            &self.moments_of_inertia,
            &self.products_of_inertia,
        )
    }

    /// Computes the center of mass corresponding to the current inertial
    /// properties in the manager.
    pub fn derive_center_of_mass(&self) -> Vector3<fph> {
        self.moments / self.mass
    }

    /// Returns a [`VoxelObjectInertialPropertyUpdater`] that can be used to
    /// update the inertial properties incrementally. The inertial properties
    /// are assumed to be defined with respect to the origin of the voxel grid.
    ///
    /// The mass density of each voxel type will be looked up in the given
    /// slice.
    pub fn begin_update<'a, 'b>(
        &'a mut self,
        voxel_extent: f64,
        voxel_type_densities: &'b [f32],
    ) -> VoxelObjectInertialPropertyUpdater<'a, 'b> {
        let voxel_extent_pow_2 = voxel_extent.powi(2);
        let voxel_extent_pow_3 = voxel_extent_pow_2 * voxel_extent;
        VoxelObjectInertialPropertyUpdater {
            parent: self,
            voxel_type_densities,
            voxel_extent,
            voxel_extent_pow_2,
            voxel_extent_pow_3,
        }
    }

    /// Returns a [`VoxelObjectInertialPropertyTransferrer`] that can be used to
    /// incrementally transfer inertial properties from this manager to the
    /// given other manager. The inertial properties for both managers are
    /// assumed to be defined with respect to the origin of the same voxel
    /// grid.
    ///
    /// The mass density of each voxel type will be looked up in the given
    /// slice.
    pub fn begin_transfer_to<'a, 'b>(
        &'a mut self,
        other: &'a mut Self,
        voxel_extent: f64,
        voxel_type_densities: &'b [f32],
    ) -> VoxelObjectInertialPropertyTransferrer<'a, 'b> {
        let voxel_extent_pow_2 = voxel_extent.powi(2);
        let voxel_extent_pow_3 = voxel_extent_pow_2 * voxel_extent;
        VoxelObjectInertialPropertyTransferrer {
            source: self,
            destination: other,
            voxel_type_densities,
            voxel_extent,
            voxel_extent_pow_2,
            voxel_extent_pow_3,
        }
    }

    /// Creates a new manager containing the sum of the inertial properties in
    /// this and the given manager. The inertial properties for both managers
    /// are assumed to be defined with respect to the same point.
    pub fn add(&self, other: &Self) -> Self {
        Self::new(
            self.mass + other.mass,
            self.moments + other.moments,
            self.moments_of_inertia + other.moments_of_inertia,
            self.products_of_inertia + other.products_of_inertia,
        )
    }

    /// Converts the inertial properties to be defined with respect to the
    /// reference point at the given offset from the point they are currently
    /// defined with respect to.
    pub fn offset_reference_point_by(&mut self, offset: &Vector3<f64>) {
        let (moment_of_inertia_deltas, product_of_inertia_deltas) =
            InertiaTensor::compute_delta_to_moments_and_products_of_inertia_defined_relative_to_point(
                self.mass,
                &self.derive_center_of_mass(),
                offset,
            );
        self.moments -= offset * self.mass;
        self.moments_of_inertia += moment_of_inertia_deltas;
        self.products_of_inertia += product_of_inertia_deltas;
    }

    /// Checks that the current inertial properties are consistent with the ones
    /// computed from scratch for the given voxel object. This is for validating
    /// that incremental updates produce the correct result.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_for_object(&self, object: &ChunkedVoxelObject, voxel_type_densities: &[f32]) {
        let from_scratch = Self::initialized_from(object, voxel_type_densities);
        approx::assert_relative_eq!(self, &from_scratch, epsilon = 1e-8, max_relative = 1e-8);
    }

    fn compute_inertial_properties_from_moments(
        mass: fph,
        moments: &Vector3<fph>,
        moments_of_inertia: &Vector3<fph>,
        products_of_inertia: &Vector3<fph>,
    ) -> InertialProperties {
        let center_of_mass = Point3::from(moments / mass);

        // This is the inertia tensor defined with respect to the origin
        #[rustfmt::skip]
        let inertia_tensor_matrix = Matrix3::from_columns(&[
            vector![  moments_of_inertia.x, -products_of_inertia.x, -products_of_inertia.z],
            vector![-products_of_inertia.x,   moments_of_inertia.y, -products_of_inertia.y],
            vector![-products_of_inertia.z, -products_of_inertia.y,   moments_of_inertia.z],
        ]);

        // This is with respect to the center of mass
        let com_inertia_tensor_matrix = inertia_tensor_matrix
            + InertiaTensor::compute_delta_to_com_inertia_matrix(mass, &center_of_mass.coords);

        let inertia_tensor = InertiaTensor::from_matrix(com_inertia_tensor_matrix);

        InertialProperties::new(mass, center_of_mass, inertia_tensor)
    }
}

impl AbsDiffEq for VoxelObjectInertialPropertyManager {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        fph::abs_diff_eq(&self.mass, &other.mass, epsilon)
            && Vector3::abs_diff_eq(&self.moments, &other.moments, epsilon)
            && Vector3::abs_diff_eq(&self.moments_of_inertia, &other.moments_of_inertia, epsilon)
            && Vector3::abs_diff_eq(
                &self.products_of_inertia,
                &other.products_of_inertia,
                epsilon,
            )
    }
}

impl RelativeEq for VoxelObjectInertialPropertyManager {
    fn default_max_relative() -> Self::Epsilon {
        fph::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        fph::relative_eq(&self.mass, &other.mass, epsilon, max_relative)
            && Vector3::relative_eq(&self.moments, &other.moments, epsilon, max_relative)
            && Vector3::relative_eq(
                &self.moments_of_inertia,
                &other.moments_of_inertia,
                epsilon,
                max_relative,
            )
            && Vector3::relative_eq(
                &self.products_of_inertia,
                &other.products_of_inertia,
                epsilon,
                max_relative,
            )
    }
}

impl VoxelObjectInertialPropertyUpdater<'_, '_> {
    /// Updates the inertial properties to account for the given voxel being
    /// removed.
    pub fn remove_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel) {
        let (voxel_mass, voxel_moments, voxel_moments_of_inertia, voxel_products_of_inertia) =
            compute_moments_for_voxel(
                self.voxel_extent,
                self.voxel_extent_pow_2,
                self.voxel_extent_pow_3,
                self.voxel_type_densities,
                object_voxel_indices,
                voxel,
            );
        self.parent.mass -= voxel_mass;
        self.parent.moments -= voxel_moments;
        self.parent.moments_of_inertia -= voxel_moments_of_inertia;
        self.parent.products_of_inertia -= voxel_products_of_inertia;
    }
}

impl VoxelObjectInertialPropertyTransferrer<'_, '_> {
    /// Updates the inertial properties of both the source and destination
    /// object to account for the given voxel being transferred from the
    /// source to the destination.
    pub fn transfer_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel) {
        let (voxel_mass, voxel_moments, voxel_moments_of_inertia, voxel_products_of_inertia) =
            compute_moments_for_voxel(
                self.voxel_extent,
                self.voxel_extent_pow_2,
                self.voxel_extent_pow_3,
                self.voxel_type_densities,
                object_voxel_indices,
                voxel,
            );

        self.source.mass -= voxel_mass;
        self.source.moments -= voxel_moments;
        self.source.moments_of_inertia -= voxel_moments_of_inertia;
        self.source.products_of_inertia -= voxel_products_of_inertia;

        self.destination.mass += voxel_mass;
        self.destination.moments += voxel_moments;
        self.destination.moments_of_inertia += voxel_moments_of_inertia;
        self.destination.products_of_inertia += voxel_products_of_inertia;
    }

    /// Updates the inertial properties of both the source and destination
    /// object to account for the given whole non-uniform chunk being
    /// transferred from the source to the destination.
    pub fn transfer_non_uniform_chunk(
        &mut self,
        chunk_indices: &[usize; 3],
        chunk_voxels: &[Voxel],
    ) {
        let (chunk_mass, chunk_moments, chunk_moments_of_inertia, chunk_products_of_inertia) =
            compute_moments_for_non_uniform_chunk(
                self.voxel_extent,
                chunk_voxels,
                self.voxel_type_densities,
                chunk_indices,
            );

        self.source.mass -= chunk_mass;
        self.source.moments -= chunk_moments;
        self.source.moments_of_inertia -= chunk_moments_of_inertia;
        self.source.products_of_inertia -= chunk_products_of_inertia;

        self.destination.mass += chunk_mass;
        self.destination.moments += chunk_moments;
        self.destination.moments_of_inertia += chunk_moments_of_inertia;
        self.destination.products_of_inertia += chunk_products_of_inertia;
    }

    /// Updates the inertial properties of both the source and destination
    /// object to account for the given whole uniform chunk being transferred
    /// from the source to the destination.
    pub fn transfer_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxel: Voxel) {
        let (chunk_mass, chunk_moments, chunk_moments_of_inertia, chunk_products_of_inertia) =
            compute_moments_for_uniform_chunk(
                self.voxel_extent,
                self.voxel_type_densities,
                chunk_voxel,
                chunk_indices,
            );

        self.source.mass -= chunk_mass;
        self.source.moments -= chunk_moments;
        self.source.moments_of_inertia -= chunk_moments_of_inertia;
        self.source.products_of_inertia -= chunk_products_of_inertia;

        self.destination.mass += chunk_mass;
        self.destination.moments += chunk_moments;
        self.destination.moments_of_inertia += chunk_moments_of_inertia;
        self.destination.products_of_inertia += chunk_products_of_inertia;
    }
}

impl disconnection::PropertyTransferrer for VoxelObjectInertialPropertyTransferrer<'_, '_> {
    fn transfer_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel) {
        self.transfer_voxel(object_voxel_indices, voxel);
    }

    fn transfer_non_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxels: &[Voxel]) {
        self.transfer_non_uniform_chunk(chunk_indices, chunk_voxels);
    }

    fn transfer_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxel: Voxel) {
        self.transfer_uniform_chunk(chunk_indices, chunk_voxel);
    }
}

/// This uses the equations for the integrated inertial properties of a cube.
fn compute_moments_for_voxel(
    voxel_extent: f64,
    voxel_extent_pow_2: f64,
    voxel_extent_pow_3: f64,
    voxel_type_densities: &[f32],
    object_voxel_indices: &[usize; 3],
    voxel: Voxel,
) -> (fph, Vector3<fph>, Vector3<fph>, Vector3<fph>) {
    let voxel_density = fph::from(voxel_type_densities[voxel.voxel_type().idx()]);

    let lower_coords = Vector3::from(object_voxel_indices.map(|index| voxel_extent * index as f64));
    let upper_coords = lower_coords.add_scalar(voxel_extent);

    let lower_coords_squared = lower_coords.component_mul(&lower_coords);
    let upper_coords_squared = upper_coords.component_mul(&upper_coords);

    let lower_coords_cubed = lower_coords_squared.component_mul(&lower_coords);
    let upper_coords_cubed = upper_coords_squared.component_mul(&upper_coords);

    let squared_coord_diff = upper_coords_squared - lower_coords_squared;
    let cubed_coord_diff = upper_coords_cubed - lower_coords_cubed;

    let mass = voxel_extent_pow_3 * voxel_density;

    let moments = (f64::ONE_HALF * voxel_extent_pow_2 * voxel_density) * squared_coord_diff;

    let moments_of_inertia = (f64::ONE_THIRD * voxel_extent_pow_2 * voxel_density)
        * vector![
            cubed_coord_diff[1] + cubed_coord_diff[2],
            cubed_coord_diff[0] + cubed_coord_diff[2],
            cubed_coord_diff[0] + cubed_coord_diff[1]
        ];

    let products_of_inertia = (f64::ONE_FOURTH * voxel_extent * voxel_density)
        * vector![
            squared_coord_diff[0] * squared_coord_diff[1],
            squared_coord_diff[1] * squared_coord_diff[2],
            squared_coord_diff[2] * squared_coord_diff[0]
        ];

    (mass, moments, moments_of_inertia, products_of_inertia)
}

/// This uses the equations for the integrated inertial properties of a cube to
/// compute the properties of each voxel, which is summed up over all voxels.
fn compute_moments_for_non_uniform_chunk(
    voxel_extent: f64,
    chunk_voxels: &[Voxel],
    voxel_type_densities: &[f32],
    chunk_indices: &[usize; 3],
) -> (fph, Vector3<fph>, Vector3<fph>, Vector3<fph>) {
    let mut mass = 0.0;
    let mut moments = Vector3::zeros();
    let mut moments_of_inertia = Vector3::zeros();
    let mut products_of_inertia = Vector3::zeros();

    // Position of the lower corner of the voxel in the lower chunk corner
    let x0 = ((chunk_indices[0] * CHUNK_SIZE) as f64) * voxel_extent;
    let y0 = ((chunk_indices[1] * CHUNK_SIZE) as f64) * voxel_extent;
    let z0 = ((chunk_indices[2] * CHUNK_SIZE) as f64) * voxel_extent;

    let mut voxel_idx = 0;

    let mut xl = x0;
    let mut xh = xl + voxel_extent;

    for _ in 0..CHUNK_SIZE {
        let mut yl = y0;
        let mut yh = yl + voxel_extent;

        let xl2 = xl * xl;
        let xh2 = xh * xh;
        let xl3 = xl2 * xl;
        let xh3 = xh2 * xh;
        let xh2_sub_xl2 = xh2 - xl2;
        let xh3_sub_xl3 = xh3 - xl3;

        for _ in 0..CHUNK_SIZE {
            let mut zl = z0;
            let mut zh = zl + voxel_extent;

            let yl2 = yl * yl;
            let yh2 = yh * yh;
            let yl3 = yl2 * yl;
            let yh3 = yh2 * yh;
            let yh2_sub_yl2 = yh2 - yl2;
            let yh3_sub_yl3 = yh3 - yl3;

            for _ in 0..CHUNK_SIZE {
                let voxel = &chunk_voxels[voxel_idx];
                if !voxel.is_empty() {
                    let zl2 = zl * zl;
                    let zh2 = zh * zh;
                    let zl3 = zl2 * zl;
                    let zh3 = zh2 * zh;
                    let zh2_sub_zl2 = zh2 - zl2;
                    let zh3_sub_zl3 = zh3 - zl3;

                    let voxel_density = fph::from(voxel_type_densities[voxel.voxel_type().idx()]);

                    mass += voxel_density;
                    moments += voxel_density * vector![xh2_sub_xl2, yh2_sub_yl2, zh2_sub_zl2];
                    moments_of_inertia += voxel_density
                        * vector![
                            yh3_sub_yl3 + zh3_sub_zl3,
                            xh3_sub_xl3 + zh3_sub_zl3,
                            xh3_sub_xl3 + yh3_sub_yl3
                        ];
                    products_of_inertia += voxel_density
                        * vector![
                            xh2_sub_xl2 * yh2_sub_yl2,
                            yh2_sub_yl2 * zh2_sub_zl2,
                            zh2_sub_zl2 * xh2_sub_xl2
                        ];
                }
                voxel_idx += 1;
                zl = zh;
                zh += voxel_extent;
            }
            yl = yh;
            yh += voxel_extent;
        }
        xl = xh;
        xh += voxel_extent;
    }

    let voxel_extent_pow_2 = voxel_extent.powi(2);
    let voxel_extent_pow_3 = voxel_extent_pow_2 * voxel_extent;

    mass *= voxel_extent_pow_3;
    moments *= f64::ONE_HALF * voxel_extent_pow_2;
    moments_of_inertia *= f64::ONE_THIRD * voxel_extent_pow_2;
    products_of_inertia *= f64::ONE_FOURTH * voxel_extent;

    (mass, moments, moments_of_inertia, products_of_inertia)
}

/// This uses the equations for the integrated inertial properties of a cube.
fn compute_moments_for_uniform_chunk(
    voxel_extent: f64,
    voxel_type_densities: &[f32],
    chunk_voxel: Voxel,
    chunk_indices: &[usize; 3],
) -> (fph, Vector3<fph>, Vector3<fph>, Vector3<fph>) {
    let density = fph::from(voxel_type_densities[chunk_voxel.voxel_type().idx()]);

    let chunk_extent = (CHUNK_SIZE as f64) * voxel_extent;

    let xl = (chunk_indices[0] as f64) * chunk_extent;
    let xh = xl + chunk_extent;
    let xl2 = xl * xl;
    let xh2 = xh * xh;
    let xl3 = xl2 * xl;
    let xh3 = xh2 * xh;
    let xh2_sub_xl2 = xh2 - xl2;
    let xh3_sub_xl3 = xh3 - xl3;

    let yl = (chunk_indices[1] as f64) * chunk_extent;
    let yh = yl + chunk_extent;
    let yl2 = yl * yl;
    let yh2 = yh * yh;
    let yl3 = yl2 * yl;
    let yh3 = yh2 * yh;
    let yh2_sub_yl2 = yh2 - yl2;
    let yh3_sub_yl3 = yh3 - yl3;

    let zl = (chunk_indices[2] as f64) * chunk_extent;
    let zh = zl + chunk_extent;
    let zl2 = zl * zl;
    let zh2 = zh * zh;
    let zl3 = zl2 * zl;
    let zh3 = zh2 * zh;
    let zh2_sub_zl2 = zh2 - zl2;
    let zh3_sub_zl3 = zh3 - zl3;

    let chunk_extent_pow_2 = chunk_extent.powi(2);
    let chunk_extent_pow_3 = chunk_extent_pow_2 * chunk_extent;

    let mass = chunk_extent_pow_3 * density;
    let moments = (f64::ONE_HALF * chunk_extent_pow_2 * density)
        * vector![xh2_sub_xl2, yh2_sub_yl2, zh2_sub_zl2];
    let moments_of_inertia = (f64::ONE_THIRD * chunk_extent_pow_2 * density)
        * vector![
            yh3_sub_yl3 + zh3_sub_zl3,
            xh3_sub_xl3 + zh3_sub_zl3,
            xh3_sub_xl3 + yh3_sub_yl3
        ];
    let products_of_inertia = (f64::ONE_FOURTH * chunk_extent * density)
        * vector![
            xh2_sub_xl2 * yh2_sub_yl2,
            yh2_sub_yl2 * zh2_sub_zl2,
            zh2_sub_zl2 * xh2_sub_xl2
        ];

    (mass, moments, moments_of_inertia, products_of_inertia)
}

fn compute_inertial_property_moments_for_object(
    voxel_extent: f64,
    occupied_chunk_ranges: &[Range<usize>; 3],
    chunk_idx_strides: &[usize; 3],
    chunks: &[VoxelChunk],
    voxels: &[Voxel],
    voxel_type_densities: &[f32],
) -> (fph, Vector3<fph>, Vector3<fph>, Vector3<fph>) {
    let mut mass = 0.0;
    let mut moments = Vector3::zeros();
    let mut moments_of_inertia = Vector3::zeros();
    let mut products_of_inertia = Vector3::zeros();

    for chunk_i in occupied_chunk_ranges[0].clone() {
        for chunk_j in occupied_chunk_ranges[1].clone() {
            for chunk_k in occupied_chunk_ranges[2].clone() {
                let chunk_indices = [chunk_i, chunk_j, chunk_k];
                let chunk_idx =
                    chunk_i * chunk_idx_strides[0] + chunk_j * chunk_idx_strides[1] + chunk_k;
                let chunk = &chunks[chunk_idx];
                chunk.accumulate_moments(
                    voxel_extent,
                    voxels,
                    voxel_type_densities,
                    &chunk_indices,
                    &mut mass,
                    &mut moments,
                    &mut moments_of_inertia,
                    &mut products_of_inertia,
                );
            }
        }
    }
    (mass, moments, moments_of_inertia, products_of_inertia)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chunks::{CHUNK_VOXEL_COUNT, LoopForChunkVoxels, LoopOverChunkVoxelData},
        generation::{SDFVoxelGenerator, sdf::BoxSDFGenerator, voxel_type::SameVoxelTypeGenerator},
        voxel_types::VoxelType,
    };
    use approx::{assert_abs_diff_eq, assert_relative_eq};
    use nalgebra::{Similarity3, UnitQuaternion};
    use std::array;

    #[test]
    fn full_non_uniform_chunk_has_same_inertial_properties_as_uniform_chunk() {
        let voxel_extent = 0.1;
        let voxel = Voxel::maximally_inside(VoxelType::from_idx(0));
        let voxels = vec![voxel; CHUNK_VOXEL_COUNT];
        let voxel_type_densities = [0.5];
        let chunk_indices = [1, 2, 3];

        let (
            uniform_mass,
            uniform_moments,
            uniform_moments_of_inertia,
            uniform_products_of_inertia,
        ) = compute_moments_for_uniform_chunk(
            voxel_extent,
            &voxel_type_densities,
            voxel,
            &chunk_indices,
        );

        let (
            non_uniform_mass,
            non_uniform_moments,
            non_uniform_moments_of_inertia,
            non_uniform_products_of_inertia,
        ) = compute_moments_for_non_uniform_chunk(
            voxel_extent,
            &voxels,
            &voxel_type_densities,
            &chunk_indices,
        );

        assert_relative_eq!(non_uniform_mass, uniform_mass, epsilon = 1e-6);
        assert_relative_eq!(non_uniform_moments, uniform_moments, epsilon = 1e-6);
        assert_relative_eq!(
            non_uniform_moments_of_inertia,
            uniform_moments_of_inertia,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            non_uniform_products_of_inertia,
            uniform_products_of_inertia,
            epsilon = 1e-6
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn box_voxel_object_has_box_inertial_properties() {
        let voxel_extent = 0.1;
        let extents = [22.0, 27.0, 19.0];
        let mass_densities = [0.5];

        let generator = SDFVoxelGenerator::new(
            voxel_extent,
            BoxSDFGenerator::new(extents).into(),
            SameVoxelTypeGenerator::new(VoxelType::from_idx(0)).into(),
        );
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);

        let occupied_voxel_ranges = object.determine_tight_occupied_voxel_ranges();
        let occupied_voxel_extents = occupied_voxel_ranges
            .clone()
            .map(|range| voxel_extent * range.len() as fph);
        let occupied_voxel_range_centers = occupied_voxel_ranges
            .clone()
            .map(|range| 0.5 * voxel_extent * (range.start + range.end) as fph);

        let object_inertial_properties =
            VoxelObjectInertialPropertyManager::initialized_from(&object, &mass_densities)
                .derive_inertial_properties();

        let mut box_inertial_properties = InertialProperties::of_uniform_box(
            occupied_voxel_extents[0],
            occupied_voxel_extents[1],
            occupied_voxel_extents[2],
            fph::from(mass_densities[0]),
        );

        box_inertial_properties.transform(&Similarity3::from_parts(
            Vector3::from(occupied_voxel_range_centers).into(),
            UnitQuaternion::identity(),
            1.0,
        ));

        assert_relative_eq!(
            object_inertial_properties,
            box_inertial_properties,
            epsilon = 1e-6
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn chunk_has_zero_moments_after_removing_each_voxel() {
        let voxel_extent = 0.1;
        let voxel = Voxel::maximally_inside(VoxelType::from_idx(0));
        let voxels = vec![voxel; CHUNK_VOXEL_COUNT];
        let voxel_type_densities = [0.5];
        let chunk_indices = [1, 2, 3];

        let (mass, moments, moments_of_inertia, products_of_inertia) =
            compute_moments_for_non_uniform_chunk(
                voxel_extent,
                &voxels,
                &voxel_type_densities,
                &chunk_indices,
            );

        let mut inertial_property_manager = VoxelObjectInertialPropertyManager::new(
            mass,
            moments,
            moments_of_inertia,
            products_of_inertia,
        );

        {
            let mut updater =
                inertial_property_manager.begin_update(voxel_extent, &voxel_type_densities);
            let chunk_lower_object_voxel_indices = chunk_indices.map(|index| index * CHUNK_SIZE);
            LoopOverChunkVoxelData::new(&LoopForChunkVoxels::over_all(), &voxels).execute(
                &mut |voxel_indices, voxel| {
                    let object_voxel_indices = array::from_fn(|dim| {
                        chunk_lower_object_voxel_indices[dim] + voxel_indices[dim]
                    });
                    updater.remove_voxel(&object_voxel_indices, *voxel);
                },
            );
        } // <- `updater` drops here

        assert_abs_diff_eq!(inertial_property_manager.mass, 0.0, epsilon = 1e-8);
        assert_abs_diff_eq!(
            inertial_property_manager.moments,
            Vector3::zeros(),
            epsilon = 1e-8
        );
        assert_abs_diff_eq!(
            inertial_property_manager.moments_of_inertia,
            Vector3::zeros(),
            epsilon = 1e-8
        );
        assert_abs_diff_eq!(
            inertial_property_manager.products_of_inertia,
            Vector3::zeros(),
            epsilon = 1e-8
        );
    }
}
