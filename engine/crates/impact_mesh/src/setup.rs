//! Mesh setup.

use crate::{
    FrontFaceSide, TriangleMesh, TriangleMeshDirtyMask, TriangleMeshID, TriangleMeshRegistry,
    VertexAttributeSet, texture_projection::TextureProjection,
};
use bytemuck::{Pod, Zeroable};
use impact_math::{hash64, point::Point3C, vector::Vector3C};
use roc_integration::roc;
use std::fmt;

/// Template specifying how to generate a [`TriangleMesh`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum TriangleMeshTemplate {
    Rectangle(RectangleMesh),
    Box(BoxMesh),
    Cylinder(CylinderMesh),
    Cone(ConeMesh),
    CircularFrustum(CircularFrustumMesh),
    Sphere(SphereMesh),
    Hemisphere(HemisphereMesh),
    Capsule(CapsuleMesh),
    ScreenFillingQuad,
    SphericalLightVolume(SphericalLightVolumeMesh),
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of an axis-aligned horizontal rectangle centered on
    /// the origin, whose front face is on the positive y side.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct BoxMesh {
        /// The extent of the box in the x-direction.
        pub extent_x: f32,
        /// The extent of the box in the y-direction.
        pub extent_y: f32,
        /// The extent of the box in the z-direction.
        pub extent_z: f32,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_u32_as_bool", deserialize_with = "deserialize_bool_as_u32"))]
        front_faces_on_outside: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a vertical cylinder with the bottom centered on the
    /// origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// A mesh consisting of a vertical capsule centered on the origin.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CapsuleMesh {
        /// The distance between the centers of the cap spheres.
        pub segment_length: f32,
        /// The radius of the spherical caps.
        pub radius: f32,
        /// The number of vertices used for representing a circular cross-section of
        /// the capsule cylinder.
        pub n_circumference_vertices: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of two triangles that exactly fill the screen in clip space.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ScreenFillingQuadMesh;
}

define_setup_type! {
    target = TriangleMeshID;
    /// A mesh consisting of a sphere with inward-facing triangles, suitable for light volumes.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SphericalLightVolumeMesh {
        /// The number of horizontal circular cross-sections of vertices making up
        /// the sphere. The number of vertices comprising each ring is proportional
        /// to `n_rings`, resulting in an approximately uniform resolution.
        pub n_rings: u32,
    }
}

define_setup_type! {
    target = TriangleMeshID;
    /// The properties of a
    /// [`PlanarTextureProjection`](crate::texture_projection::PlanarTextureProjection).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct PlanarTextureProjection {
        /// The origin of the plane, where the texture coordinates will be zero.
        pub origin: Point3C,
        /// The axis along which the U texture coordinate will increase. The texture
        /// coordinate will be unity at the tip of the vector.
        pub u_vector: Vector3C,
        /// The axis along which the V texture coordinate will increase. The texture
        /// coordinate will be unity at the tip of the vector.
        pub v_vector: Vector3C,
    }
}

impl TriangleMeshTemplate {
    /// Generates the [`TriangleMesh`] corresponding to this template.
    pub fn generate_mesh(&self) -> TriangleMesh {
        match self {
            Self::Rectangle(rectangle_mesh) => {
                TriangleMesh::create_rectangle(rectangle_mesh.extent_x, rectangle_mesh.extent_z)
            }
            Self::Box(box_mesh) => TriangleMesh::create_box(
                box_mesh.extent_x,
                box_mesh.extent_y,
                box_mesh.extent_z,
                box_mesh.front_face_side(),
            ),
            Self::Cylinder(cylinder_mesh) => TriangleMesh::create_cylinder(
                cylinder_mesh.length,
                cylinder_mesh.diameter,
                cylinder_mesh.n_circumference_vertices as usize,
            ),
            Self::Cone(cone_mesh) => TriangleMesh::create_cone(
                cone_mesh.length,
                cone_mesh.max_diameter,
                cone_mesh.n_circumference_vertices as usize,
            ),
            Self::CircularFrustum(circular_frustum_mesh) => TriangleMesh::create_circular_frustum(
                circular_frustum_mesh.length,
                circular_frustum_mesh.bottom_diameter,
                circular_frustum_mesh.top_diameter,
                circular_frustum_mesh.n_circumference_vertices as usize,
            ),
            Self::Sphere(sphere_mesh) => TriangleMesh::create_sphere(sphere_mesh.n_rings as usize),
            Self::Hemisphere(hemisphere_mesh) => {
                TriangleMesh::create_hemisphere(hemisphere_mesh.n_rings as usize)
            }
            Self::Capsule(capsule_mesh) => TriangleMesh::create_capsule(
                capsule_mesh.segment_length,
                capsule_mesh.radius,
                capsule_mesh.n_circumference_vertices as usize,
            ),
            Self::ScreenFillingQuad => TriangleMesh::create_screen_filling_quad(),
            Self::SphericalLightVolume(spherical_light_volume_mesh) => {
                TriangleMesh::create_spherical_light_volume(
                    spherical_light_volume_mesh.n_rings as usize,
                )
            }
        }
    }

