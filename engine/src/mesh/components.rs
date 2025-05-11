//! [`Component`](impact_ecs::component::Component)s related to meshes.

use crate::mesh::{FrontFaceSide, MeshID};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use impact_math::hash64;
use roc_codegen::roc;
use std::fmt::Display;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is an axis-aligned horizontal rectangle centered on the
/// origin, whose front face is on the positive y side.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "RectangleMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct RectangleMeshComp {
    /// The extent of the rectangle in the x-direction.
    pub extent_x: f32,
    /// The extent of the rectangle in the z-direction.
    pub extent_z: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is an axis-aligned box centered on the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "BoxMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct BoxMeshComp {
    /// The extent of the box in the x-direction.
    pub extent_x: f32,
    /// The extent of the box in the y-direction.
    pub extent_y: f32,
    /// The extent of the box in the z-direction.
    pub extent_z: f32,
    front_faces_on_outside: u32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is a vertical cylinder with the bottom centered on
/// the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "CylinderMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct CylinderMeshComp {
    /// The length of the cylinder.
    pub length: f32,
    /// The diameter of the cylinder.
    pub diameter: f32,
    /// The number of vertices used for representing a circular cross-section of
    /// the cylinder.
    pub n_circumference_vertices: u32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is an upward-pointing cone with the bottom centered on
/// the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "ConeMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct ConeMeshComp {
    /// The length of the cone.
    pub length: f32,
    /// The maximum diameter of the cone.
    pub max_diameter: f32,
    /// The number of vertices used for representing a circular cross-section of
    /// the cone.
    pub n_circumference_vertices: u32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is a vertical circular frustum with the bottom centered
/// on the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "CircularFrustumMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct CircularFrustumMeshComp {
    /// The length of the frustum.
    pub length: f32,
    /// The bottom diameter of the frustum.
    pub bottom_diameter: f32,
    /// The top diameter of the frustum.
    pub top_diameter: f32,
    /// The number of vertices used for representing a circular cross-section of
    /// the frustum.
    pub n_circumference_vertices: u32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is a unit diameter sphere centered on the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "SphereMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct SphereMeshComp {
    /// The number of horizontal circular cross-sections of vertices making up
    /// the sphere. The number of vertices comprising each ring is proportional
    /// to `n_rings`, resulting in an approximately uniform resolution.
    pub n_rings: u32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose mesh is a unit diameter hemisphere whose disk lies in the
/// xz-plane and is centered on the origin.
///
/// The purpose of this component is to aid in constructing a [`MeshComp`] for
/// the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "HemisphereMesh")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct HemisphereMeshComp {
    /// The number of horizontal circular cross-sections of vertices making up
    /// the hemisphere. The number of vertices comprising each ring is
    /// proportional to `n_rings`, resulting in an approximately uniform
    /// resolution.
    pub n_rings: u32,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// [`TriangleMesh`](crate::mesh::TriangleMesh).
#[roc(parents = "Comp", name = "Mesh")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MeshComp {
    /// The ID of the entity's [`TriangleMesh`](crate::mesh::TriangleMesh).
    pub id: MeshID,
}

#[roc]
impl MeshComp {
    /// Creates a new component representing a
    /// [`TriangleMesh`](crate::mesh::TriangleMesh) with the given ID.
    #[roc(body = "{ id: mesh_id }")]
    pub fn new(mesh_id: MeshID) -> Self {
        Self { id: mesh_id }
    }
}

#[roc]
impl RectangleMeshComp {
    #[roc(expr = "{ extent_x: 1.0, extent_z: 1.0 }")]
    pub const UNIT_SQUARE: Self = Self {
        extent_x: 1.0,
        extent_z: 1.0,
    };

    /// Creates a new component for a rectangle mesh with the given horizontal
    /// extents.
    #[roc(body = "{ extent_x, extent_z }")]
    pub fn new(extent_x: f32, extent_z: f32) -> Self {
        Self { extent_x, extent_z }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Rectangle mesh {{ extent_x = {}, extent_z = {}, projection = {} }}",
            self.extent_x, self.extent_z, projection_label
        )))
    }
}

