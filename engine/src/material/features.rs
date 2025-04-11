//! Instance features representing material properties.

use crate::{
    impl_InstanceFeature,
    material::{
        RGBColor,
        components::{ParallaxMapComp, UniformColorComp},
    },
    model::{InstanceFeature, InstanceFeatureID, InstanceFeatureManager, InstanceFeatureTypeID},
};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector2;

const MATERIAL_VERTEX_BINDING_START: u32 = 20;

/// Vertex attribute location of a specific type of material instance feature.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MaterialInstanceFeatureLocation {
    SpecularReflectance = MATERIAL_VERTEX_BINDING_START,
    Roughness = (MATERIAL_VERTEX_BINDING_START + 1),
    Metalness = (MATERIAL_VERTEX_BINDING_START + 2),
    EmissiveLuminance = (MATERIAL_VERTEX_BINDING_START + 3),
    Color = (MATERIAL_VERTEX_BINDING_START + 4),
    ParallaxDisplacementScale = (MATERIAL_VERTEX_BINDING_START + 5),
    ParallaxUVPerDistance = (MATERIAL_VERTEX_BINDING_START + 6),
}

bitflags! {
    /// Bitflags encoding information related to a material's per-instance
    /// features.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct MaterialInstanceFeatureFlags: u8 {
        const HAS_COLOR = 1 << 0;
        const USES_PARALLAX_MAPPING = 1 << 1;
    }
}

/// Fixed material information for a non-physical material with a uniform color
/// that is independent of lighting.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct FixedColorMaterialFeature {
    color: RGBColor,
}

/// Fixed material information for a physical material with a uniform
/// base color.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorPhysicalMaterialFeature {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    color: RGBColor,
}

/// Fixed material information for a physical material with a textured
/// base color.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorPhysicalMaterialFeature {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
}

/// Fixed material information for a physical material with a uniform
/// base color and parallax mapping.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorParallaxMappedPhysicalMaterialFeature {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    color: RGBColor,
    parallax_displacement_scale: f32,
    parallax_uv_per_distance: Vector2<f32>,
}

/// Fixed material information for a physical material with a textured
/// base color and parallax mapping.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorParallaxMappedPhysicalMaterialFeature {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    parallax_displacement_scale: f32,
    parallax_uv_per_distance: Vector2<f32>,
}

impl FixedColorMaterialFeature {
    pub fn new(color: RGBColor) -> Self {
        Self { color }
    }
}

pub fn register_material_feature_types(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.register_feature_type::<FixedColorMaterialFeature>();
    instance_feature_manager.register_feature_type::<UniformColorPhysicalMaterialFeature>();
    instance_feature_manager.register_feature_type::<TexturedColorPhysicalMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformColorParallaxMappedPhysicalMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<TexturedColorParallaxMappedPhysicalMaterialFeature>();
}

/// Creates the appropriate physical material feature for the given set of
/// components and properties and adds it to the instance feature manager.
///
/// # Returns
/// The ID of the created feature type and the ID of the created feature.
pub fn create_physical_material_feature(
    instance_feature_manager: &mut InstanceFeatureManager,
    uniform_color: Option<&UniformColorComp>,
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    parallax_map: Option<&ParallaxMapComp>,
) -> (
    InstanceFeatureTypeID,
    InstanceFeatureID,
    MaterialInstanceFeatureFlags,
) {
    match (uniform_color, parallax_map) {
        (Some(color), None) => (
            UniformColorPhysicalMaterialFeature::FEATURE_TYPE_ID,
            UniformColorPhysicalMaterialFeature::add_feature(
                instance_feature_manager,
                color,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
            ),
            MaterialInstanceFeatureFlags::HAS_COLOR,
        ),
        (None, None) => (
            TexturedColorPhysicalMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorPhysicalMaterialFeature::add_feature(
                instance_feature_manager,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
            ),
            MaterialInstanceFeatureFlags::empty(),
        ),
        (Some(color), Some(parallax_map)) => (
            UniformColorParallaxMappedPhysicalMaterialFeature::FEATURE_TYPE_ID,
            UniformColorParallaxMappedPhysicalMaterialFeature::add_feature(
                instance_feature_manager,
                color,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
                parallax_map,
            ),
            MaterialInstanceFeatureFlags::HAS_COLOR
                | MaterialInstanceFeatureFlags::USES_PARALLAX_MAPPING,
        ),
        (None, Some(parallax_map)) => (
            TexturedColorParallaxMappedPhysicalMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorParallaxMappedPhysicalMaterialFeature::add_feature(
                instance_feature_manager,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
                parallax_map,
            ),
            MaterialInstanceFeatureFlags::USES_PARALLAX_MAPPING,
        ),
    }
}

impl UniformColorPhysicalMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        color: &UniformColorComp,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformColorPhysicalMaterialFeature features")
            .add_feature(&Self {
                color: color.0,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
            })
    }
}

impl TexturedColorPhysicalMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorPhysicalMaterialFeature features")
            .add_feature(&Self {
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
            })
    }
}

impl UniformColorParallaxMappedPhysicalMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        color: &UniformColorComp,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformColorParallaxMappedPhysicalMaterialFeature features",
            )
            .add_feature(&Self {
                color: color.0,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl TexturedColorParallaxMappedPhysicalMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for TexturedColorParallaxMappedPhysicalMaterialFeature features",
            )
            .add_feature(&Self {
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl_InstanceFeature!(
    FixedColorMaterialFeature,
    wgpu::vertex_attr_array![MaterialInstanceFeatureLocation::Color as u32 => Float32x3]
);

impl_InstanceFeature!(
    UniformColorPhysicalMaterialFeature,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
        MaterialInstanceFeatureLocation::Color as u32 => Float32x3,
    ]
);

impl_InstanceFeature!(
    TexturedColorPhysicalMaterialFeature,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
    ]
);

impl_InstanceFeature!(
    UniformColorParallaxMappedPhysicalMaterialFeature,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
        MaterialInstanceFeatureLocation::Color as u32 => Float32x3,
        MaterialInstanceFeatureLocation::ParallaxDisplacementScale as u32 => Float32,
        MaterialInstanceFeatureLocation::ParallaxUVPerDistance as u32 => Float32x2,
    ]
);

impl_InstanceFeature!(
    TexturedColorParallaxMappedPhysicalMaterialFeature,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
        MaterialInstanceFeatureLocation::ParallaxDisplacementScale as u32 => Float32,
        MaterialInstanceFeatureLocation::ParallaxUVPerDistance as u32 => Float32x2,
    ]
);
