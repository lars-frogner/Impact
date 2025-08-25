//! Light sources.

#[macro_use]
mod macros;

pub mod gpu_resource;
pub mod setup;
pub mod shadow_map;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use gpu_resource::LightGPUResources;
use impact_geometry::{
    AxisAlignedBox, CubeMapper, CubemapFace, Frustum, OrientedBox, OrthographicTransform, Sphere,
};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, uniform::UniformBuffer,
    wgpu,
};
use impact_math::{Angle, Degrees, Float, UpperExclusiveBounds};
use nalgebra::{
    self as na, Point3, Scale3, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3,
};
use roc_integration::roc;
use shadow_map::{CascadeIdx, ShadowMappingConfig};
use std::iter;

/// The luminous intensity of a light source, which is the visible power
/// (luminous flux) emitted per unit solid angle, represented as an RGB triplet.
pub type LuminousIntensity = Vector3<f32>;

/// The illuminance of surface, which is the visible power (luminous flux)
/// received per unit area, represented as an RGB triplet.
pub type Illumninance = Vector3<f32>;

/// A luminance, which is the visible power (luminous flux) per unit solid angle
/// and area of light traveling in a given direction, represented as an RGB
/// triplet.
pub type Luminance = Vector3<f32>;

define_component_type! {
    /// A spatially uniform and isotropic (ambient) light field.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct AmbientEmission {
        /// The illuminance (incident flux per area) of a surface due to the ambient
        /// emission.
        ///
        /// # Unit
        /// Lux (lx = lm/m²)
        pub illuminance: Illumninance,
    }
}

define_component_type! {
    /// Uniform emission of light in all directions. The light can not be
    /// shadowed (use [`ShadowableOmnidirectionalEmission`] for light with
    /// shadows).
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct OmnidirectionalEmission {
        /// The luminous intensity of the emitted light.
        ///
        /// # Unit
        /// Candela (cd = lm/sr)
        pub luminous_intensity: LuminousIntensity,
        /// The physical extent of the light source, which determines the extent of
        /// specular highlights.
        ///
        /// # Unit
        /// Meter (m)
        pub source_extent: f32,
    }
}

define_component_type! {
    /// Uniform emission of light in all directions. The light can be shadowed
    /// (use [`OmnidirectionalEmission`] for light without shadows).
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ShadowableOmnidirectionalEmission {
        /// The luminous intensity of the emitted light.
        ///
        /// # Unit
        /// Candela (cd = lm/sr)
        pub luminous_intensity: LuminousIntensity,
        /// The physical extent of the light source, which determines the extent of
        /// specular highlights and the softness of shadows.
        ///
        /// # Unit
        /// Meter (m)
        pub source_extent: f32,
    }
}

define_component_type! {
    /// Emission of light in a single direction. The light can not be shadowed
    /// (use [`ShadowableUnidirectionalEmission`] for light with shadows).
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UnidirectionalEmission {
        /// The illuminance (incident flux per area) of an illuminated surface
        /// perpendicular to the light direction.
        ///
        /// # Unit
        /// Lux (lx = lm/m²)
        pub perpendicular_illuminance: Illumninance,
        /// The direction of the emitted light.
        pub direction: UnitVector3<f32>,
        /// The angular extent of the light source, which determines the extent of
        /// specular highlights.
        pub angular_source_extent: Degrees<f32>,
    }
}

define_component_type! {
    /// Emission of light in a single direction. The light can be shadowed (use
    /// [`UnidirectionalEmission`] for light without shadows).
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ShadowableUnidirectionalEmission {
        /// The illuminance (incident flux per area) of an illuminated surface
        /// perpendicular to the light direction.
        ///
        /// # Unit
        /// Lux (lx = lm/m²)
        pub perpendicular_illuminance: Illumninance,
        /// The direction of the emitted light.
        pub direction: UnitVector3<f32>,
        /// The angular extent of the light source, which determines the extent of
        /// specular highlights and the softness of shadows.
        pub angular_source_extent: Degrees<f32>,
    }
}

define_component_type! {
    /// The ID of an [`AmbientLight`] in the [`LightManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct AmbientLightID(u32);
}

define_component_type! {
    /// The ID of an [`OmnidirectionalLight`] in the [`LightManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct OmnidirectionalLightID(u32);
}

define_component_type! {
    /// The ID of a [`ShadowableOmnidirectionalLight`] in the [`LightManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct ShadowableOmnidirectionalLightID(u32);
}

define_component_type! {
    /// The ID of a [`UnidirectionalLight`] in the [`LightManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct UnidirectionalLightID(u32);
}

define_component_type! {
    /// The ID of a [`ShadowableUnidirectionalLight`] in the [`LightManager`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct ShadowableUnidirectionalLightID(u32);
}

/// A spatially uniform and isotropic light field, represented by an RGB
/// incident luminance that applies to any surface affected by the light.
///
/// This struct is intended to be stored in a [`LightManager`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct AmbientLight {
    luminance: Luminance,
    // Padding to make size multiple of 16-bytes
    _padding: f32,
}

/// An omnidirectional light source represented by a camera space position, an
/// RGB luminous intensity and an extent. The struct also includes a max reach
/// restricting the distance at which the light can illuminate objects.
///
/// This struct is intended to be stored in a [`LightManager`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms, and the fields that
/// will be accessed on the GPU are aligned to 16-byte boundaries.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct OmnidirectionalLight {
    // Camera space position and far distance are treated as a single
    // 4-component vector in the shader
    camera_space_position: Point3<f32>,
    max_reach: f32,
    // Luminous intensity and emissive radius are treated as a single
    // 4-component vector in the shader
    luminous_intensity: LuminousIntensity,
    emissive_radius: f32,
    flags: LightFlags,
    // Padding to make size multiple of 16-bytes
    _padding: [u8; 15],
}

/// A shadowable omnidirectional light source represented by a camera space
/// position, an RGB luminous intensity and an extent. The struct also includes
/// a rotation quaternion that defines the orientation of the light's local
/// coordinate system with respect to camera space, and a near and far distance
/// restricting the distance range in which the light can illuminate objects and
/// cast shadows.
///
/// This struct is intended to be stored in a [`LightManager`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms, and the fields that
/// will be accessed on the GPU are aligned to 16-byte boundaries.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ShadowableOmnidirectionalLight {
    camera_to_light_space_rotation: UnitQuaternion<f32>,
    camera_space_position: Point3<f32>,
    // Padding to obtain 16-byte alignment for next field
    flags: LightFlags, // Use some of the padding for bitflags
    _padding_1: [u8; 3],
    // Luminous intensity and emissive radius are treated as a single
    // 4-component vector in the shader
    luminous_intensity: LuminousIntensity,
    emissive_radius: f32,
    // The `near_distance` and `inverse_distance_span` fields are accessed as a
    // struct in a single field in the shader
    near_distance: f32,
    inverse_distance_span: f32,
    // Padding to make size multiple of 16-bytes
    far_distance: f32,
    max_reach: f32,
}

