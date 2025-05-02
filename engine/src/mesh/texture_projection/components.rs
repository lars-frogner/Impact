//! [`Component`](impact_ecs::component::Component)s related to texture
//! projections.

use crate::{
    mesh::components::RectangleMeshComp, mesh::texture_projection::PlanarTextureProjection,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::SetupComponent;
use nalgebra::{Point3, Vector3, point, vector};
use roc_codegen::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that use a [`PlanarTextureProjection`].
///
/// The purpose of this component is to aid in constructing a
/// [`MeshComp`](crate::mesh::components::MeshComp) for the entity. It is
/// therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct PlanarTextureProjectionComp {
    /// The origin of the plane, where the texture coordinates will be zero.
    pub origin: Point3<f32>,
    /// The axis along which the U texture coordinate will increase. The texture
    /// coordinate will be unity at the tip of the vector.
    pub u_vector: Vector3<f32>,
    /// The axis along which the V texture coordinate will increase. The texture
    /// coordinate will be unity at the tip of the vector.
    pub v_vector: Vector3<f32>,
}

#[roc(dependencies=[RectangleMeshComp])]
impl PlanarTextureProjectionComp {
    /// Creates the component for a projection onto the plane defined by the
    /// given origin and two vectors defining the axes along which the U and V
    /// texture coordinates will increase. The texture coordinates will be zero
    /// at the origin and unity at the tip of the respective u- or v-vector.
    #[roc(body = "{ origin, u_vector, v_vector }")]
    pub fn new(origin: Point3<f32>, u_vector: Vector3<f32>, v_vector: Vector3<f32>) -> Self {
        Self {
            origin,
            u_vector,
            v_vector,
        }
    }

    /// Creates the component for a projection onto the axis-aligned horizontal
    /// rectangle specified by the given [`RectangleMeshComp`], scaling the
    /// projection so that the texture will repeat the given numbers of times
    /// along the U and V texture coordinate directions. The U-axis will be
    /// aligned with the x-axis and the V-axis will be aligned with the negative
    /// z-axis.
    #[roc(body = r#"
    origin = (-0.5, 0.0, 0.5)
    u_vector = (rectangle.extent_x / n_repeats_u, 0.0, 0.0)
    v_vector = (0.0, 0.0, -rectangle.extent_z / n_repeats_v)
    new(origin, u_vector, v_vector)
    "#)]
    pub fn for_rectangle(rectangle: RectangleMeshComp, n_repeats_u: f32, n_repeats_v: f32) -> Self {
        let origin = point![-0.5, 0.0, 0.5];
        let u_vector = vector![rectangle.extent_x / n_repeats_u, 0.0, 0.0];
        let v_vector = vector![0.0, 0.0, -rectangle.extent_z / n_repeats_v];
        Self::new(origin, u_vector, v_vector)
    }

    /// Creates the [`PlanarTextureProjection`] corresponding to this component.
    pub fn create_projection(&self) -> PlanarTextureProjection<f32> {
        PlanarTextureProjection::new(self.origin, self.u_vector, self.v_vector)
    }
}
