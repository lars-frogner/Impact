//! Geometrical objects.

#[macro_use]
mod macros;

pub mod axis_aligned_box;
pub mod capsule;
pub mod frustum;
pub mod model_transform;
pub mod oriented_box;
pub mod plane;
pub mod projection;
pub mod reference_frame;
pub mod sphere;

pub use axis_aligned_box::AxisAlignedBox;
pub use capsule::Capsule;
pub use frustum::Frustum;
pub use model_transform::ModelTransform;
pub use oriented_box::OrientedBox;
pub use plane::Plane;
pub use reference_frame::ReferenceFrame;
pub use sphere::Sphere;

use impact_math::{
    angle::{Angle, Radians},
    consts::f32::PI,
    quaternion::UnitQuaternion,
    vector::{UnitVector3, Vector3},
};

/// Uses the Frisvad method.
pub fn orthonormal_basis_with_z_axis(z: UnitVector3) -> (UnitVector3, UnitVector3, UnitVector3) {
    let zx = z.x();
    let zy = z.y();
    let zz = z.z();

    let sign = if zz >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (sign + zz);
    let b = zx * zy * a;

    let x = Vector3::new(1.0 + sign * zx * zx * a, sign * b, -sign * zx);
    let y = Vector3::new(b, sign + zy * zy * a, -zy);

    let x = UnitVector3::normalized_from(x);
    let y = UnitVector3::normalized_from(y);

    (x, y, z)
}

pub fn rotation_between_axes(a: &UnitVector3, b: &UnitVector3) -> UnitQuaternion {
    if let Some(rotation) = UnitQuaternion::rotation_between_axis(a, b) {
        rotation
    } else {
        // If the axes are antiparallel, we pick a suitable axis about which to
        // flip `a`
        let axis_most_orthogonal_to_a = cartesian_axis_most_orthogonal_to_vector(a);
        let axis_perpendicular_to_a =
            UnitVector3::normalized_from(a.cross(&axis_most_orthogonal_to_a));

        UnitQuaternion::from_axis_angle(&axis_perpendicular_to_a, PI)
    }
}

pub fn cartesian_axis_most_orthogonal_to_vector(vector: &Vector3) -> UnitVector3 {
    if vector.x().abs() < vector.y().abs() && vector.x().abs() < vector.z().abs() {
        UnitVector3::unit_x()
    } else if vector.y().abs() < vector.z().abs() {
        UnitVector3::unit_y()
    } else {
        UnitVector3::unit_z()
    }
}

/// Computes the given number of radial directions, making them close to
/// uniformly distributed.
///
/// # Returns
/// An iterator over the directions.
///
/// # Panics
/// If the given number of directions is zero.
pub fn compute_uniformly_distributed_radial_directions(
    n_direction_samples: usize,
) -> impl Iterator<Item = UnitVector3> {
    let idx_norm = if n_direction_samples > 1 {
        (n_direction_samples - 1) as f32
    } else {
        1.0
    }
    .recip();
    let golden_angle = compute_golden_angle();

    (0..n_direction_samples).map(move |idx| {
        let idx = idx as f32;

        // Distribute evenly in z
        let z = 1.0 - 2.0 * idx * idx_norm;
        let horizontal_radius = (1.0 - z.powi(2)).sqrt();

        // Use golden angle to space the azimuthal angles, giving a close to
        // uniform distribution over the sphere
        let azimuthal_angle = idx * golden_angle.radians();

        let (sin_azimuthal_angle, cos_azimuthal_angle) = azimuthal_angle.sin_cos();
        let x = horizontal_radius * cos_azimuthal_angle;
        let y = horizontal_radius * sin_azimuthal_angle;

        UnitVector3::normalized_from(Vector3::new(x, y, z))
    })
}

fn compute_golden_angle() -> Radians<f32> {
    Radians(PI * (3.0 - 5.0_f32.sqrt()))
}