/// An unidirectional light source represented by a camera space direction, an
/// RGB perpendicular illuminance and an angular extent.
///
/// This struct is intended to be stored in a [`LightManager`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms, and the fields that
/// will be accessed on the GPU are aligned to 16-byte boundaries.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct UnidirectionalLight {
    camera_space_direction: UnitVector3<f32>,
    // Padding to obtain 16-byte alignment for next field
    flags: LightFlags, // Use some of the padding for bitflags
    _padding: [u8; 3],
    // Illuminance and angular radius are treated as a single 4-component vector
    // in the shader
    perpendicular_illuminance: Illumninance,
    tan_angular_radius: f32,
}

/// An unidirectional light source represented by a camera space direction, an
/// RGB perpendicular illuminance and an angular extent. The struct also
/// includes a rotation quaternion that defines the orientation of the light's
/// local coordinate system with respect to camera space, orthographic
/// transformations that map the light's space to clip space in such a way as to
/// include all objects in the scene that may cast shadows inside or into
/// specific cascades (partitions) of the camera view frustum, and the camera
/// linear depths (not the non-linear clip space depths) representing the
/// boundaries between the cascades.
///
/// This struct is intended to be stored in a [`LightManager`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms, and the fields that
/// will be accessed on the GPU are aligned to 16-byte boundaries.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ShadowableUnidirectionalLight {
    camera_to_light_space_rotation: UnitQuaternion<f32>,
    camera_space_direction: UnitVector3<f32>,
    // Padding to obtain 16-byte alignment for next field
    near_partition_depth: f32,
    // Illuminance and angular radius are treated as a single 4-component vector
    // in the shader
    perpendicular_illuminance: Illumninance,
    tan_angular_radius: f32,
    orthographic_transforms: [OrthographicTranslationAndScaling; MAX_SHADOW_MAP_CASCADES_USIZE],
    partition_depths: [f32; MAX_SHADOW_MAP_CASCADES_USIZE - 1],
    // Padding to make size multiple of 16-bytes
    far_partition_depth: f32,
    flags: LightFlags, // Use some of the padding for bitflags
    _padding_3: [u8; 3],
    _padding_4: [f32; 7 - MAX_SHADOW_MAP_CASCADES_USIZE],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct OrthographicTranslationAndScaling {
    translation: Translation3<f32>,
    // Padding to obtain 16-byte alignment for next field
    flags: LightFlags, // Use some of the padding for bitflags for the parent light
    _padding_1: [u8; 3],
    scaling: Scale3<f32>,
    // Padding to make size multiple of 16-bytes
    _padding_2: f32,
}

/// Maximum number of cascades supported in a cascaded shadow map for
/// unidirectional lights.
///
/// # Warning
/// Increasing this above 4 will require changes to the [`UnidirectionalLight`]
/// struct and associated shader code to meet uniform padding requirements.
pub const MAX_SHADOW_MAP_CASCADES: u32 = 4;
const MAX_SHADOW_MAP_CASCADES_USIZE: usize = MAX_SHADOW_MAP_CASCADES as usize;

bitflags! {
    /// Bitflags encoding a set of binary states or properties for a light.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct LightFlags: u8 {
        /// The source emits no light.
        const IS_DISABLED   = 1 << 0;
    }
}

type AmbientLightUniformBuffer = UniformBuffer<AmbientLightID, AmbientLight>;
type OmnidirectionalLightUniformBuffer =
    UniformBuffer<OmnidirectionalLightID, OmnidirectionalLight>;
type ShadowableOmnidirectionalLightUniformBuffer =
    UniformBuffer<ShadowableOmnidirectionalLightID, ShadowableOmnidirectionalLight>;
type UnidirectionalLightUniformBuffer = UniformBuffer<UnidirectionalLightID, UnidirectionalLight>;
type ShadowableUnidirectionalLightUniformBuffer =
    UniformBuffer<ShadowableUnidirectionalLightID, ShadowableUnidirectionalLight>;

/// Manager of all light sources in a scene.
#[derive(Debug)]
pub struct LightManager {
    ambient_light_buffer: AmbientLightUniformBuffer,
    omnidirectional_light_buffer: OmnidirectionalLightUniformBuffer,
    shadowable_omnidirectional_light_buffer: ShadowableOmnidirectionalLightUniformBuffer,
    unidirectional_light_buffer: UnidirectionalLightUniformBuffer,
    shadowable_unidirectional_light_buffer: ShadowableUnidirectionalLightUniformBuffer,
    light_id_counter: u32,
    total_ambient_luminance: Luminance,
}

#[roc]
impl AmbientEmission {
    /// Creates a new ambient light emission component with the given
    /// illuminance (in lux).
    #[roc(body = "{ illuminance }")]
    pub fn new(illuminance: Illumninance) -> Self {
        Self { illuminance }
    }
}

#[roc]
impl OmnidirectionalEmission {
    /// Creates a new omnidirectional emission component with the given
    /// luminous intensity (in candela) and source extent.
    #[roc(body = "{ luminous_intensity, source_extent }")]
    pub fn new(luminous_intensity: LuminousIntensity, source_extent: f32) -> Self {
        Self {
            luminous_intensity,
            source_extent,
        }
    }
}

#[roc]
impl ShadowableOmnidirectionalEmission {
    /// Creates a new shadowable omnidirectional emission component with
    /// the given luminous intensity (in candela) and source extent.
    #[roc(body = "{ luminous_intensity, source_extent }")]
    pub fn new(luminous_intensity: LuminousIntensity, source_extent: f32) -> Self {
        Self {
            luminous_intensity,
            source_extent,
        }
    }
}

#[roc]
impl UnidirectionalEmission {
    /// Creates a new unidirectional emission component with the given
    /// perpendicular illuminance (in lux), direction, and angular
    /// source extent.
    #[roc(body = "{ perpendicular_illuminance, direction, angular_source_extent }")]
    pub fn new(
        perpendicular_illuminance: Illumninance,
        direction: UnitVector3<f32>,
        angular_source_extent: Degrees<f32>,
    ) -> Self {
        Self {
            perpendicular_illuminance,
            direction,
            angular_source_extent,
        }
    }
}

#[roc]
impl ShadowableUnidirectionalEmission {
    /// Creates a new shadowable unidirectional emission component with the
    /// given perpendicular illuminance (in lux), direction, and angular
    /// source extent.
    #[roc(body = "{ perpendicular_illuminance, direction, angular_source_extent }")]
    pub fn new(
        perpendicular_illuminance: Illumninance,
        direction: UnitVector3<f32>,
        angular_source_extent: Degrees<f32>,
    ) -> Self {
        Self {
            perpendicular_illuminance,
            direction,
            angular_source_extent,
        }
    }
}

impl From<AmbientLightID> for u32 {
    fn from(id: AmbientLightID) -> u32 {
        id.0
    }
}

impl std::fmt::Display for AmbientLightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<OmnidirectionalLightID> for u32 {
    fn from(id: OmnidirectionalLightID) -> u32 {
        id.0
    }
}

impl std::fmt::Display for OmnidirectionalLightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ShadowableOmnidirectionalLightID> for u32 {
    fn from(id: ShadowableOmnidirectionalLightID) -> u32 {
        id.0
    }
}

impl std::fmt::Display for ShadowableOmnidirectionalLightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<UnidirectionalLightID> for u32 {
    fn from(id: UnidirectionalLightID) -> u32 {
        id.0
    }
}

