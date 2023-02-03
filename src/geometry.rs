//! Geometrical objects.

mod angle;
mod bounds;
mod camera;
mod frustum;
mod instance;
mod mesh;
mod plane;
mod sphere;
mod tracking;
mod uniform;

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera, CameraConfiguration, PerspectiveCamera};
pub use frustum::Frustum;
pub use instance::{
    DynamicInstanceFeatureBuffer, InstanceFeature, InstanceFeatureID, InstanceFeatureStorage,
    InstanceFeatureTypeID, InstanceModelViewTransform,
};
pub use mesh::{ColorVertex, Mesh, TextureVertex, TriangleMesh};
pub use plane::Plane;
pub use sphere::Sphere;
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
pub use uniform::UniformBuffer;
