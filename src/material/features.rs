//! Instance features representing material properties.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    gpu::{
        rendering::fre,
        shader::{
            FixedColorFeatureShaderInput, InstanceFeatureShaderInput,
            LightMaterialFeatureShaderInput,
        },
    },
    impl_InstanceFeature,
    material::{
        components::{AlbedoComp, EmissiveLuminanceComp, ParallaxMapComp, SpecularReflectanceComp},
        RGBColor,
    },
    model::{InstanceFeature, InstanceFeatureID, InstanceFeatureManager, InstanceFeatureTypeID},
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector2;

/// Fixed material properties for a non-physical material with a uniform color
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

/// Fixed material properties for a physical material with no uniform
/// reflectance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedMaterialFeature {
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform albedo.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseMaterialFeature {
    albedo: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform specular
/// reflectance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularMaterialFeature {
    specular_reflectance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform albedo and
/// specular reflectance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularMaterialFeature {
    albedo: RGBColor,
    specular_reflectance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with no uniform
/// reflectance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedParallaxMappingMaterialFeature {
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform albedo
/// using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseParallaxMappingMaterialFeature {
    albedo: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform specular
/// reflectance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularParallaxMappingMaterialFeature {
    specular_reflectance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform albedo and
/// specular reflectance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularParallaxMappingMaterialFeature {
    albedo: RGBColor,
    specular_reflectance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with only an emissive
/// uniform luminance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedEmissiveMaterialFeature {
    emissive_luminance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform albedo and
/// emissive luminance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseEmissiveMaterialFeature {
    albedo: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform specular
/// reflectance and emissive luminance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularEmissiveMaterialFeature {
    specular_reflectance: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform albedo,
/// specular reflectance and emissive luminance.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularEmissiveMaterialFeature {
    albedo: RGBColor,
    specular_reflectance: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a physical material with a uniform emissive
/// luminance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedParallaxMappingEmissiveMaterialFeature {
    emissive_luminance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform albedo and
/// emissive luminance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseParallaxMappingEmissiveMaterialFeature {
    albedo: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform specular
/// reflectance and emissive luminance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularParallaxMappingEmissiveMaterialFeature {
    specular_reflectance: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a physical material with a uniform albedo,
/// specular reflectance and emissive luminance using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::model::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature {
    albedo: RGBColor,
    specular_reflectance: RGBColor,
    emissive_luminance: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

impl FixedColorMaterialFeature {
    pub fn new(color: RGBColor) -> Self {
        Self { color }
    }
}

pub fn register_material_feature_types(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.register_feature_type::<FixedColorMaterialFeature>();
    instance_feature_manager.register_feature_type::<TexturedMaterialFeature>();
    instance_feature_manager.register_feature_type::<UniformDiffuseMaterialFeature>();
    instance_feature_manager.register_feature_type::<UniformSpecularMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseUniformSpecularMaterialFeature>();
    instance_feature_manager.register_feature_type::<TexturedParallaxMappingMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseParallaxMappingMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformSpecularParallaxMappingMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingMaterialFeature>();
    instance_feature_manager.register_feature_type::<TexturedEmissiveMaterialFeature>();
    instance_feature_manager.register_feature_type::<UniformDiffuseEmissiveMaterialFeature>();
    instance_feature_manager.register_feature_type::<UniformSpecularEmissiveMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseUniformSpecularEmissiveMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<TexturedParallaxMappingEmissiveMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseParallaxMappingEmissiveMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformSpecularParallaxMappingEmissiveMaterialFeature>();
    instance_feature_manager
        .register_feature_type::<UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature>();
}

/// Creates the appropriate physical material feature for the given set of
/// components and adds it to the instance feature manager. A tag identifying
/// the feature type is appended to the given list of name parts.
///
/// # Returns
/// The ID of the created feature type and the ID of the created feature.
pub fn create_physical_material_feature(
    instance_feature_manager: &mut InstanceFeatureManager,
    material_name_parts: &mut Vec<&str>,
    albedo: Option<&AlbedoComp>,
    specular_reflectance: Option<&SpecularReflectanceComp>,
    emissive_luminance: Option<&EmissiveLuminanceComp>,
    roughness: fre,
    parallax_map: Option<&ParallaxMapComp>,
) -> (InstanceFeatureTypeID, InstanceFeatureID) {
    match (
        albedo,
        specular_reflectance,
        emissive_luminance,
        parallax_map,
    ) {
        (None, None, None, None) => (
            TexturedMaterialFeature::FEATURE_TYPE_ID,
            TexturedMaterialFeature::add_feature(instance_feature_manager, roughness),
        ),
        (Some(albedo), None, None, None) => {
            material_name_parts.push("UniformDiffuse");

            (
                UniformDiffuseMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    roughness,
                ),
            )
        }
        (None, Some(specular_reflectance), None, None) => {
            material_name_parts.push("UniformSpecular");

            (
                UniformSpecularMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_reflectance,
                    roughness,
                ),
            )
        }
        (Some(albedo), Some(specular_reflectance), None, None) => {
            material_name_parts.push("UniformDiffuseUniformSpecular");

            (
                UniformDiffuseUniformSpecularMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    specular_reflectance,
                    roughness,
                ),
            )
        }
        (None, None, None, Some(parallax_map)) => {
            material_name_parts.push("ParallaxMapping");
            (
                TexturedParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                TexturedParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(albedo), None, None, Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseParallaxMapping");

            (
                UniformDiffuseParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, Some(specular_reflectance), None, Some(parallax_map)) => {
            material_name_parts.push("UniformSpecularParallaxMapping");

            (
                UniformSpecularParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_reflectance,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(albedo), Some(specular_reflectance), None, Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseUniformSpecularParallaxMapping");

            (
                UniformDiffuseUniformSpecularParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    specular_reflectance,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, None, Some(emissive_luminance), None) => {
            material_name_parts.push("Emissive");
            (
                TexturedEmissiveMaterialFeature::FEATURE_TYPE_ID,
                TexturedEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    emissive_luminance,
                    roughness,
                ),
            )
        }
        (Some(albedo), None, Some(emissive_luminance), None) => {
            material_name_parts.push("UniformDiffuseEmissive");

            (
                UniformDiffuseEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    emissive_luminance,
                    roughness,
                ),
            )
        }
        (None, Some(specular_reflectance), Some(emissive_luminance), None) => {
            material_name_parts.push("UniformSpecularEmissive");

            (
                UniformSpecularEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_reflectance,
                    emissive_luminance,
                    roughness,
                ),
            )
        }
        (Some(albedo), Some(specular_reflectance), Some(emissive_luminance), None) => {
            material_name_parts.push("UniformDiffuseUniformSpecularEmissive");

            (
                UniformDiffuseUniformSpecularEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    specular_reflectance,
                    emissive_luminance,
                    roughness,
                ),
            )
        }
        (None, None, Some(emissive_luminance), Some(parallax_map)) => {
            material_name_parts.push("ParallaxMappingEmissive");
            (
                TexturedParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                TexturedParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    emissive_luminance,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(albedo), None, Some(emissive_luminance), Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseParallaxMappingEmissive");

            (
                UniformDiffuseParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    emissive_luminance,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, Some(specular_reflectance), Some(emissive_luminance), Some(parallax_map)) => {
            material_name_parts.push("UniformSpecularParallaxMappingEmissive");

            (
                UniformSpecularParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_reflectance,
                    emissive_luminance,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (
            Some(albedo),
            Some(specular_reflectance),
            Some(emissive_luminance),
            Some(parallax_map),
        ) => {
            material_name_parts.push("UniformDiffuseUniformSpecularParallaxMappingEmissive");

            (
                UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    albedo,
                    specular_reflectance,
                    emissive_luminance,
                    roughness,
                    parallax_map,
                ),
            )
        }
    }
}

impl TexturedMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedMaterialFeature features")
            .add_feature(&Self { roughness })
    }
}

