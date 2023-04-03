//! [`Component`](impact_ecs::component::Component)s related to texture projections.

use crate::{geometry::PlanarTextureProjection, rendering::fre, scene::PlaneMeshComp};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{point, vector, Point3, Vector3};

/// [`Component`](impact_ecs::component::Component) for entities using a
/// [`PlanarTextureProjection`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PlanarTextureProjectionComp {
    /// The origin of the plane, where the texture coordinates will be zero.
    pub origin: Point3<fre>,
    /// The axis along which the U texture coordinate will increase. The texture
    /// coordinate will be unity at the tip of the vector.
    pub u_vector: Vector3<fre>,
    /// The axis along which the V texture coordinate will increase. The texture
    /// coordinate will be unity at the tip of the vector.
    pub v_vector: Vector3<fre>,
}

impl PlanarTextureProjectionComp {
    /// Creates the component for a projection onto the plane defined by the
    /// given origin and two vectors defining the axes along which the U and V
    /// texture coordinates will increase. The texture coordinates will be zero
    /// at the origin and unity at the tip of the respective u- or v-vector.
    pub fn new(origin: Point3<fre>, u_vector: Vector3<fre>, v_vector: Vector3<fre>) -> Self {
        Self {
            origin,
            u_vector,
            v_vector,
        }
    }

    /// Creates the component for a projection onto the axis-aligned horizontal
    /// plane specified by the given [`PlaneMeshComp`], scaling the projection
    /// so that the texture will repeat the given numbers of times along the U
    /// and V texture coordinate directions. The U-axis will be aligned with the
    /// x-axis and the V-axis will be aligned with the negative z-axis.
    pub fn for_plane(plane: &PlaneMeshComp, n_repeats_u: fre, n_repeats_v: fre) -> Self {
        let origin = point![-0.5, 0.0, 0.5];
        let u_vector = vector![plane.extent_x / n_repeats_u, 0.0, 0.0];
        let v_vector = vector![0.0, 0.0, -plane.extent_z / n_repeats_v];
        Self::new(origin, u_vector, v_vector)
    }

    /// Creates the [`PlanarTextureProjection`] corresponding to this component.
    pub fn create_projection(&self) -> PlanarTextureProjection<fre> {
        PlanarTextureProjection::new(self.origin, self.u_vector, self.v_vector)
    }
}