#[roc]
impl BoxMeshComp {
    #[roc(expr = "{ extent_x: 1.0, extent_y: 1.0, extent_z: 1.0, front_faces_on_outside: 1 }")]
    pub const UNIT_CUBE: Self = Self {
        extent_x: 1.0,
        extent_y: 1.0,
        extent_z: 1.0,
        front_faces_on_outside: 1,
    };

    #[roc(expr = "{ extent_x: 1.0, extent_y: 1.0, extent_z: 1.0, front_faces_on_outside: 0 }")]
    pub const SKYBOX: Self = Self {
        extent_x: 1.0,
        extent_y: 1.0,
        extent_z: 1.0,
        front_faces_on_outside: 0,
    };

    /// Creates a new component for a box mesh with the given extents.
    pub fn new(
        extent_x: f32,
        extent_y: f32,
        extent_z: f32,
        front_face_side: FrontFaceSide,
    ) -> Self {
        Self {
            extent_x,
            extent_y,
            extent_z,
            front_faces_on_outside: match front_face_side {
                FrontFaceSide::Outside => 1,
                FrontFaceSide::Inside => 0,
            },
        }
    }

    /// Returns the [`FrontFaceSide`] for the box mesh.
    pub fn front_face_side(&self) -> FrontFaceSide {
        match self.front_faces_on_outside {
            1 => FrontFaceSide::Outside,
            0 => FrontFaceSide::Inside,
            _ => unreachable!(),
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Box mesh {{ extent_x = {}, extent_y = {}, extent_z = {}, front_faces_on_outside = {}, projection = {} }}",
            self.extent_x,
            self.extent_y,
            self.extent_z,
            self.front_faces_on_outside,
            projection_label
        )))
    }
}

#[roc]
impl CylinderMeshComp {
    /// Creates a new component for a cylinder mesh with the given length,
    /// diameter and number of circumeference vertices.
    #[roc(body = "{ length, diameter, n_circumference_vertices }")]
    pub fn new(length: f32, diameter: f32, n_circumference_vertices: u32) -> Self {
        Self {
            length,
            diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Cylinder mesh {{ length = {}, diameter = {}, n_circumference_vertices = {}, projection = {} }}",
            self.length, self.diameter, self.n_circumference_vertices, projection_label
        )))
    }
}

#[roc]
impl ConeMeshComp {
    /// Creates a new component for a cone mesh with the given length, maximum
    /// diameter and number of circumeference vertices.
    #[roc(body = "{ length, max_diameter, n_circumference_vertices }")]
    pub fn new(length: f32, max_diameter: f32, n_circumference_vertices: u32) -> Self {
        Self {
            length,
            max_diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Cone mesh {{ length = {}, max_diameter = {}, n_circumference_vertices = {}, projection = {} }}",
            self.length, self.max_diameter, self.n_circumference_vertices, projection_label
        )))
    }
}

#[roc]
impl CircularFrustumMeshComp {
    /// Creates a new component for a circular frustum mesh with the given
    /// length, bottom and top diameter and number of circumeference vertices.
    #[roc(body = r#"
    {
        length,
        bottom_diameter,
        top_diameter,
        n_circumference_vertices,
    }"#)]
    pub fn new(
        length: f32,
        bottom_diameter: f32,
        top_diameter: f32,
        n_circumference_vertices: u32,
    ) -> Self {
        Self {
            length,
            bottom_diameter,
            top_diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Circular frustum mesh {{ length = {}, bottom_diameter = {}, top_diameter = {}, n_circumference_vertices = {}, projection = {} }}",
            self.length,
            self.bottom_diameter,
            self.top_diameter,
            self.n_circumference_vertices,
            projection_label
        )))
    }
}

#[roc]
impl SphereMeshComp {
    /// Creates a new component for a sphere mesh with the given number of
    /// rings.
    #[roc(body = "{ n_rings }")]
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Sphere mesh {{ n_rings = {}, projection = {} }}",
            self.n_rings, projection_label
        )))
    }
}

#[roc]
impl HemisphereMeshComp {
    /// Creates a new component for a hemisphere mesh with the given number of
    /// rings.
    #[roc(body = "{ n_rings }")]
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`MeshID`] for the mesh of this component, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl Display) -> MeshID {
        MeshID(hash64!(format!(
            "Hemisphere mesh {{ n_rings = {}, projection = {} }}",
            self.n_rings, projection_label
        )))
    }
}
