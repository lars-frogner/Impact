//! Planar collidable geometry.

use crate::material::ContactResponseParameters;
use impact_geometry::Plane;
use impact_math::transform::Isometry3A;

#[derive(Clone, Debug)]
pub struct PlaneCollidable {
    plane: Plane,
    response_params: ContactResponseParameters,
}

impl PlaneCollidable {
    pub fn new(plane: Plane, response_params: ContactResponseParameters) -> Self {
        Self {
            plane,
            response_params,
        }
    }

    pub fn plane(&self) -> &Plane {
        &self.plane
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }

    pub fn transformed(&self, transform: &Isometry3A) -> Self {
        Self {
            plane: self
                .plane
                .aligned()
                .translated_and_rotated(transform)
                .unaligned(),
            response_params: self.response_params,
        }
    }

    pub fn with_response_params(&self, response_params: ContactResponseParameters) -> Self {
        Self {
            plane: self.plane,
            response_params,
        }
    }
}
