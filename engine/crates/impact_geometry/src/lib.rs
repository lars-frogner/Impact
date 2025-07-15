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
pub use oriented_box::OrientedBox;
pub use plane::Plane;
pub use projection::{CubeMapper, CubemapFace, OrthographicTransform, PerspectiveTransform};
pub use reference_frame::ReferenceFrame;
pub use sphere::Sphere;

use impact_math::Float;
use nalgebra::Point3;

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
