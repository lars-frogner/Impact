//! Texture projection.

use anyhow::{Result, bail};
use approx::abs_diff_eq;
use impact_math::Float;
use nalgebra::{Point3, UnitVector3, Vector2, Vector3, vector};

/// Represents a projection of 3D positions into UV texture coordinates.
pub trait TextureProjection<F: Float> {
    /// Returns a string uniquely identifying the projection.
    fn identifier(&self) -> String;

    /// Computes the UV texture coordinates for the given position.
    fn project_position(&self, position: &Point3<F>) -> Vector2<F>;
}

/// Projection of 3D positions onto a plane defined by an origin and two vectors
/// defining the axes along which the U and V texture coordinates will increase.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarTextureProjection<F: Float> {
    origin: Point3<F>,
    u_direction: UnitVector3<F>,
    v_normal_to_u_direction: UnitVector3<F>,
    v_direction_comp_along_u_direction: F,
    inverse_v_direction_comp_normal_to_u_direction: F,
    inverse_u_vector_length: F,
    inverse_v_vector_length: F,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum TextureProjectionSpecification {
    Planar {
        origin: Point3<f32>,
        u_vector: Vector3<f32>,
        v_vector: Vector3<f32>,
    },
}

impl<F: Float> PlanarTextureProjection<F> {
    /// Creates a projection onto the plane defined by the given origin and two
    /// vectors defining the axes along which the U and V texture coordinates
    /// will increase. The texture coordinates will be zero at the origin and
    /// unity at the tip of the respective u- or v-vector.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If the u- or v-vector has zero length.
    /// - If the u- and v-vectors are colinear.
    pub fn new(origin: Point3<F>, u_vector: Vector3<F>, v_vector: Vector3<F>) -> Result<Self> {
        let (u_direction, u_vector_length) = UnitVector3::new_and_get(u_vector);
        if abs_diff_eq!(u_vector_length, F::ZERO) {
            bail!("u_vector has zero length");
        }
        let (v_direction, v_vector_length) = UnitVector3::new_and_get(v_vector);
        if abs_diff_eq!(v_vector_length, F::ZERO) {
            bail!("v_vector has zero length");
        }

        let (v_normal_to_u_direction, v_normal_to_u_length) = UnitVector3::new_and_get(
            v_direction.as_ref() - u_direction.as_ref() * v_direction.dot(&u_direction),
        );
        if abs_diff_eq!(v_normal_to_u_length, F::ZERO) {
            bail!("u_vector and v_vector are parallel");
        }

        let v_direction_comp_along_u_direction = v_direction.dot(&u_direction);
        let inverse_v_direction_comp_normal_to_u_direction =
            F::ONE / v_direction.dot(&v_normal_to_u_direction);

        Ok(Self {
            origin,
            u_direction,
            v_normal_to_u_direction,
            v_direction_comp_along_u_direction,
            inverse_v_direction_comp_normal_to_u_direction,
            inverse_u_vector_length: F::ONE / u_vector_length,
            inverse_v_vector_length: F::ONE / v_vector_length,
        })
    }
}

impl<F: Float> TextureProjection<F> for PlanarTextureProjection<F> {
    fn identifier(&self) -> String {
        format!("{:?}", self)
    }

    fn project_position(&self, position: &Point3<F>) -> Vector2<F> {
        let displacement = position - self.origin;

        let displacement_along_u_direction = displacement.dot(&self.u_direction);
        let displacement_normal_to_u_direction = displacement.dot(&self.v_normal_to_u_direction);

        let v = displacement_normal_to_u_direction
            * self.inverse_v_direction_comp_normal_to_u_direction;
        let u = displacement_along_u_direction - v * self.v_direction_comp_along_u_direction;

        vector![
            u * self.inverse_u_vector_length,
            v * self.inverse_v_vector_length
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::point;

    #[test]
    fn plane_texture_projection_works() {
        let origin = point![-0.3, 3.9, 12.8];
        let u_vector = vector![-2.1, 4.8, 0.2];
        let v_vector = vector![6.3, -8.1, 5.5];
        let projection = PlanarTextureProjection::new(origin, u_vector, v_vector).unwrap();

        assert_abs_diff_eq!(
            projection.project_position(&origin),
            vector![0.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + u_vector)),
            vector![1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + v_vector)),
            vector![0.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin - u_vector)),
            vector![-1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin - v_vector)),
            vector![0.0, -1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + u_vector * 2.0)),
            vector![2.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + v_vector * 2.0)),
            vector![0.0, 2.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin - u_vector * 2.0)),
            vector![-2.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin - v_vector * 2.0)),
            vector![0.0, -2.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + u_vector + v_vector)),
            vector![1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + u_vector * 3.5 - v_vector * 1.2)),
            vector![3.5, -1.2],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            projection.project_position(&(origin + u_vector * 0.37 + v_vector * 0.44)),
            vector![0.37, 0.44],
            epsilon = 1e-9
        );
    }
}