    /// Generates a [`TriangleMeshID`] for this template's mesh, using the given
    /// label to describe the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        match self {
            Self::Rectangle(mesh) => mesh.generate_id(projection_label),
            Self::Box(mesh) => mesh.generate_id(projection_label),
            Self::Cylinder(mesh) => mesh.generate_id(projection_label),
            Self::Cone(mesh) => mesh.generate_id(projection_label),
            Self::CircularFrustum(mesh) => mesh.generate_id(projection_label),
            Self::Sphere(mesh) => mesh.generate_id(projection_label),
            Self::Hemisphere(mesh) => mesh.generate_id(projection_label),
            Self::Capsule(mesh) => mesh.generate_id(projection_label),
            Self::ScreenFillingQuad => ScreenFillingQuadMesh.generate_id(projection_label),
            Self::SphericalLightVolume(mesh) => mesh.generate_id(projection_label),
        }
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

#[roc]
impl CapsuleMesh {
    /// Defines a capsule mesh with the given segment length, radius and number
    /// of circumeference vertices.
    #[roc(body = "{ segment_length, radius, n_circumference_vertices }")]
    pub fn new(segment_length: f32, radius: f32, n_circumference_vertices: u32) -> Self {
        Self {
            segment_length,
            radius,
            n_circumference_vertices,
        }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Capsule mesh {{ segment_length = {}, radius = {}, n_circumference_vertices = {}, projection = {} }}",
            self.segment_length, self.radius, self.n_circumference_vertices, projection_label
        )))
    }
}

#[roc]
impl ScreenFillingQuadMesh {
    /// Creates a new screen-filling quad mesh.
    #[roc(body = "{}")]
    pub fn new() -> Self {
        Self
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to describe
    /// the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Screen filling quad mesh {{ projection = {} }}",
            projection_label
        )))
    }
}

impl Default for ScreenFillingQuadMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[roc]
impl SphericalLightVolumeMesh {
    /// Defines a spherical light volume mesh with the given number of rings.
    #[roc(body = "{ n_rings }")]
    pub fn new(n_rings: u32) -> Self {
        Self { n_rings }
    }

    /// Generates a [`TriangleMeshID`] for this mesh, using the given label to
    /// describe the texture projection.
    pub fn generate_id(&self, projection_label: impl fmt::Display) -> TriangleMeshID {
        TriangleMeshID(hash64!(format!(
            "Spherical light volume mesh {{ n_rings = {}, projection = {} }}",
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
    pub fn new(origin: Point3C, u_vector: Vector3C, v_vector: Vector3C) -> Self {
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
        let origin = Point3C::new(-0.5, 0.0, 0.5);
        let u_vector = Vector3C::new(rectangle.extent_x / n_repeats_u, 0.0, 0.0);
        let v_vector = Vector3C::new(0.0, 0.0, -rectangle.extent_z / n_repeats_v);
        Self::new(origin, u_vector, v_vector)
    }

    /// Creates the
    /// [`PlanarTextureProjection`](crate::texture_projection::PlanarTextureProjection)
    /// corresponding with these properties.
    ///
    /// # Panics
    /// On error from [`PlanarTextureProjection::new`].
    pub fn create(&self) -> crate::texture_projection::PlanarTextureProjection {
        crate::texture_projection::PlanarTextureProjection::new(
            self.origin.aligned(),
            self.u_vector.aligned(),
            self.v_vector.aligned(),
        )
        .unwrap()
    }
}

pub fn setup_triangle_mesh_from_template(
    registry: &mut TriangleMeshRegistry,
    template: &TriangleMeshTemplate,
    mesh_id: Option<TriangleMeshID>,
    projection: Option<&impl TextureProjection>,
) -> TriangleMeshID {
    let mesh_id =
        mesh_id.unwrap_or_else(|| template.generate_id(create_projection_label(projection)));

    if !registry.contains(mesh_id) {
        let mut mesh = template.generate_mesh();

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection, &mut TriangleMeshDirtyMask::empty());
        }

        registry.insert(mesh_id, mesh);
    }
    mesh_id
}

/// Generates the vertex attributes missing from the giving requirements for the
/// specified mesh, if possible.
pub fn generate_missing_vertex_properties_for_mesh(
    registry: &mut TriangleMeshRegistry,
    mesh_id: TriangleMeshID,
    vertex_attribute_requirements: VertexAttributeSet,
) {
    if !vertex_attribute_requirements.intersects(
        VertexAttributeSet::NORMAL_VECTOR | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
    ) {
        return;
    }

    let Some(mut mesh) = registry.get_mut(mesh_id) else {
        log::warn!("Tried to generate missing vertex properties for missing mesh: {mesh_id}");
        return;
    };

    let mut dirty_mask = TriangleMeshDirtyMask::empty();

    if vertex_attribute_requirements.contains(VertexAttributeSet::NORMAL_VECTOR)
        && !mesh.has_normal_vectors()
    {
        log::info!("Generating normal vectors for mesh: {mesh_id}");
        mesh.generate_smooth_normal_vectors(&mut dirty_mask);
    }

    if vertex_attribute_requirements.contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION)
        && !mesh.has_tangent_space_quaternions()
    {
        log::info!("Generating tangent space quaternions for mesh: {mesh_id}");
        mesh.generate_smooth_tangent_space_quaternions(&mut dirty_mask);
    }

    mesh.set_dirty_mask(dirty_mask);
}

fn create_projection_label(projection: Option<&impl TextureProjection>) -> String {
    projection
        .as_ref()
        .map_or("None".to_string(), |projection| projection.identifier())
}

#[cfg(feature = "serde")]
fn serialize_u32_as_bool<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bool(*value != 0)
}

#[cfg(feature = "serde")]
fn deserialize_bool_as_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let value = bool::deserialize(deserializer)?;
    Ok(if value { 1 } else { 0 })
}
