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

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera, CameraConfiguration, CameraID, CameraRepository, PerspectiveCamera};
pub use frustum::Frustum;
pub use instance::{ModelID, ModelInstance, ModelInstanceBuffer, ModelInstancePool};
pub use mesh::{ColorVertex, Mesh, MeshID, MeshRepository, TextureVertex};
pub use plane::Plane;
pub use sphere::Sphere;
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
