//! Mesh setup.

use crate::{
    FrontFaceSide, MeshRepository, TriangleMesh, TriangleMeshID, VertexAttributeSet,
    texture_projection::TextureProjection,
};
use bytemuck::{Pod, Zeroable};
use impact_math::hash64;
use nalgebra::{Point3, Vector3, point, vector};
use roc_integration::roc;
use std::fmt;

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of an axis-aligned horizontal rectangle centered on
    /// the origin, whose front face is on the positive y side.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct RectangleMesh {
        /// The extent of the rectangle in the x-direction.
        pub extent_x: f32,
        /// The extent of the rectangle in the z-direction.
        pub extent_z: f32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of an axis-aligned box centered on the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct BoxMesh {
        /// The extent of the box in the x-direction.
        pub extent_x: f32,
        /// The extent of the box in the y-direction.
        pub extent_y: f32,
        /// The extent of the box in the z-direction.
        pub extent_z: f32,
        front_faces_on_outside: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a vertical cylinder with the bottom centered on the
    /// origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CylinderMesh {
        /// The length of the cylinder.
        pub length: f32,
        /// The diameter of the cylinder.
        pub diameter: f32,
        /// The number of vertices used for representing a circular cross-section of
        /// the cylinder.
        pub n_circumference_vertices: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of an upward-pointing cone with the bottom centered on
    /// the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ConeMesh {
        /// The length of the cone.
        pub length: f32,
        /// The maximum diameter of the cone.
        pub max_diameter: f32,
        /// The number of vertices used for representing a circular cross-section of
        /// the cone.
        pub n_circumference_vertices: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a vertical circular frustum with the bottom
    /// centered on the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CircularFrustumMesh {
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
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a unit diameter sphere centered on the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SphereMesh {
        /// The number of horizontal circular cross-sections of vertices making up
        /// the sphere. The number of vertices comprising each ring is proportional
        /// to `n_rings`, resulting in an approximately uniform resolution.
        pub n_rings: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a unit diameter hemisphere whose disk lies in the
    /// xz-plane and is centered on the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HemisphereMesh {
        /// The number of horizontal circular cross-sections of vertices making up
        /// the hemisphere. The number of vertices comprising each ring is
        /// proportional to `n_rings`, resulting in an approximately uniform
        /// resolution.
        pub n_rings: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// The properties of a
    /// [`PlanarTextureProjection`](crate::texture_projection::PlanarTextureProjection).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct PlanarTextureProjection {
        /// The origin of the plane, where the texture coordinates will be zero.
        pub origin: Point3<f32>,
        /// The axis along which the U texture coordinate will increase. The texture
        /// coordinate will be unity at the tip of the vector.
        pub u_vector: Vector3<f32>,
        /// The axis along which the V texture coordinate will increase. The texture
        /// coordinate will be unity at the tip of the vector.
        pub v_vector: Vector3<f32>,
    }
}

#[roc]
impl RectangleMesh {
    #[roc(expr = "{ extent_x: 1.0, extent_z: 1.0 }")]
    pub const UNIT_SQUARE: Self = Self {
        extent_x: 1.0,
        extent_z: 1.0,
    };

    /// Defines a a rectangle mesh with the given horizontal extents.
    #[roc(body = "{ extent_x, extent_z }")]
    pub fn new(extent_x: f32, extent_z: f32) -> Self {
        Self { extent_x, extent_z }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Rectangle mesh {{ extent_x = {}, extent_z = {}, projection = {} }}",
            self.extent_x, self.extent_z, projection_label
        )))
    }
}

#[roc(dependencies=[FrontFaceSide])]
impl BoxMesh {
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

    /// Defines a box mesh with the given extents.
    #[roc(body = r#"
    front_faces_on_outside =
        when front_face_side is
            Outside -> 1
            Inside -> 0
    {
        extent_x,
        extent_y,
        extent_z,
        front_faces_on_outside,
    }"#)]
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

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
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
impl CylinderMesh {
    /// Defines a cylinder mesh with the given length, diameter and number of
    /// circumeference vertices.
    #[roc(body = "{ length, diameter, n_circumference_vertices }")]
    pub fn new(length: f32, diameter: f32, n_circumference_vertices: u32) -> Self {
        Self {
            length,
            diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Cylinder mesh {{ length = {}, diameter = {}, n_circumference_vertices = {}, projection = {} }}",
            self.length, self.diameter, self.n_circumference_vertices, projection_label
        )))
    }
}

#[roc]
impl ConeMesh {
    /// Defines a cone mesh with the given length, maximum diameter and number
    /// of circumeference vertices.
    #[roc(body = "{ length, max_diameter, n_circumference_vertices }")]
    pub fn new(length: f32, max_diameter: f32, n_circumference_vertices: u32) -> Self {
        Self {
            length,
            max_diameter,
            n_circumference_vertices,
        }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Cone mesh {{ length = {}, max_diameter = {}, n_circumference_vertices = {}, projection = {} }}",
            self.length, self.max_diameter, self.n_circumference_vertices, projection_label
        )))
    }
}

