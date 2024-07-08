//! Geometrical objects.

mod angle;
mod axis_aligned_box;
mod frustum;
mod oriented_box;
mod plane;
mod projection;
mod sphere;
mod voxel;

pub use angle::{Angle, Degrees, Radians};
pub use axis_aligned_box::AxisAlignedBox;
pub use frustum::Frustum;
pub use oriented_box::OrientedBox;
pub use plane::Plane;
pub use projection::{CubeMapper, CubemapFace, OrthographicTransform, PerspectiveTransform};
pub use sphere::Sphere;
pub use voxel::{
    UniformBoxVoxelGenerator, UniformSphereVoxelGenerator, VoxelPropertyMap, VoxelTree,
    VoxelTreeLODController, VoxelType,
};

use crate::num::Float;
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