impl std::fmt::Display for UnidirectionalLightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ShadowableUnidirectionalLightID> for u32 {
    fn from(id: ShadowableUnidirectionalLightID) -> u32 {
        id.0
    }
}

impl std::fmt::Display for ShadowableUnidirectionalLightID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl LightManager {
    /// By creating light uniform buffers with a small initial capacity, we
    /// avoid excessive buffer reallocation when the first few lights are added.
    pub const INITIAL_LIGHT_CAPACITY: usize = 5;

    /// Creates a new light manager with no lights.
    pub fn new() -> Self {
        Self {
            ambient_light_buffer: AmbientLightUniformBuffer::with_capacity(1),
            omnidirectional_light_buffer: OmnidirectionalLightUniformBuffer::with_capacity(
                Self::INITIAL_LIGHT_CAPACITY,
            ),
            shadowable_omnidirectional_light_buffer:
                ShadowableOmnidirectionalLightUniformBuffer::with_capacity(
                    Self::INITIAL_LIGHT_CAPACITY,
                ),
            unidirectional_light_buffer: UnidirectionalLightUniformBuffer::with_capacity(
                Self::INITIAL_LIGHT_CAPACITY,
            ),
            shadowable_unidirectional_light_buffer:
                ShadowableUnidirectionalLightUniformBuffer::with_capacity(
                    Self::INITIAL_LIGHT_CAPACITY,
                ),
            total_ambient_luminance: Luminance::zeros(),
            light_id_counter: 0,
        }
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`AmbientLight`]s.
    pub fn ambient_light_buffer(&self) -> &AmbientLightUniformBuffer {
        &self.ambient_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`OmnidirectionalLight`]s.
    pub fn omnidirectional_light_buffer(&self) -> &OmnidirectionalLightUniformBuffer {
        &self.omnidirectional_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`ShadowableOmnidirectionalLight`]s.
    pub fn shadowable_omnidirectional_light_buffer(
        &self,
    ) -> &ShadowableOmnidirectionalLightUniformBuffer {
        &self.shadowable_omnidirectional_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`UnidirectionalLight`]s.
    pub fn unidirectional_light_buffer(&self) -> &UnidirectionalLightUniformBuffer {
        &self.unidirectional_light_buffer
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`ShadowableUnidirectionalLight`]s.
    pub fn shadowable_unidirectional_light_buffer(
        &self,
    ) -> &ShadowableUnidirectionalLightUniformBuffer {
        &self.shadowable_unidirectional_light_buffer
    }

    /// Adds the given [`AmbientLight`] to the storage.
    ///
    /// # Returns
    /// A new [`AmbientLightID`] representing the added light source.
    pub fn add_ambient_light(&mut self, ambient_light: AmbientLight) -> AmbientLightID {
        let light_id = AmbientLightID(self.create_new_light_id());
        self.ambient_light_buffer
            .add_uniform(light_id, ambient_light);

        self.total_ambient_luminance += ambient_light.luminance;
        self.update_max_reach_for_omnidirectional_lights();

        light_id
    }

    /// Adds the given [`OmnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`OmnidirectionalLightID`] representing the added light source.
    pub fn add_omnidirectional_light(
        &mut self,
        omnidirectional_light: OmnidirectionalLight,
    ) -> OmnidirectionalLightID {
        let light_id = OmnidirectionalLightID(self.create_new_light_id());
        self.omnidirectional_light_buffer
            .add_uniform(light_id, omnidirectional_light);
        light_id
    }

    /// Adds the given [`ShadowableOmnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`ShadowableOmnidirectionalLightID`] representing the added light
    /// source.
    pub fn add_shadowable_omnidirectional_light(
        &mut self,
        omnidirectional_light: ShadowableOmnidirectionalLight,
    ) -> ShadowableOmnidirectionalLightID {
        let light_id = ShadowableOmnidirectionalLightID(self.create_new_light_id());
        self.shadowable_omnidirectional_light_buffer
            .add_uniform(light_id, omnidirectional_light);
        light_id
    }

    /// Adds the given [`UnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`UnidirectionalLightID`] representing the added light source.
    pub fn add_unidirectional_light(
        &mut self,
        unidirectional_light: UnidirectionalLight,
    ) -> UnidirectionalLightID {
        let light_id = UnidirectionalLightID(self.create_new_light_id());
        self.unidirectional_light_buffer
            .add_uniform(light_id, unidirectional_light);
        light_id
    }

    /// Adds the given [`ShadowableUnidirectionalLight`] to the storage.
    ///
    /// # Returns
    /// A new [`ShadowableUnidirectionalLightID`] representing the added light
    /// source.
    pub fn add_shadowable_unidirectional_light(
        &mut self,
        unidirectional_light: ShadowableUnidirectionalLight,
    ) -> ShadowableUnidirectionalLightID {
        let light_id = ShadowableUnidirectionalLightID(self.create_new_light_id());
        self.shadowable_unidirectional_light_buffer
            .add_uniform(light_id, unidirectional_light);
        light_id
    }

    /// Removes the [`AmbientLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no ambient light with the given ID exists.
    pub fn remove_ambient_light(&mut self, light_id: AmbientLightID) {
        self.total_ambient_luminance -= self.ambient_light_buffer.uniform(light_id).luminance;
        self.ambient_light_buffer.remove_uniform(light_id);
        self.update_max_reach_for_omnidirectional_lights();
    }

    /// Removes the [`OmnidirectionalLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no omnidirectional light with the given ID exists.
    pub fn remove_omnidirectional_light(&mut self, light_id: OmnidirectionalLightID) {
        self.omnidirectional_light_buffer.remove_uniform(light_id);
    }

    /// Removes the [`ShadowableOmnidirectionalLight`] with the given ID from
    /// the storage.
    ///
    /// # Panics
    /// If no shadowable omnidirectional light with the given ID exists.
    pub fn remove_shadowable_omnidirectional_light(
        &mut self,
        light_id: ShadowableOmnidirectionalLightID,
    ) {
        self.shadowable_omnidirectional_light_buffer
            .remove_uniform(light_id);
    }

    /// Removes the [`UnidirectionalLight`] with the given ID from the storage.
    ///
    /// # Panics
    /// If no unidirectional light with the given ID exists.
    pub fn remove_unidirectional_light(&mut self, light_id: UnidirectionalLightID) {
        self.unidirectional_light_buffer.remove_uniform(light_id);
    }

    /// Removes the [`ShadowableUnidirectionalLight`] with the given ID from the
    /// storage.
    ///
    /// # Panics
    /// If no shadowable unidirectional light with the given ID exists.
    pub fn remove_shadowable_unidirectional_light(
        &mut self,
        light_id: ShadowableUnidirectionalLightID,
    ) {
        self.shadowable_unidirectional_light_buffer
            .remove_uniform(light_id);
    }