impl UniformDiffuseMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                roughness,
            })
    }
}

impl UniformSpecularMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: &SpecularReflectanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularMaterialFeature features")
            .add_feature(&Self {
                specular_reflectance: specular_reflectance.0,
                roughness,
            })
    }
}

impl UniformDiffuseUniformSpecularMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        specular_reflectance: &SpecularReflectanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                specular_reflectance: specular_reflectance.0,
                roughness,
            })
    }
}

impl TexturedParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformSpecularParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: &SpecularReflectanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                specular_reflectance: specular_reflectance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseUniformSpecularParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        specular_reflectance: &SpecularReflectanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                specular_reflectance: specular_reflectance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl TexturedEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedEmissiveMaterialFeature features")
            .add_feature(&Self {
                emissive_luminance: emissive_luminance.0,
                roughness,
            })
    }
}

impl UniformDiffuseEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseEmissiveMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
            })
    }
}

impl UniformSpecularEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: &SpecularReflectanceComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularEmissiveMaterialFeature features")
            .add_feature(&Self {
                specular_reflectance: specular_reflectance.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
            })
    }
}

impl UniformDiffuseUniformSpecularEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        specular_reflectance: &SpecularReflectanceComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformDiffuseUniformSpecularEmissiveMaterialFeature features",
            )
            .add_feature(&Self {
                albedo: albedo.0,
                specular_reflectance: specular_reflectance.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
            })
    }
}

