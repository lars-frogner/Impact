//! Planar collidable geometry.

use crate::material::ContactResponseParameters;
use impact_geometry::Plane;
use nalgebra::Isometry3;

#[derive(Clone, Debug)]
pub struct PlaneCollidable {
    plane: Plane<f32>,
    response_params: ContactResponseParameters,
}

impl PlaneCollidable {
    pub fn new(plane: Plane<f32>, response_params: ContactResponseParameters) -> Self {
        Self {
            plane,
            response_params,
        }
    }

    pub fn plane(&self) -> &Plane<f32> {
        &self.plane
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }

    pub fn transformed(&self, transform: &Isometry3<f32>) -> Self {
        Self {
            plane: self.plane.translated_and_rotated(transform),
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
