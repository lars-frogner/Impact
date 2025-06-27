//! Planar collidable geometry.

use crate::physics::fph;
use impact_geometry::Plane;
use nalgebra::Similarity3;

#[derive(Clone, Debug)]
pub struct PlaneCollidableGeometry {
    plane: Plane<fph>,
}

impl PlaneCollidableGeometry {
    pub fn new(plane: Plane<fph>) -> Self {
        Self { plane }
    }

    pub fn plane(&self) -> &Plane<fph> {
        &self.plane
    }

    pub fn transformed(&self, transform: &Similarity3<fph>) -> Self {
        Self {
            plane: self.plane.transformed(transform),
        }
    }
}
