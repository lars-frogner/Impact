//! Geometrical objects.

#[macro_use]
mod macros;

mod axis_aligned_box;
mod capsule;
mod frustum;
mod model_transform;
mod oriented_box;
mod plane;
mod projection;
mod reference_frame;
mod sphere;

pub use axis_aligned_box::AxisAlignedBox;
pub use capsule::Capsule;
pub use frustum::Frustum;
pub use model_transform::ModelTransform;
pub use oriented_box::{OrientedBox, compute_box_intersection_bounds};
pub use plane::Plane;
pub use projection::{CubeMapper, CubemapFace, OrthographicTransform, PerspectiveTransform};
pub use reference_frame::ReferenceFrame;
pub use sphere::Sphere;

use impact_math::Float;
use nalgebra::{Point3, UnitQuaternion, UnitVector3, Vector3};

/// Anything that represents a 3D point.
pub trait Point<F: Float> {
    /// Returns a reference to the point.
    fn point(&self) -> &Point3<F>;
}

impl<F: Float> Point<F> for Point3<F> {
    fn point(&self) -> &Point3<F> {
        self
    }
}

pub fn rotation_between_axes<F: Float>(
    a: &UnitVector3<F>,
    b: &UnitVector3<F>,
) -> UnitQuaternion<F> {
    if let Some(rotation) = UnitQuaternion::rotation_between_axis(a, b) {
        rotation
    } else {
        // If the axes are antiparallel, we pick a suitable axis about which to
        // flip `a`
        let axis_most_orthogonal_to_a = if a.x.abs() < a.y.abs() && a.x.abs() < a.z.abs() {
            Vector3::x()
        } else if a.y.abs() < a.z.abs() {
            Vector3::y()
        } else {
            Vector3::z()
        };
        let axis_perpendicular_to_a =
            UnitVector3::new_normalize(a.cross(&axis_most_orthogonal_to_a));

        UnitQuaternion::from_axis_angle(&axis_perpendicular_to_a, <F as Float>::PI)
    }
}