#[roc]
impl CircularFrustumMesh {
    /// Defines a circular frustum mesh with the given length, bottom and top
    /// diameter and number of circumeference vertices.
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

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
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
impl SphereMesh {
    /// Defines a sphere mesh with the given number of rings.
    #[roc(body = "{ n_rings }")]
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Sphere mesh {{ n_rings = {}, projection = {} }}",
            self.n_rings, projection_label
        )))
    }
}

#[roc]
impl HemisphereMesh {
    /// Defines a hemisphere mesh with the given number of rings.
    #[roc(body = "{ n_rings }")]
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Hemisphere mesh {{ n_rings = {}, projection = {} }}",
            self.n_rings, projection_label
        )))
    }
}

#[roc(dependencies=[RectangleMesh])]
impl PlanarTextureProjection {
    /// Creates the properties of a projection onto the plane defined by the
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

    /// Creates the properties of a projection onto the axis-aligned horizontal
    /// rectangle specified by the given [`RectangleMesh`], scaling the
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
    pub fn for_rectangle(rectangle: RectangleMesh, n_repeats_u: f32, n_repeats_v: f32) -> Self {
        let origin = point![-0.5, 0.0, 0.5];
        let u_vector = vector![rectangle.extent_x / n_repeats_u, 0.0, 0.0];
        let v_vector = vector![0.0, 0.0, -rectangle.extent_z / n_repeats_v];
        Self::new(origin, u_vector, v_vector)
    }

    /// Creates the
    /// [`PlanarTextureProjection`](crate::texture_projection::PlanarTextureProjection)
    /// corresponding with these properties.
    ///
    /// # Panics
    /// On error from [`PlanarTextureProjection::new`].
    pub fn create(&self) -> crate::texture_projection::PlanarTextureProjection<f32> {
        crate::texture_projection::PlanarTextureProjection::new(
            self.origin,
            self.u_vector,
            self.v_vector,
        )
        .unwrap()
    }
}

pub fn setup_rectangle_mesh(
    mesh_repository: &mut MeshRepository,
    rectangle_mesh: &RectangleMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = rectangle_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh =
            TriangleMesh::create_rectangle(rectangle_mesh.extent_x, rectangle_mesh.extent_z);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_box_mesh(
    mesh_repository: &mut MeshRepository,
    box_mesh: &BoxMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = box_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_box(
            box_mesh.extent_x,
            box_mesh.extent_y,
            box_mesh.extent_z,
            box_mesh.front_face_side(),
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_cylinder_mesh(
    mesh_repository: &mut MeshRepository,
    cylinder_mesh: &CylinderMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = cylinder_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_cylinder(
            cylinder_mesh.length,
            cylinder_mesh.diameter,
            cylinder_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_cone_mesh(
    mesh_repository: &mut MeshRepository,
    cone_mesh: &ConeMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = cone_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_cone(
            cone_mesh.length,
            cone_mesh.max_diameter,
            cone_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_circular_frustum_mesh(
    mesh_repository: &mut MeshRepository,
    circular_frustum_mesh: &CircularFrustumMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = circular_frustum_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_circular_frustum(
            circular_frustum_mesh.length,
            circular_frustum_mesh.bottom_diameter,
            circular_frustum_mesh.top_diameter,
            circular_frustum_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_sphere_mesh(
    mesh_repository: &mut MeshRepository,
    sphere_mesh: &SphereMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = sphere_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_sphere(sphere_mesh.n_rings as usize);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

pub fn setup_hemisphere_mesh(
    mesh_repository: &mut MeshRepository,
    hemisphere_mesh: &HemisphereMesh,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshID {
    let mesh_id = hemisphere_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_hemisphere(hemisphere_mesh.n_rings as usize);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    mesh_id
}

/// Generates the vertex attributes missing from the giving requirements for the
/// specified mesh, if possible.
pub fn generate_missing_vertex_properties_for_mesh(
    mesh_repository: &mut MeshRepository,
    mesh_id: TriangleMeshID,
    vertex_attribute_requirements: VertexAttributeSet,
) {
    if vertex_attribute_requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
        let Some(mesh) = mesh_repository.get_triangle_mesh(mesh_id) else {
            impact_log::warn!(
                "Tried to generate missing vertex properties for missing mesh {mesh_id}"
            );
            return;
        };

        if !mesh.has_normal_vectors() {
            impact_log::info!("Generating normal vectors for mesh {mesh_id}");

            mesh_repository
                .get_triangle_mesh_mut(mesh_id)
                .unwrap()
                .generate_smooth_normal_vectors();
        }
    }

    if vertex_attribute_requirements.contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION) {
        let Some(mesh) = mesh_repository.get_triangle_mesh(mesh_id) else {
            impact_log::warn!(
                "Tried to generate missing vertex properties for missing mesh {mesh_id}"
            );
            return;
        };

        if !mesh.has_tangent_space_quaternions() {
            impact_log::info!("Generating tangent space quaternions for mesh {mesh_id}");

            mesh_repository
                .get_triangle_mesh_mut(mesh_id)
                .unwrap()
                .generate_smooth_tangent_space_quaternions();
        }
    }
}

fn create_projection_label(projection: Option<&impl TextureProjection<f32>>) -> String {
    projection
        .as_ref()
        .map_or("None".to_string(), |projection| projection.identifier())
}
