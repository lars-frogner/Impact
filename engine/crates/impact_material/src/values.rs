//! Material properties.

use crate::{
    RGBColor,
    setup::physical::{ParallaxMap, UniformColor},
};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_gpu::vertex_attribute_ranges::MATERIAL_START;
use impact_gpu::wgpu;
use impact_math::vector::Vector2;
use impact_model::impl_InstanceFeatureForGPU;
use impact_model::{InstanceFeature, InstanceFeatureTypeID, ModelInstanceManager};
use std::hash::Hash;

bitflags! {
    /// Bitflags encoding information related to a material's properties.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct MaterialPropertyFlags: u8 {
        const HAS_COLOR = 1 << 0;
        const USES_PARALLAX_MAPPING = 1 << 1;
    }
}

/// Fixed property values for a material.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MaterialPropertyValues {
    Fixed(FixedMaterialPropertyValues),
    Physical(PhysicalMaterialPropertyValues),
}

/// Fixed property values for a fixed material with a uniform or textured color
/// that is independent of lighting.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FixedMaterialPropertyValues {
    FixedColor(FixedColorMaterialValues),
    None,
}

/// Fixed property values for a physical material.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PhysicalMaterialPropertyValues {
    UniformColor(UniformColorPhysicalMaterialValues),
    TexturedColor(TexturedColorPhysicalMaterialValues),
    UniformColorParallaxMapped(UniformColorParallaxMappedPhysicalMaterialValues),
    TexturedColorParallaxMapped(TexturedColorParallaxMappedPhysicalMaterialValues),
}

/// Fixed property values for a non-physical material with a uniform color that
/// is independent of lighting.
///
/// This type implements [`InstanceFeature`], and can thus be buffered in a
/// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct FixedColorMaterialValues {
    color: RGBColor,
}

/// Fixed property values for a physical material with a uniform base color.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type implements [`InstanceFeature`], and can thus be buffered in a
/// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorPhysicalMaterialValues {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    color: RGBColor,
}

/// Fixed property values for a physical material with a textured base color.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type implements [`InstanceFeature`], and can thus be buffered in a
/// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorPhysicalMaterialValues {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
}

/// Fixed property values for a physical material with a uniform base color and
/// parallax mapping.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type implements [`InstanceFeature`], and can thus be buffered in a
/// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorParallaxMappedPhysicalMaterialValues {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    color: RGBColor,
    parallax_displacement_scale: f32,
    parallax_uv_per_distance: Vector2,
}

/// Fixed property values for a physical material with a textured base color and
/// parallax mapping.
///
/// Each of the other material properties may either be uniform, in which case
/// the value in this object is used directly, or textured, in which case the
/// value in this object is used as a scale factor for the value sampled from
/// the texture.
///
/// This type implements [`InstanceFeature`], and can thus be buffered in a
/// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorParallaxMappedPhysicalMaterialValues {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    parallax_displacement_scale: f32,
    parallax_uv_per_distance: Vector2,
}

/// Vertex attribute location of a specific type of material instance feature.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MaterialInstanceFeatureLocation {
    SpecularReflectance = MATERIAL_START,
    Roughness = MATERIAL_START + 1,
    Metalness = MATERIAL_START + 2,
    EmissiveLuminance = MATERIAL_START + 3,
    Color = MATERIAL_START + 4,
    ParallaxDisplacementScale = MATERIAL_START + 5,
    ParallaxUVPerDistance = MATERIAL_START + 6,
}

impl MaterialPropertyValues {
    /// Wraps the appropriate [`FixedMaterialPropertyValues`] for the given set
    /// of fixed material properties.
    pub fn from_fixed_material_properties(fixed_color: Option<RGBColor>) -> Self {
        Self::Fixed(FixedMaterialPropertyValues::from_properties(fixed_color))
    }

    /// Wraps the appropriate [`PhysicalMaterialPropertyValues`] for the given
    /// set of physical material properties.
    pub fn from_physical_material_properties(
        uniform_color: Option<&UniformColor>,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
        parallax_map: Option<&ParallaxMap>,
    ) -> Self {
        Self::Physical(PhysicalMaterialPropertyValues::from_properties(
            uniform_color,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            parallax_map,
        ))
    }

