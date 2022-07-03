//! Geometrical objects.

mod angle;
mod bounds;
mod camera;
mod vertex;

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use camera::{Camera3, CameraConfiguration3, PerspectiveCamera3};
pub use vertex::{Vertex, VertexWithTexture};
