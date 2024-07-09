//! [`Component`](impact_ecs::component::Component)s related to texture projections.

use crate::{
    component::ComponentRegistry, gpu::rendering::fre, mesh::components::RectangleMeshComp,
    mesh::texture_projection::PlanarTextureProjection,
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{point, vector, Point3, Vector3};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that use a [`PlanarTextureProjection`].
///
/// The purpose of this component is to aid in constructing a
/// [`MeshComp`](crate::mesh::components::MeshComp) for the entity. It is
/// therefore not kept after entity creation.
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
    /// rectangle specified by the given [`RectangleMeshComp`], scaling the
    /// projection so that the texture will repeat the given numbers of times
    /// along the U and V texture coordinate directions. The U-axis will be
    /// aligned with the x-axis and the V-axis will be aligned with the negative
    /// z-axis.
    pub fn for_rectangle(
        rectangle: &RectangleMeshComp,
        n_repeats_u: fre,
        n_repeats_v: fre,
    ) -> Self {
        let origin = point![-0.5, 0.0, 0.5];
        let u_vector = vector![rectangle.extent_x / n_repeats_u, 0.0, 0.0];
        let v_vector = vector![0.0, 0.0, -rectangle.extent_z / n_repeats_v];
        Self::new(origin, u_vector, v_vector)
    }

    /// Creates the [`PlanarTextureProjection`] corresponding to this component.
    pub fn create_projection(&self) -> PlanarTextureProjection<fre> {
        PlanarTextureProjection::new(self.origin, self.u_vector, self.v_vector)
    }
}

/// Registers all texture projection
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_texture_projection_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, PlanarTextureProjectionComp)
}