impl TexturedParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedParallaxMappingEmissiveMaterialFeature features")
            .add_feature(&Self {
                emissive_luminance: emissive_luminance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformDiffuseParallaxMappingEmissiveMaterialFeature features",
            )
            .add_feature(&Self {
                albedo: albedo.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformSpecularParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_reflectance: &SpecularReflectanceComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularParallaxMappingEmissiveMaterialFeature features")
            .add_feature(&Self {
                specular_reflectance: specular_reflectance.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        albedo: &AlbedoComp,
        specular_reflectance: &SpecularReflectanceComp,
        emissive_luminance: &EmissiveLuminanceComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature features")
            .add_feature(&Self {
                albedo: albedo.0,
                specular_reflectance: specular_reflectance.0,
                emissive_luminance: emissive_luminance.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl_InstanceFeature!(
    FixedColorMaterialFeature,
    wgpu::vertex_attr_array![MATERIAL_VERTEX_BINDING_START => Float32x3],
    InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
        color_location: MATERIAL_VERTEX_BINDING_START,
    })
);

impl_InstanceFeature!(
    TexturedMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: None,
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformDiffuseMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: None,
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformSpecularMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformDiffuseUniformSpecularMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    TexturedParallaxMappingMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: None,
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
    })
);

impl_InstanceFeature!(
    UniformDiffuseParallaxMappingMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: None,
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
    })
);

impl_InstanceFeature!(
    UniformSpecularParallaxMappingMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
    })
);

impl_InstanceFeature!(
    UniformDiffuseUniformSpecularParallaxMappingMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
        MATERIAL_VERTEX_BINDING_START + 4 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_luminance_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
    })
);

impl_InstanceFeature!(
    TexturedEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: None,
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformDiffuseEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: None,
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformSpecularEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    UniformDiffuseUniformSpecularEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    TexturedParallaxMappingEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: None,
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
    })
);

impl_InstanceFeature!(
    UniformDiffuseParallaxMappingEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
        MATERIAL_VERTEX_BINDING_START + 4 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: None,
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
    })
);

impl_InstanceFeature!(
    UniformSpecularParallaxMappingEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
        MATERIAL_VERTEX_BINDING_START + 4 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: None,
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
    })
);

impl_InstanceFeature!(
    UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
        MATERIAL_VERTEX_BINDING_START + 4 => Float32,
        MATERIAL_VERTEX_BINDING_START + 5 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 5),
    })
);