    /// Sets the uniform illuminance of the [`AmbientLight`] with the given ID
    /// to the given value.
    ///
    /// # Panics
    /// If no ambient light with the given ID exists.
    pub fn set_ambient_light_illuminance(
        &mut self,
        light_id: AmbientLightID,
        illuminance: Illumninance,
    ) {
        let light = self
            .ambient_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing ambient light");

        self.total_ambient_luminance -= light.luminance;
        light.set_illuminance(illuminance);

        self.total_ambient_luminance += light.luminance;
        self.update_max_reach_for_omnidirectional_lights();
    }

    /// Returns a reference to the [`OmnidirectionalLight`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_omnidirectional_light(
        &self,
        light_id: OmnidirectionalLightID,
    ) -> Option<&OmnidirectionalLight> {
        self.omnidirectional_light_buffer.get_uniform(light_id)
    }

    /// Returns a reference to the [`OmnidirectionalLight`] with the given ID.
    ///
    /// # Panics
    /// If no omnidirectional light with the given ID exists.
    pub fn omnidirectional_light(&self, light_id: OmnidirectionalLightID) -> &OmnidirectionalLight {
        self.get_omnidirectional_light(light_id)
            .expect("Requested missing omnidirectional light")
    }

    /// Returns a reference to the [`ShadowableOmnidirectionalLight`] with the
    /// given ID, or [`None`] if it does not exist.
    pub fn get_shadowable_omnidirectional_light(
        &self,
        light_id: ShadowableOmnidirectionalLightID,
    ) -> Option<&ShadowableOmnidirectionalLight> {
        self.shadowable_omnidirectional_light_buffer
            .get_uniform(light_id)
    }

    /// Returns a mutable reference to the [`OmnidirectionalLight`] with the
    /// given ID.
    ///
    /// # Panics
    /// If no omnidirectional light with the given ID exists.
    pub fn omnidirectional_light_mut(
        &mut self,
        light_id: OmnidirectionalLightID,
    ) -> &mut OmnidirectionalLight {
        self.omnidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing omnidirectional light")
    }

    /// Returns a reference to the [`ShadowableOmnidirectionalLight`] with the
    /// given ID.
    ///
    /// # Panics
    /// If no shadowable omnidirectional light with the given ID exists.
    pub fn shadowable_omnidirectional_light(
        &self,
        light_id: ShadowableOmnidirectionalLightID,
    ) -> &ShadowableOmnidirectionalLight {
        self.get_shadowable_omnidirectional_light(light_id)
            .expect("Requested missing shadowable omnidirectional light")
    }

    /// Returns a mutable reference to the [`ShadowableOmnidirectionalLight`]
    /// with the given ID.
    ///
    /// # Panics
    /// If no shadowable omnidirectional light with the given ID exists.
    pub fn shadowable_omnidirectional_light_mut(
        &mut self,
        light_id: ShadowableOmnidirectionalLightID,
    ) -> &mut ShadowableOmnidirectionalLight {
        self.shadowable_omnidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing shadowable omnidirectional light")
    }

    /// Returns a reference to the [`UnidirectionalLight`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_unidirectional_light(
        &self,
        light_id: UnidirectionalLightID,
    ) -> Option<&UnidirectionalLight> {
        self.unidirectional_light_buffer.get_uniform(light_id)
    }

    /// Returns a reference to the [`UnidirectionalLight`] with the given ID.
    ///
    /// # Panics
    /// If no unidirectional light with the given ID exists.
    pub fn unidirectional_light(&self, light_id: UnidirectionalLightID) -> &UnidirectionalLight {
        self.get_unidirectional_light(light_id)
            .expect("Requested missing unidirectional light")
    }

    /// Returns a mutable reference to the [`UnidirectionalLight`] with the
    /// given ID.
    ///
    /// # Panics
    /// If no unidirectional light with the given ID exists.
    pub fn unidirectional_light_mut(
        &mut self,
        light_id: UnidirectionalLightID,
    ) -> &mut UnidirectionalLight {
        self.unidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing unidirectional light")
    }

    /// Returns a reference to the [`ShadowableUnidirectionalLight`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_shadowable_unidirectional_light(
        &self,
        light_id: ShadowableUnidirectionalLightID,
    ) -> Option<&ShadowableUnidirectionalLight> {
        self.shadowable_unidirectional_light_buffer
            .get_uniform(light_id)
    }

    /// Returns a reference to the [`ShadowableUnidirectionalLight`] with the
    /// given ID.
    ///
    /// # Panics
    /// If no shadowable unidirectional light with the given ID exists.
    pub fn shadowable_unidirectional_light(
        &self,
        light_id: ShadowableUnidirectionalLightID,
    ) -> &ShadowableUnidirectionalLight {
        self.get_shadowable_unidirectional_light(light_id)
            .expect("Requested missing shadowable unidirectional light")
    }

    /// Returns a mutable reference to the [`ShadowableUnidirectionalLight`]
    /// with the given ID.
    ///
    /// # Panics
    /// If no shadowable unidirectional light with the given ID exists.
    pub fn shadowable_unidirectional_light_mut(
        &mut self,
        light_id: ShadowableUnidirectionalLightID,
    ) -> &mut ShadowableUnidirectionalLight {
        self.shadowable_unidirectional_light_buffer
            .get_uniform_mut(light_id)
            .expect("Requested missing shadowable unidirectional light")
    }

    /// Returns the slice of all omnidirectional lights in the storage.
    pub fn omnidirectional_lights(&self) -> &[OmnidirectionalLight] {
        self.omnidirectional_light_buffer.valid_uniforms()
    }

    /// Returns the slice of all shadowable omnidirectional lights in the
    /// storage.
    pub fn shadowable_omnidirectional_lights(&self) -> &[ShadowableOmnidirectionalLight] {
        self.shadowable_omnidirectional_light_buffer
            .valid_uniforms()
    }

    /// Returns the slice of all unidirectional lights in the storage.
    pub fn unidirectional_lights(&self) -> &[UnidirectionalLight] {
        self.unidirectional_light_buffer.valid_uniforms()
    }

    /// Returns the slice of all shadowable unidirectional lights in the
    /// storage.
    pub fn shadowable_unidirectional_lights(&self) -> &[ShadowableUnidirectionalLight] {
        self.shadowable_unidirectional_light_buffer.valid_uniforms()
    }

    /// Returns an iterator over the omnidirectional lights in the storage where
    /// each item contains the light ID and a mutable reference to the light.
    pub fn omnidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (OmnidirectionalLightID, &mut OmnidirectionalLight)> {
        self.omnidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the shadowable omnidirectional lights in the
    /// storage where each item contains the light ID and a mutable
    /// reference to the light.
    pub fn shadowable_omnidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<
        Item = (
            ShadowableOmnidirectionalLightID,
            &mut ShadowableOmnidirectionalLight,
        ),
    > {
        self.shadowable_omnidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the unidirectional lights in the storage where
    /// each item contains the light ID and a mutable reference to the light.
    pub fn unidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<Item = (UnidirectionalLightID, &mut UnidirectionalLight)> {
        self.unidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
    }

    /// Returns an iterator over the shadowable unidirectional lights in the
    /// storage where each item contains the light ID and a mutable
    /// reference to the light.
    pub fn shadowable_unidirectional_lights_with_ids_mut(
        &mut self,
    ) -> impl Iterator<
        Item = (
            ShadowableUnidirectionalLightID,
            &mut ShadowableUnidirectionalLight,
        ),
    > {
        self.shadowable_unidirectional_light_buffer
            .valid_uniforms_with_ids_mut()
    }

    /// Removes all lights from the storage.
    pub fn remove_all_lights(&mut self) {
        self.ambient_light_buffer.remove_all_uniforms();
        self.omnidirectional_light_buffer.remove_all_uniforms();
        self.shadowable_omnidirectional_light_buffer
            .remove_all_uniforms();
        self.unidirectional_light_buffer.remove_all_uniforms();
        self.shadowable_unidirectional_light_buffer
            .remove_all_uniforms();
        self.total_ambient_luminance = Luminance::zeros();
    }

    /// Performs any required updates for keeping the given GPU resources in
    /// sync with the current light data.
    pub fn sync_gpu_resources(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        light_gpu_resources: &mut Option<LightGPUResources>,
        shadow_mapping_config: &ShadowMappingConfig,
    ) {
        if let Some(light_gpu_resources) = light_gpu_resources {
            light_gpu_resources.sync_with_light_manager(
                graphics_device,
                staging_belt,
                command_encoder,
                bind_group_layout_registry,
                self,
            );
        } else {
            *light_gpu_resources = Some(LightGPUResources::for_light_manager(
                graphics_device,
                bind_group_layout_registry,
                self,
                shadow_mapping_config,
            ));
        }
    }

    /// Uses the total ambient luminance to compute the maximum reach for all
    /// omnidirectional lights, based on the heuristic that the maximum reach
    /// (where the light contribution should be insignificant) is where the
    /// incident luminance from the light equals some fixed number times the
    /// total ambient luminance.
    fn update_max_reach_for_omnidirectional_lights(&mut self) {
        let total_ambient_luminance =
            compute_scalar_luminance_from_rgb_luminance(&self.total_ambient_luminance);
        let min_incident_luminance = f32::max(
            OmnidirectionalLight::MIN_INCIDENT_LUMINANCE_FLOOR,
            total_ambient_luminance
                * OmnidirectionalLight::MIN_INCIDENT_LUMINANCE_TO_AMBIENT_LUMINANCE_RATIO,
        );
        for light in self.omnidirectional_light_buffer.valid_uniforms_mut() {
            light.update_max_reach_based_on_min_incident_luminance(min_incident_luminance);
        }
        for light in self
            .shadowable_omnidirectional_light_buffer
            .valid_uniforms_mut()
        {
            light.update_max_reach_based_on_min_incident_luminance(min_incident_luminance);
        }
    }

    fn create_new_light_id(&mut self) -> u32 {
        let light_id = self.light_id_counter;
        self.light_id_counter = self.light_id_counter.checked_add(1).unwrap();
        light_id
    }
}

impl Default for LightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientLight {
    pub fn new(luminance: Luminance) -> Self {
        Self {
            luminance,
            _padding: 0.0,
        }
    }

    /// Sets the uniform illuminance due to the light to the given value.
    pub fn set_illuminance(&mut self, illuminance: Illumninance) {
        self.luminance = compute_luminance_for_uniform_illuminance(&illuminance);
    }
}

impl OmnidirectionalLight {
    const MIN_INCIDENT_LUMINANCE_FLOOR: f32 = 0.1;
    const MIN_INCIDENT_LUMINANCE_TO_AMBIENT_LUMINANCE_RATIO: f32 = 0.1;

    pub fn new(
        camera_space_position: Point3<f32>,
        luminous_intensity: LuminousIntensity,
        emissive_extent: f32,
        flags: LightFlags,
    ) -> Self {
        let max_reach = Self::compute_max_reach_from_min_incident_luminance(
            &luminous_intensity,
            Self::MIN_INCIDENT_LUMINANCE_FLOOR,
        );
        Self {
            camera_space_position,
            max_reach,
            luminous_intensity,
            emissive_radius: 0.5 * emissive_extent,
            flags,
            _padding: [0; 15],
        }
    }

    /// Returns the light's flags.
    pub fn flags(&self) -> LightFlags {
        self.flags
    }

    /// Updates the light's flags.
    pub fn set_flags(&mut self, flags: LightFlags) {
        self.flags = flags;
    }

    /// Returns a reference to the camera space position of the light.
    pub fn camera_space_position(&self) -> &Point3<f32> {
        &self.camera_space_position
    }

    /// Returns the distance above which incident luminance from the light is
    /// clamped to zero.
    pub fn max_reach(&self) -> f32 {
        self.max_reach
    }

    /// Sets the camera space position of the light to the given position.
    pub fn set_camera_space_position(&mut self, camera_space_position: Point3<f32>) {
        self.camera_space_position = camera_space_position;
    }

    /// Sets the luminous intensity of the light to the given value. This also
    /// updates the max reach of the light.
    pub fn set_luminous_intensity(&mut self, luminous_intensity: LuminousIntensity) {
        self.max_reach *= Self::compute_max_reach_change_factor_due_to_change_in_luminius_intensity(
            &self.luminous_intensity,
            &luminous_intensity,
        );
        self.luminous_intensity = luminous_intensity;
    }

    /// Sets the emissive extent of the light to the given value.
    pub fn set_emissive_extent(&mut self, emissive_extent: f32) {
        self.emissive_radius = 0.5 * emissive_extent;
    }

    /// Sets `self.max_reach` to the distance at which the incident
    /// luminance from the light equals `min_incident_luminance`.
    fn update_max_reach_based_on_min_incident_luminance(&mut self, min_incident_luminance: f32) {
        self.max_reach = Self::compute_max_reach_from_min_incident_luminance(
            &self.luminous_intensity,
            min_incident_luminance,
        );
    }

    /// Computes the multiplicative factor by which the max reach of a light
    /// changes when going from the old to a new luminous intensity.
    fn compute_max_reach_change_factor_due_to_change_in_luminius_intensity(
        old_luminous_intensity: &LuminousIntensity,
        new_luminous_intensity: &LuminousIntensity,
    ) -> f32 {
        let old_scalar_luminous_intensity =
            compute_scalar_luminance_from_rgb_luminance(old_luminous_intensity);
        let new_scalar_luminous_intensity =
            compute_scalar_luminance_from_rgb_luminance(new_luminous_intensity);
        f32::sqrt(new_scalar_luminous_intensity / old_scalar_luminous_intensity)
    }

    /// Computes the distance at which the incident scalar luminance from an
    /// omnidirectional light with the given luminous intensity equals
    /// `min_incident_luminance`.
    fn compute_max_reach_from_min_incident_luminance(
        luminous_intensity: &LuminousIntensity,
        min_incident_luminance: f32,
    ) -> f32 {
        let scalar_luminuous_intensity =
            compute_scalar_luminance_from_rgb_luminance(luminous_intensity);

        f32::sqrt(scalar_luminuous_intensity / min_incident_luminance)
    }
}

impl ShadowableOmnidirectionalLight {
    const MIN_NEAR_DISTANCE: f32 = 1e-2;

    pub fn new(
        camera_space_position: Point3<f32>,
        luminous_intensity: LuminousIntensity,
        emissive_extent: f32,
        flags: LightFlags,
    ) -> Self {
        let max_reach = OmnidirectionalLight::compute_max_reach_from_min_incident_luminance(
            &luminous_intensity,
            OmnidirectionalLight::MIN_INCIDENT_LUMINANCE_FLOOR,
        );
        Self {
            camera_to_light_space_rotation: UnitQuaternion::identity(),
            camera_space_position,
            flags,
            _padding_1: [0; 3],
            luminous_intensity,
            emissive_radius: 0.5 * emissive_extent,
            near_distance: 0.0,
            inverse_distance_span: 0.0,
            far_distance: 0.0,
            max_reach,
        }
    }

    /// Returns the light's flags.
    pub fn flags(&self) -> LightFlags {
        self.flags
    }

    /// Updates the light's flags.
    pub fn set_flags(&mut self, flags: LightFlags) {
        self.flags = flags;
    }

    /// Takes a transform into camera space and returns the corresponding
    /// transform into the space of the positive z face for points lying in
    /// front of the given face.
    pub fn create_transform_to_positive_z_cubemap_face_space(
        &self,
        face: CubemapFace,
        transform_to_camera_space: &Similarity3<f32>,
    ) -> Similarity3<f32> {
        self.create_transform_from_camera_space_to_positive_z_cubemap_face_space(face)
            * transform_to_camera_space
    }

    /// Computes the transform from camera space into the space of the positive
    /// z face for points lying in front of the given face.
    pub fn create_transform_from_camera_space_to_positive_z_cubemap_face_space(
        &self,
        face: CubemapFace,
    ) -> Similarity3<f32> {
        CubeMapper::rotation_to_positive_z_face_from_face(face)
            * self.create_camera_to_light_space_transform()
    }

    /// Returns a reference to the camera space position of the light.
    pub fn camera_space_position(&self) -> &Point3<f32> {
        &self.camera_space_position
    }

    /// Returns the distance above which incident luminance from the light is
    /// clamped to zero.
    pub fn max_reach(&self) -> f32 {
        self.max_reach
    }

    /// Returns the near plane distance of the shadow cubemap frusta.
    pub fn near_distance(&self) -> f32 {
        self.near_distance
    }

    /// Returns the far plane distance of the shadow cubemap frusta.
    pub fn far_distance(&self) -> f32 {
        self.far_distance
    }

    /// Sets the camera space position of the light to the given position.
    pub fn set_camera_space_position(&mut self, camera_space_position: Point3<f32>) {
        self.camera_space_position = camera_space_position;
    }

    /// Sets the luminous intensity of the light to the given value. This also
    /// updates the max reach of the light.
    pub fn set_luminous_intensity(&mut self, luminous_intensity: LuminousIntensity) {
        self.max_reach *= OmnidirectionalLight::compute_max_reach_change_factor_due_to_change_in_luminius_intensity(
            &self.luminous_intensity,
            &luminous_intensity,
        );
        self.luminous_intensity = luminous_intensity;
    }

    /// Sets the emissive extent of the light to the given value.
    pub fn set_emissive_extent(&mut self, emissive_extent: f32) {
        self.emissive_radius = 0.5 * emissive_extent;
    }

    /// Updates the cubemap orientation and near and far distances to encompass
    /// all shadow casting models without wasting depth resolution or causing
    /// unnecessary draw calls.
    pub fn orient_and_scale_cubemap_for_shadow_casting_models(
        &mut self,
        camera_space_bounding_sphere: &Sphere<f32>,
        camera_space_aabb_for_visible_models: Option<&AxisAlignedBox<f32>>,
    ) {
        let bounding_sphere_center_distance = na::distance(
            &self.camera_space_position,
            camera_space_bounding_sphere.center(),
        );

        let (camera_to_light_space_rotation, far_distance) = if let Some(
            camera_space_aabb_for_visible_models,
        ) =
            camera_space_aabb_for_visible_models
        {
            // Let the orientation of cubemap space be so that the negative
            // z-axis points towards the center of the volume containing visible
            // models
            let camera_to_light_space_rotation =
                Self::compute_camera_to_light_space_rotation(&UnitVector3::new_normalize(
                    camera_space_aabb_for_visible_models.center() - self.camera_space_position,
                ));

            // Use the farthest point of the volume containing visible models as
            // the far distance
            let far_distance = na::distance(
                &camera_space_aabb_for_visible_models
                    .compute_farthest_corner(&self.camera_space_position),
                &self.camera_space_position,
            );

            (camera_to_light_space_rotation, far_distance)
        } else {
            // In this case no models are visible, so the rotation does not
            // matter
            let camera_to_light_space_rotation = UnitQuaternion::identity();

            let far_distance =
                bounding_sphere_center_distance + camera_space_bounding_sphere.radius();

            (camera_to_light_space_rotation, far_distance)
        };

        self.camera_to_light_space_rotation = camera_to_light_space_rotation;

        // The near distance must never be farther than the closest model to the
        // light source
        self.near_distance = (bounding_sphere_center_distance
            - camera_space_bounding_sphere.radius())
        .clamp(Self::MIN_NEAR_DISTANCE, self.max_reach - 1e-9);

        self.far_distance = far_distance.clamp(self.near_distance + 1e-9, self.max_reach);

        self.inverse_distance_span = 1.0 / (self.far_distance - self.near_distance);
    }

    /// Computes the frustum for the given positive z cubemap face in light
    /// space.
    pub fn compute_light_space_frustum_for_positive_z_face(&self) -> Frustum<f32> {
        CubeMapper::compute_frustum_for_positive_z_face(self.near_distance, self.far_distance)
    }

    /// Computes the frustum for the given cubemap face in camera space.
    pub fn compute_camera_space_frustum_for_face(&self, face: CubemapFace) -> Frustum<f32> {
        CubeMapper::compute_transformed_frustum_for_face(
            face,
            &self.create_camera_to_light_space_transform(),
            self.near_distance,
            self.far_distance,
        )
    }

    /// Returns the transform from camera space to the local space of the light.
    pub fn create_camera_to_light_space_transform(&self) -> Similarity3<f32> {
        Similarity3::from_isometry(
            self.camera_to_light_space_rotation * Translation3::from(-self.camera_space_position),
            1.0,
        )
    }

    /// Returns the transform from the local space of the light to camera space.
    pub fn create_light_space_to_camera_transform(&self) -> Similarity3<f32> {
        Similarity3::from_isometry(
            Translation3::from(self.camera_space_position)
                * self.camera_to_light_space_rotation.inverse(),
            1.0,
        )
    }

    /// Whether the given cubemap face frustum may contain any visible models.
    pub fn camera_space_frustum_for_face_may_contain_visible_models(
        camera_space_aabb_for_visible_models: Option<&AxisAlignedBox<f32>>,
        camera_space_face_frustum: &Frustum<f32>,
    ) -> bool {
        if let Some(camera_space_aabb_for_visible_models) = camera_space_aabb_for_visible_models {
            !camera_space_face_frustum
                .compute_aabb()
                .box_lies_outside(camera_space_aabb_for_visible_models)
        } else {
            // In this case no models are visible
            false
        }
    }

    /// Sets `self.max_reach` to the distance at which the incident
    /// luminance from the light equals `min_incident_luminance`.
    fn update_max_reach_based_on_min_incident_luminance(&mut self, min_incident_luminance: f32) {
        self.max_reach = OmnidirectionalLight::compute_max_reach_from_min_incident_luminance(
            &self.luminous_intensity,
            min_incident_luminance,
        );
    }

    fn compute_camera_to_light_space_rotation(
        camera_space_direction: &UnitVector3<f32>,
    ) -> UnitQuaternion<f32> {
        let direction_is_very_close_to_vertical =
            f32::abs(camera_space_direction.y.abs() - 1.0) < 1e-3;

        // We orient the light's local coordinate system so that the light
        // direction in camera space maps to the -z-direction in light space,
        // and the y-direction in camera space maps to the y-direction in light
        // space, unless the light direction is nearly vertical in camera space,
        // in which case we map the -z-direction in camera space to the
        // y-direction in light space
        if direction_is_very_close_to_vertical {
            UnitQuaternion::look_at_rh(camera_space_direction, &-Vector3::z())
        } else {
            UnitQuaternion::look_at_rh(camera_space_direction, &Vector3::y())
        }
    }
}

impl UnidirectionalLight {
    pub fn new(
        camera_space_direction: UnitVector3<f32>,
        illuminance: Illumninance,
        angular_extent: impl Angle<f32>,
        flags: LightFlags,
    ) -> Self {
        Self {
            camera_space_direction,
            flags,
            _padding: [0; 3],
            perpendicular_illuminance: illuminance,
            tan_angular_radius: Self::tan_angular_radius_from_angular_extent(angular_extent),
        }
    }

    /// Returns the light's flags.
    pub fn flags(&self) -> LightFlags {
        self.flags
    }

    /// Updates the light's flags.
    pub fn set_flags(&mut self, flags: LightFlags) {
        self.flags = flags;
    }

    /// Sets the camera space direction of the light to the given direction.
    pub fn set_camera_space_direction(&mut self, camera_space_direction: UnitVector3<f32>) {
        self.camera_space_direction = camera_space_direction;
    }

    /// Sets the perpendicular illuminance of the light to the given value.
    pub fn set_perpendicular_illuminance(&mut self, illuminance: Illumninance) {
        self.perpendicular_illuminance = illuminance;
    }

    /// Sets the angular extent of the light to the given value.
    pub fn set_angular_extent(&mut self, angular_extent: impl Angle<f32>) {
        self.tan_angular_radius = Self::tan_angular_radius_from_angular_extent(angular_extent);
    }

    fn tan_angular_radius_from_angular_extent(angular_extent: impl Angle<f32>) -> f32 {
        f32::tan(0.5 * angular_extent.radians())
    }
}

impl ShadowableUnidirectionalLight {
    pub fn new(
        camera_space_direction: UnitVector3<f32>,
        illuminance: Illumninance,
        angular_extent: impl Angle<f32>,
        flags: LightFlags,
    ) -> Self {
        Self {
            camera_to_light_space_rotation: Self::compute_camera_to_light_space_rotation(
                &camera_space_direction,
            ),
            camera_space_direction,
            near_partition_depth: 0.0,
            perpendicular_illuminance: illuminance,
            tan_angular_radius: UnidirectionalLight::tan_angular_radius_from_angular_extent(
                angular_extent,
            ),
            orthographic_transforms: [OrthographicTranslationAndScaling::zeroed();
                MAX_SHADOW_MAP_CASCADES_USIZE],
            partition_depths: [0.0; MAX_SHADOW_MAP_CASCADES_USIZE - 1],
            far_partition_depth: 0.0,
            flags,
            _padding_3: [0; 3],
            _padding_4: [0.0; 7 - MAX_SHADOW_MAP_CASCADES_USIZE],
        }
    }

    /// Returns the light's flags.
    pub fn flags(&self) -> LightFlags {
        self.flags
    }

    /// Updates the light's flags.
    pub fn set_flags(&mut self, flags: LightFlags) {
        self.flags = flags;
    }

    /// Returns a reference to the quaternion that rotates camera space to light
    /// space.
    pub fn camera_to_light_space_rotation(&self) -> &UnitQuaternion<f32> {
        &self.camera_to_light_space_rotation
    }

    /// Takes a transform into camera space and returns the corresponding
    /// transform into the light's space.
    pub fn create_transform_to_light_space(
        &self,
        transform_to_camera_space: &Similarity3<f32>,
    ) -> Similarity3<f32> {
        self.camera_to_light_space_rotation * transform_to_camera_space
    }

    /// Creates an axis-aligned bounding box in the light's reference frame
    /// containing all models that may cast visible shadows into the given
    /// cascade.
    pub fn create_light_space_orthographic_aabb_for_cascade(
        &self,
        cascade_idx: CascadeIdx,
    ) -> AxisAlignedBox<f32> {
        self.orthographic_transforms[cascade_idx as usize].compute_aabb()
    }

    /// Creates an oriented bounding box in the light's reference frame
    /// containing all models that may cast visible shadows into the given
    /// cascade.
    pub fn create_light_space_orthographic_obb_for_cascade(
        &self,
        cascade_idx: CascadeIdx,
    ) -> OrientedBox<f32> {
        OrientedBox::from_axis_aligned_box(
            &self.create_light_space_orthographic_aabb_for_cascade(cascade_idx),
        )
    }

    /// Returns the array of linear depths (not the non-linear clip space
    /// depths) representing the boundaries between the shadow map cascades.
    pub fn partition_depths(&self) -> &[f32; MAX_SHADOW_MAP_CASCADES_USIZE - 1] {
        &self.partition_depths
    }

    /// Returns linear depth (not the non-linear clip space depth) of the near
    /// plane of the first view frustum partition used in the cascaded shadow
    /// map.
    pub fn near_partition_depth(&self) -> f32 {
        self.near_partition_depth
    }

    /// Returns linear depth (not the non-linear clip space depth) of the far
    /// plane of the last view frustum partition used in the cascaded shadow
    /// map.
    pub fn far_partition_depth(&self) -> f32 {
        self.far_partition_depth
    }

    /// Sets the camera space direction of the light to the given direction.
    pub fn set_camera_space_direction(&mut self, camera_space_direction: UnitVector3<f32>) {
        self.camera_space_direction = camera_space_direction;
        self.camera_to_light_space_rotation =
            Self::compute_camera_to_light_space_rotation(&camera_space_direction);
    }

    /// Sets the perpendicular illuminance of the light to the given value.
    pub fn set_perpendicular_illuminance(&mut self, illuminance: Illumninance) {
        self.perpendicular_illuminance = illuminance;
    }

    /// Sets the angular extent of the light to the given value.
    pub fn set_angular_extent(&mut self, angular_extent: impl Angle<f32>) {
        self.tan_angular_radius =
            UnidirectionalLight::tan_angular_radius_from_angular_extent(angular_extent);
    }

    /// Updates the partition of view frustum cascades for the light based on
    /// the near and far distance required for encompassing visible models.
    pub fn update_cascade_partition_depths(
        &mut self,
        camera_space_view_frustum: &Frustum<f32>,
        camera_space_bounding_sphere: &Sphere<f32>,
    ) {
        const EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT: f32 = 0.5;

        // Find the tightest near and far distance that encompass visible models
        let near_distance = f32::max(
            camera_space_view_frustum.near_distance(),
            -(camera_space_bounding_sphere.center().z + camera_space_bounding_sphere.radius()),
        );

        let far_distance = f32::max(
            near_distance + f32::EPSILON,
            f32::min(
                camera_space_view_frustum.far_distance(),
                -(camera_space_bounding_sphere.center().z - camera_space_bounding_sphere.radius()),
            ),
        );

        // Use a blend between exponential and linear increase in the span of
        // cascades going from the near distance to the far distance

        let distance_ratio =
            (far_distance / near_distance).powf(1.0 / (MAX_SHADOW_MAP_CASCADES as f32));

        let distance_difference = (far_distance - near_distance) / (MAX_SHADOW_MAP_CASCADES as f32);

        let mut exponential_distance = near_distance;
        let mut linear_distance = near_distance;

        for partition_depth in &mut self.partition_depths {
            exponential_distance *= distance_ratio;
            linear_distance += distance_difference;

            let distance = EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT * exponential_distance
                + (1.0 - EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT) * linear_distance;

            *partition_depth =
                camera_space_view_frustum.convert_view_distance_to_linear_depth(distance);
        }

        self.near_partition_depth =
            camera_space_view_frustum.convert_view_distance_to_linear_depth(near_distance);
        self.far_partition_depth =
            camera_space_view_frustum.convert_view_distance_to_linear_depth(far_distance);
    }

    /// Updates the light's orthographic transforms so that all objects in the
    /// scene within or in front of each cascade in the camera view frustum with
    /// respect to the light, i.e. all objects that may cast visible shadows
    /// into each cascade, will be included in the clip space for that cascade.
    pub fn bound_orthographic_transforms_to_cascaded_view_frustum(
        &mut self,
        camera_space_view_frustum: &Frustum<f32>,
        camera_space_bounding_sphere: &Sphere<f32>,
    ) {
        // Rotate to light space, where the light direction is -z
        let light_space_view_frustum =
            camera_space_view_frustum.rotated(&self.camera_to_light_space_rotation);
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        let bounding_sphere_aabb = light_space_bounding_sphere.compute_aabb();

        // For the near plane we use the point on the bounding sphere farthest
        // towards the light source, as models between the light and the view
        // frustum may cast shadows into the frustum
        let near = light_space_bounding_sphere.center().z + light_space_bounding_sphere.radius();

        for (partition_depth_limits, orthographic_transform) in
            (iter::once(&self.near_partition_depth).chain(self.partition_depths.iter()))
                .zip(
                    self.partition_depths
                        .iter()
                        .chain(iter::once(&self.far_partition_depth)),
                )
                .map(|(&lower, &upper)| UpperExclusiveBounds::new(lower, upper))
                .zip(self.orthographic_transforms.iter_mut())
        {
            // Use the bounds of the view frustum in light space along with the
            // bounding sphere to constrain limits for orthographic projection
            let light_space_view_frustum_aabb =
                light_space_view_frustum.compute_aabb_for_subfrustum(partition_depth_limits);

            // Constrain limits using either the view frustum or the bounding
            // volume, depending on which gives the snuggest fit
            let aabb_for_visible_models =
                bounding_sphere_aabb.union_with(&light_space_view_frustum_aabb);

            if let Some(aabb_for_visible_models) = aabb_for_visible_models {
                let visible_models_aabb_lower_corner = aabb_for_visible_models.lower_corner();
                let visible_models_aabb_upper_corner = aabb_for_visible_models.upper_corner();

                let left = visible_models_aabb_lower_corner.x;
                let right = visible_models_aabb_upper_corner.x;

                let bottom = visible_models_aabb_lower_corner.y;
                let top = visible_models_aabb_upper_corner.y;

                // We use lower corner here because smaller (more negative) z is
                // farther away
                let far = visible_models_aabb_lower_corner.z;

                orthographic_transform.set_planes(left, right, bottom, top, near, far);
            }
        }
    }

    /// Determines whether the object with the given camera space bounding
    /// sphere would be included in the clip space for the given cascade,
    /// meaning that it could potentially cast a visible shadow within the
    /// cascade.
    pub fn bounding_sphere_may_cast_visible_shadow_in_cascade(
        &self,
        cascade_idx: CascadeIdx,
        camera_space_bounding_sphere: &Sphere<f32>,
    ) -> bool {
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        let orthographic_aabb = self.create_light_space_orthographic_aabb_for_cascade(cascade_idx);

        !light_space_bounding_sphere.is_outside_axis_aligned_box(&orthographic_aabb)
    }

    fn compute_camera_to_light_space_rotation(
        camera_space_direction: &UnitVector3<f32>,
    ) -> UnitQuaternion<f32> {
        let direction_is_very_close_to_vertical =
            f32::abs(camera_space_direction.y.abs() - 1.0) < 1e-3;

        // We orient the light's local coordinate system so that the light
        // direction in camera space maps to the -z-direction in light space,
        // and the y-direction in camera space maps to the y-direction in light
        // space, unless the light direction is nearly vertical in camera space,
        // in which case we map the -z-direction in camera space to the
        // y-direction in light space
        if direction_is_very_close_to_vertical {
            UnitQuaternion::look_at_rh(camera_space_direction, &-Vector3::z())
        } else {
            UnitQuaternion::look_at_rh(camera_space_direction, &Vector3::y())
        }
    }
}

impl OrthographicTranslationAndScaling {
    fn set_planes(&mut self, left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) {
        (self.translation, self.scaling) =
            OrthographicTransform::compute_orthographic_translation_and_scaling(
                left, right, bottom, top, near, far,
            );

        // Use same scaling in x- and y-direction so that projected shadow map
        // texels are always square
        if self.scaling.x < self.scaling.y {
            self.scaling.y = self.scaling.x;
        } else {
            self.scaling.x = self.scaling.y;
        }
    }

    fn compute_aabb(&self) -> AxisAlignedBox<f32> {
        compute_orthographic_transform_aabb(&self.translation, &self.scaling)
    }
}

/// Computes the isotropic luminance incident on any surface in a light field
/// with the given uniform illuminance.
pub fn compute_luminance_for_uniform_illuminance(illuminance: &Illumninance) -> Luminance {
    illuminance * f32::FRAC_1_PI
}

fn compute_scalar_luminance_from_rgb_luminance(rgb_luminance: &Luminance) -> f32 {
    0.2125 * rgb_luminance.x + 0.7154 * rgb_luminance.y + 0.0721 * rgb_luminance.z
}

fn compute_orthographic_transform_aabb(
    translation: &Translation3<f32>,
    scaling: &Scale3<f32>,
) -> AxisAlignedBox<f32> {
    let (orthographic_center, orthographic_half_extents) =
        OrthographicTransform::compute_center_and_half_extents_from_translation_and_scaling(
            translation,
            scaling,
        );

    let orthographic_lower_corner = orthographic_center - orthographic_half_extents;
    let orthographic_upper_corner = orthographic_center + orthographic_half_extents;

    let orthographic_aabb =
        AxisAlignedBox::new(orthographic_lower_corner, orthographic_upper_corner);

    orthographic_aabb
}