    /// Whether the material is a fixed material.
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed(_))
    }

    /// Whether the material is a physical material.
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::Physical(_))
    }

    /// Returns the [`MaterialPropertyFlags`] corresponding to these material
    /// property values.
    pub fn flags(&self) -> MaterialPropertyFlags {
        match self {
            Self::Fixed(values) => values.flags(),
            Self::Physical(values) => values.flags(),
        }
    }

    /// Returns the [`InstanceFeatureTypeID`] the values can be stored under in
    /// a
    /// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
    pub fn instance_feature_type_id(&self) -> InstanceFeatureTypeID {
        match self {
            Self::Fixed(values) => values.instance_feature_type_id(),
            Self::Physical(values) => values.instance_feature_type_id(),
        }
    }

    /// Returns the [`InstanceFeatureTypeID`] the values can be stored under in
    /// a
    /// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer)
    /// if it is not N/A.
    pub fn instance_feature_type_id_if_applicable(&self) -> Option<InstanceFeatureTypeID> {
        let instance_feature_type_id = self.instance_feature_type_id();
        if instance_feature_type_id.is_not_applicable() {
            None
        } else {
            Some(instance_feature_type_id)
        }
    }

    /// Pushes the material property values onto the associated buffer for the
    /// model with the given ID.
    pub fn buffer<MID: Copy + Eq + Hash>(
        &self,
        model_instance_manager: &mut ModelInstanceManager<MID>,
        model_id: &MID,
    ) {
        match self {
            Self::Fixed(values) => {
                values.buffer(model_instance_manager, model_id);
            }
            Self::Physical(values) => {
                values.buffer(model_instance_manager, model_id);
            }
        }
    }
}

impl FixedMaterialPropertyValues {
    /// Constructs the appropriate [`FixedMaterialPropertyValues`] for the given
    /// set of properties.
    pub fn from_properties(fixed_color: Option<RGBColor>) -> FixedMaterialPropertyValues {
        match fixed_color {
            Some(color) => {
                FixedMaterialPropertyValues::FixedColor(FixedColorMaterialValues { color })
            }
            None => FixedMaterialPropertyValues::None,
        }
    }

    /// Returns the [`MaterialPropertyFlags`] corresponding to these material
    /// property values.
    pub fn flags(&self) -> MaterialPropertyFlags {
        match self {
            Self::FixedColor(_) => MaterialPropertyFlags::HAS_COLOR,
            Self::None => MaterialPropertyFlags::empty(),
        }
    }

    /// Returns the [`InstanceFeatureTypeID`] the values can be stored under in
    /// a
    /// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
    pub fn instance_feature_type_id(&self) -> InstanceFeatureTypeID {
        match self {
            Self::FixedColor(_) => FixedColorMaterialValues::FEATURE_TYPE_ID,
            Self::None => InstanceFeatureTypeID::not_applicable(),
        }
    }

    /// Pushes the material property values onto the associated buffer for the
    /// model with the given ID.
    pub fn buffer<MID: Copy + Eq + Hash>(
        &self,
        model_instance_manager: &mut ModelInstanceManager<MID>,
        model_id: &MID,
    ) {
        match self {
            Self::FixedColor(values) => {
                model_instance_manager.buffer_instance_feature(model_id, values);
            }
            Self::None => {}
        }
    }
}

impl PhysicalMaterialPropertyValues {
    /// Constructs the appropriate [`PhysicalMaterialPropertyValues`] for the
    /// given set of properties.
    pub fn from_properties(
        uniform_color: Option<&UniformColor>,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
        parallax_map: Option<&ParallaxMap>,
    ) -> PhysicalMaterialPropertyValues {
        match (uniform_color, parallax_map) {
            (Some(color), None) => {
                PhysicalMaterialPropertyValues::UniformColor(UniformColorPhysicalMaterialValues {
                    color: color.0,
                    specular_reflectance,
                    roughness,
                    metalness,
                    emissive_luminance,
                })
            }
            (None, None) => {
                PhysicalMaterialPropertyValues::TexturedColor(TexturedColorPhysicalMaterialValues {
                    specular_reflectance,
                    roughness,
                    metalness,
                    emissive_luminance,
                })
            }
            (Some(color), Some(parallax_map)) => {
                PhysicalMaterialPropertyValues::UniformColorParallaxMapped(
                    UniformColorParallaxMappedPhysicalMaterialValues {
                        color: color.0,
                        specular_reflectance,
                        roughness,
                        metalness,
                        emissive_luminance,
                        parallax_displacement_scale: parallax_map.displacement_scale as f32,
                        parallax_uv_per_distance: parallax_map.uv_per_distance,
                    },
                )
            }
            (None, Some(parallax_map)) => {
                PhysicalMaterialPropertyValues::TexturedColorParallaxMapped(
                    TexturedColorParallaxMappedPhysicalMaterialValues {
                        specular_reflectance,
                        roughness,
                        metalness,
                        emissive_luminance,
                        parallax_displacement_scale: parallax_map.displacement_scale as f32,
                        parallax_uv_per_distance: parallax_map.uv_per_distance,
                    },
                )
            }
        }
    }

