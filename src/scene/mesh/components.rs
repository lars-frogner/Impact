//! [`Component`](impact_ecs::component::Component)s related to meshes.

use super::MeshID;
use crate::rendering::fre;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use impact_utils::hash64;

/// [`Component`](impact_ecs::component::Component) for entities whose mesh is
/// an axis-aligned horizontal plane centered on the origin, whose front face is
/// on the positive y side.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PlaneMeshComp {
    /// The extent of the plane in the x-direction.
    pub extent_x: fre,
    /// The extent of the plane in the z-direction.
    pub extent_z: fre,
}

/// [`Component`](impact_ecs::component::Component) for entities whose mesh is
/// an axis-aligned box centered on the origin.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct BoxMeshComp {
    /// The extent of the box in the x-direction.
    pub extent_x: fre,
    /// The extent of the box in the y-direction.
    pub extent_y: fre,
    /// The extent of the box in the z-direction.
    pub extent_z: fre,
}

/// [`Component`](impact_ecs::component::Component) for entities whose mesh is a
/// vertical cylinder centered on the origin.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CylinderMeshComp {
    /// The length of the cylinder.
    pub extent_y: fre,
    /// The diameter of the cylinder.
    pub diameter: fre,
    /// The number of vertices used for representing a circular cross-section of
    /// the cylinder.
    pub n_circumference_vertices: u32,
}

/// [`Component`](impact_ecs::component::Component) for entities whose mesh is a
/// unit diameter sphere centered on the origin.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SphereMeshComp {
    /// The number of horizontal circular cross-sections of vertices making up
    /// the sphere. The number of vertices comprising each ring is proportional
    /// to `n_rings`, resulting in an approximately uniform resolution.
    pub n_rings: u32,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// [`TriangleMesh`](crate::geometry::TriangleMesh).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MeshComp {
    /// The ID of the entity's [`TriangleMesh`](crate::geometry::TriangleMesh).
    pub id: MeshID,
}

impl MeshComp {
    /// Creates a new component representing a
    /// [`TriangleMesh`](crate::geometry::TriangleMesh) with the given ID.
    pub fn new(mesh_id: MeshID) -> Self {
        Self { id: mesh_id }
    }
}

impl PlaneMeshComp {
    pub const UNIT_PLANE: Self = Self {
        extent_x: 1.0,
        extent_z: 1.0,
    };

    /// Creates a new component for a plane mesh with the given horizontal extents.
    pub fn new(extent_x: fre, extent_z: fre) -> Self {
        Self { extent_x, extent_z }
    }

    /// Generates a [`MeshID`] for the mesh of this component.
    pub fn generate_id(&self) -> MeshID {
        MeshID(hash64!(format!(
            "Plane mesh {{ extent_x = {}, extent_z = {} }}",
            self.extent_x, self.extent_z
        )))
    }
}

impl BoxMeshComp {
    pub const UNIT_CUBE: Self = Self {
        extent_x: 1.0,
        extent_y: 1.0,
        extent_z: 1.0,
    };

    /// Creates a new component for a box mesh with the given extents.
    pub fn new(extent_x: fre, extent_y: fre, extent_z: fre) -> Self {
        Self {
            extent_x,
            extent_y,
            extent_z,
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component.
    pub fn generate_id(&self) -> MeshID {
        MeshID(hash64!(format!(
            "Box mesh {{ extent_x = {}, extent_y = {}, extent_z = {} }}",
            self.extent_x, self.extent_y, self.extent_z
        )))
    }
}

impl CylinderMeshComp {
    /// Creates a new component for a box mesh with the given y-extent, diameter
    /// and number of circumeference vertices.
    pub fn new(extent_y: fre, diameter: fre, n_circumference_vertices: u32) -> Self {
        Self {
            extent_y,
            diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component.
    pub fn generate_id(&self) -> MeshID {
        MeshID(hash64!(format!(
            "Cylinder mesh {{ extent_y = {}, diameter = {}, n_circumference_vertices = {} }}",
            self.extent_y, self.diameter, self.n_circumference_vertices
        )))
    }
}

impl SphereMeshComp {
    /// Creates a new component for a sphere mesh with the given number of
    /// rings.
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`MeshID`] for the mesh of this component.
    pub fn generate_id(&self) -> MeshID {
        MeshID(hash64!(format!(
            "Sphere mesh {{ n_rings = {} }}",
            self.n_rings
        )))
    }
}
