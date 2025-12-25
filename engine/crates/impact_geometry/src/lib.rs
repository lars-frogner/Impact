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

pub use axis_aligned_box::{AxisAlignedBox, AxisAlignedBoxA};
pub use capsule::{Capsule, CapsuleA};
pub use frustum::{Frustum, FrustumA};
pub use model_transform::{ModelTransform, ModelTransformA};
pub use oriented_box::{OrientedBox, OrientedBoxA};
pub use plane::{Plane, PlaneA};
pub use reference_frame::{ReferenceFrame, ReferenceFrameA};
pub use sphere::{Sphere, SphereA};

use impact_math::{
    angle::{Angle, Radians},
    consts::f32::PI,
    vector::{UnitVector3A, Vector3A},
};

/// Uses the Frisvad method.
pub fn orthonormal_basis_with_z_axis(
    z: UnitVector3A,
) -> (UnitVector3A, UnitVector3A, UnitVector3A) {
    let zx = z.x();
    let zy = z.y();
    let zz = z.z();

    let sign = if zz >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (sign + zz);
    let b = zx * zy * a;

    let x = Vector3A::new(1.0 + sign * zx * zx * a, sign * b, -sign * zx);
    let y = Vector3A::new(b, sign + zy * zy * a, -zy);

    let x = UnitVector3A::normalized_from(x);
    let y = UnitVector3A::normalized_from(y);

    (x, y, z)
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
) -> impl Iterator<Item = UnitVector3A> {
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

        UnitVector3A::normalized_from(Vector3A::new(x, y, z))
    })
}

fn compute_golden_angle() -> Radians<f32> {
    Radians(PI * (3.0 - 5.0_f32.sqrt()))
}
