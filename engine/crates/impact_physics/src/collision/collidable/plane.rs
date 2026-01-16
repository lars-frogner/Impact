//! Planar collidable geometry.

use crate::material::ContactResponseParameters;
use impact_geometry::PlaneC;
use impact_math::transform::Isometry3;

#[derive(Clone, Debug)]
pub struct PlaneCollidable {
    plane: PlaneC,
    response_params: ContactResponseParameters,
}

impl PlaneCollidable {
    pub fn new(plane: PlaneC, response_params: ContactResponseParameters) -> Self {
        Self {
            plane,
            response_params,
        }
    }

    pub fn plane(&self) -> &PlaneC {
        &self.plane
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }

    pub fn transformed(&self, transform: &Isometry3) -> Self {
        let plane = self.plane.aligned();
        let transformed_plane = plane.iso_transformed(transform);
        Self {
            plane: transformed_plane.compact(),
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
