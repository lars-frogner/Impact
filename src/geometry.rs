//! Geometrical objects.

mod angle;
mod bounds;
mod camera;
mod instance;
mod mesh;
mod tracking;

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera, CameraConfiguration, CameraID, CameraRepository, PerspectiveCamera};
pub use instance::{ModelID, ModelInstance, ModelInstanceBuffer, ModelInstancePool};
pub use mesh::{ColorVertex, Mesh, MeshID, MeshRepository, TextureVertex};
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
