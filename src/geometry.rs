//! Geometrical objects.

mod angle;
mod bounds;
mod camera;
mod frustum;
mod instance;
mod mesh;
mod plane;
mod scene;
mod sphere;
mod tracking;

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera, CameraConfiguration, CameraID, CameraRepository, PerspectiveCamera};
pub use frustum::Frustum;
pub use instance::{ModelID, ModelInstance, ModelInstanceBuffer, ModelInstancePool};
pub use mesh::{ColorVertex, MeshID, MeshRepository, TextureVertex, TriangleMesh};
pub use plane::Plane;
pub use scene::{CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage, SceneGraph};
pub use sphere::Sphere;
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
