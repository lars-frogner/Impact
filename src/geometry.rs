//! Geometrical objects.

mod angle;
mod axis_aligned_box;
mod bounds;
mod camera;
mod frustum;
mod instance;
mod mesh;
mod oriented_box;
mod plane;
mod projection;
mod sphere;
mod tracking;
mod uniform;

pub use angle::{Angle, Degrees, Radians};
pub use axis_aligned_box::AxisAlignedBox;
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera, OrthographicCamera, PerspectiveCamera};
pub use frustum::Frustum;
pub use instance::{
    DynamicInstanceFeatureBuffer, InstanceFeature, InstanceFeatureBufferRangeID,
    InstanceFeatureBufferRangeMap, InstanceFeatureID, InstanceFeatureStorage,
    InstanceFeatureTypeID, InstanceModelLightTransform, InstanceModelViewTransform,
};
pub use mesh::{
    TriangleMesh, VertexAttribute, VertexAttributeSet, VertexColor, VertexNormalVector,
    VertexPosition, VertexTextureCoords, N_VERTEX_ATTRIBUTES, VERTEX_ATTRIBUTE_FLAGS,
    VERTEX_ATTRIBUTE_NAMES,
};
pub use oriented_box::OrientedBox;
pub use plane::Plane;
pub use projection::{CubeMapFace, CubeMapper, OrthographicTransform, PerspectiveTransform};
pub use sphere::Sphere;
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
pub use uniform::UniformBuffer;

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