    /// Returns the [`MaterialPropertyFlags`] corresponding to these material
    /// property values.
    pub fn flags(&self) -> MaterialPropertyFlags {
        match self {
            Self::UniformColor(_) => MaterialPropertyFlags::HAS_COLOR,
            Self::TexturedColor(_) => MaterialPropertyFlags::empty(),
            Self::UniformColorParallaxMapped(_) => {
                MaterialPropertyFlags::HAS_COLOR | MaterialPropertyFlags::USES_PARALLAX_MAPPING
            }
            Self::TexturedColorParallaxMapped(_) => MaterialPropertyFlags::USES_PARALLAX_MAPPING,
        }
    }

    /// Returns the [`InstanceFeatureTypeID`] the values can be stored under in
    /// a
    /// [`DynamicInstanceFeatureBuffer`](impact_model::DynamicInstanceFeatureBuffer).
    pub fn instance_feature_type_id(&self) -> InstanceFeatureTypeID {
        match self {
            Self::UniformColor(_) => UniformColorPhysicalMaterialValues::FEATURE_TYPE_ID,
            Self::TexturedColor(_) => TexturedColorPhysicalMaterialValues::FEATURE_TYPE_ID,
            Self::UniformColorParallaxMapped(_) => {
                UniformColorParallaxMappedPhysicalMaterialValues::FEATURE_TYPE_ID
            }
            Self::TexturedColorParallaxMapped(_) => {
                TexturedColorParallaxMappedPhysicalMaterialValues::FEATURE_TYPE_ID
            }
        }
    }

    /// Pushes the material property values onto the associated buffer for the
    /// model with the given ID.
    pub fn buffer<MID: Copy + Eq + Hash>(
        &self,
        model_instance_manager: &mut ModelInstanceManager<MID>,
        model_id: &MID,
    ) {
        match self {
            Self::UniformColor(values) => {
                model_instance_manager.buffer_instance_feature(model_id, values);
            }
            Self::TexturedColor(values) => {
                model_instance_manager.buffer_instance_feature(model_id, values);
            }
            Self::UniformColorParallaxMapped(values) => {
                model_instance_manager.buffer_instance_feature(model_id, values);
            }
            Self::TexturedColorParallaxMapped(values) => {
                model_instance_manager.buffer_instance_feature(model_id, values);
            }
        }
    }
}

impl_InstanceFeatureForGPU!(
    FixedColorMaterialValues,
    wgpu::vertex_attr_array![MaterialInstanceFeatureLocation::Color as u32 => Float32x3]
);

impl_InstanceFeatureForGPU!(
    UniformColorPhysicalMaterialValues,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
        MaterialInstanceFeatureLocation::Color as u32 => Float32x3,
    ]
);

impl_InstanceFeatureForGPU!(
    TexturedColorPhysicalMaterialValues,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
    ]
);

impl_InstanceFeatureForGPU!(
    UniformColorParallaxMappedPhysicalMaterialValues,
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

impl_InstanceFeatureForGPU!(
    TexturedColorParallaxMappedPhysicalMaterialValues,
    wgpu::vertex_attr_array![
        MaterialInstanceFeatureLocation::SpecularReflectance as u32 => Float32,
        MaterialInstanceFeatureLocation::Roughness as u32 => Float32,
        MaterialInstanceFeatureLocation::Metalness as u32 => Float32,
        MaterialInstanceFeatureLocation::EmissiveLuminance as u32 => Float32,
        MaterialInstanceFeatureLocation::ParallaxDisplacementScale as u32 => Float32,
        MaterialInstanceFeatureLocation::ParallaxUVPerDistance as u32 => Float32x2,
    ]
);

pub fn register_material_feature_types<MID: Copy + Eq + Hash>(
    model_instance_manager: &mut ModelInstanceManager<MID>,
) {
    model_instance_manager.register_feature_type::<FixedColorMaterialValues>();
    model_instance_manager.register_feature_type::<UniformColorPhysicalMaterialValues>();
    model_instance_manager.register_feature_type::<TexturedColorPhysicalMaterialValues>();
    model_instance_manager
        .register_feature_type::<UniformColorParallaxMappedPhysicalMaterialValues>();
    model_instance_manager
        .register_feature_type::<TexturedColorParallaxMappedPhysicalMaterialValues>();
}
