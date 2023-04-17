//! Instance features representing material properties.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, InstanceFeatureID, InstanceFeatureTypeID},
    impl_InstanceFeature,
    rendering::{fre, InstanceFeatureShaderInput, LightMaterialFeatureShaderInput},
    scene::{
        DiffuseColorComp, EmissiveColorComp, InstanceFeatureManager, ParallaxMapComp, RGBColor,
        SpecularColorComp,
    },
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector2;

/// Fixed material properties for a material with no uniform color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorMaterialFeature {
    roughness: fre,
}

/// Fixed material properties for a material with a uniform diffuse color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseMaterialFeature {
    diffuse_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform specular color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularMaterialFeature {
    specular_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform diffuse and specular
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with no uniform color using
/// parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorParallaxMappingMaterialFeature {
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform diffuse color using
/// parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseParallaxMappingMaterialFeature {
    diffuse_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform specular color using
/// parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularParallaxMappingMaterialFeature {
    specular_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform diffuse and specular
/// color using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularParallaxMappingMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with only an emissive uniform
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorEmissiveMaterialFeature {
    emissive_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform diffuse and emissive
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseEmissiveMaterialFeature {
    diffuse_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform specular and
/// emissive color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularEmissiveMaterialFeature {
    specular_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform diffuse, specular
/// and emissive color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularEmissiveMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
}

/// Fixed material properties for a material with a uniform emissive color using
/// parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorParallaxMappingEmissiveMaterialFeature {
    emissive_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform diffuse and emissive
/// color using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseParallaxMappingEmissiveMaterialFeature {
    diffuse_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform specular and
/// emissive color using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularParallaxMappingEmissiveMaterialFeature {
    specular_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a material with a uniform diffuse, specular
/// and emissive color using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    emissive_color: RGBColor,
    roughness: fre,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Creates the appropriate material feature for the given set of components and
/// adds it to the instance feature manager. A tag identifying the feature type
/// is appended to the given list of name parts.
///
/// # Returns
/// The ID of the created feature type and the ID of the created feature.
pub fn create_material_feature(
    instance_feature_manager: &mut InstanceFeatureManager,
    material_name_parts: &mut Vec<&str>,
    diffuse_color: Option<&DiffuseColorComp>,
    specular_color: Option<&SpecularColorComp>,
    emissive_color: Option<&EmissiveColorComp>,
    roughness: fre,
    parallax_map: Option<&ParallaxMapComp>,
) -> (InstanceFeatureTypeID, InstanceFeatureID) {
    match (diffuse_color, specular_color, emissive_color, parallax_map) {
        (None, None, None, None) => (
            TexturedColorMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorMaterialFeature::add_feature(instance_feature_manager, roughness),
        ),
        (Some(diffuse_color), None, None, None) => {
            material_name_parts.push("UniformDiffuse");

            (
                UniformDiffuseMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    roughness,
                ),
            )
        }
        (None, Some(specular_color), None, None) => {
            material_name_parts.push("UniformSpecular");

            (
                UniformSpecularMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    roughness,
                ),
            )
        }
        (Some(diffuse_color), Some(specular_color), None, None) => {
            material_name_parts.push("UniformDiffuseUniformSpecular");

            (
                UniformDiffuseUniformSpecularMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    roughness,
                ),
            )
        }
        (None, None, None, Some(parallax_map)) => {
            material_name_parts.push("ParallaxMapping");
            (
                TexturedColorParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                TexturedColorParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(diffuse_color), None, None, Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseParallaxMapping");

            (
                UniformDiffuseParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, Some(specular_color), None, Some(parallax_map)) => {
            material_name_parts.push("UniformSpecularParallaxMapping");

            (
                UniformSpecularParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(diffuse_color), Some(specular_color), None, Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseUniformSpecularParallaxMapping");

            (
                UniformDiffuseUniformSpecularParallaxMappingMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularParallaxMappingMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, None, Some(emissive_color), None) => {
            material_name_parts.push("Emissive");
            (
                TexturedColorEmissiveMaterialFeature::FEATURE_TYPE_ID,
                TexturedColorEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    emissive_color,
                    roughness,
                ),
            )
        }
        (Some(diffuse_color), None, Some(emissive_color), None) => {
            material_name_parts.push("UniformDiffuseEmissive");

            (
                UniformDiffuseEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    emissive_color,
                    roughness,
                ),
            )
        }
        (None, Some(specular_color), Some(emissive_color), None) => {
            material_name_parts.push("UniformSpecularEmissive");

            (
                UniformSpecularEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    emissive_color,
                    roughness,
                ),
            )
        }
        (Some(diffuse_color), Some(specular_color), Some(emissive_color), None) => {
            material_name_parts.push("UniformDiffuseUniformSpecularEmissive");

            (
                UniformDiffuseUniformSpecularEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    emissive_color,
                    roughness,
                ),
            )
        }
        (None, None, Some(emissive_color), Some(parallax_map)) => {
            material_name_parts.push("ParallaxMappingEmissive");
            (
                TexturedColorParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                TexturedColorParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    emissive_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(diffuse_color), None, Some(emissive_color), Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseParallaxMappingEmissive");

            (
                UniformDiffuseParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    emissive_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (None, Some(specular_color), Some(emissive_color), Some(parallax_map)) => {
            material_name_parts.push("UniformSpecularParallaxMappingEmissive");

            (
                UniformSpecularParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    emissive_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
        (Some(diffuse_color), Some(specular_color), Some(emissive_color), Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseUniformSpecularParallaxMappingEmissive");

            (
                UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    emissive_color,
                    roughness,
                    parallax_map,
                ),
            )
        }
    }
}

impl TexturedColorMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorMaterialFeature features")
            .add_feature(&Self { roughness })
    }
}

impl UniformDiffuseMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                roughness,
            })
    }
}

impl UniformSpecularMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularMaterialFeature features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                roughness,
            })
    }
}

impl UniformDiffuseUniformSpecularMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                roughness,
            })
    }
}

impl TexturedColorParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorParallaxMappingMaterialFeature features")
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
        diffuse_color: &DiffuseColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformSpecularParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseUniformSpecularParallaxMappingMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularParallaxMappingMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl TexturedColorEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorEmissiveMaterialFeature features")
            .add_feature(&Self {
                emissive_color: emissive_color.0,
                roughness,
            })
    }
}

impl UniformDiffuseEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseEmissiveMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                emissive_color: emissive_color.0,
                roughness,
            })
    }
}

impl UniformSpecularEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularEmissiveMaterialFeature features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                emissive_color: emissive_color.0,
                roughness,
            })
    }
}

impl UniformDiffuseUniformSpecularEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformDiffuseUniformSpecularEmissiveMaterialFeature features",
            )
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                emissive_color: emissive_color.0,
                roughness,
            })
    }
}

impl TexturedColorParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for TexturedColorParallaxMappingEmissiveMaterialFeature features",
            )
            .add_feature(&Self {
                emissive_color: emissive_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformDiffuseParallaxMappingEmissiveMaterialFeature features",
            )
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                emissive_color: emissive_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformSpecularParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularParallaxMappingEmissiveMaterialFeature features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                emissive_color: emissive_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        emissive_color: &EmissiveColorComp,
        roughness: fre,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseUniformSpecularParallaxMappingEmissiveMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                emissive_color: emissive_color.0,
                roughness,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl_InstanceFeature!(
    TexturedColorMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        emissive_color_location: None,
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        emissive_color_location: None,
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
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_color_location: None,
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_color_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    TexturedColorParallaxMappingMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        emissive_color_location: None,
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        emissive_color_location: None,
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
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_color_location: None,
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_color_location: None,
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
    })
);

impl_InstanceFeature!(
    TexturedColorEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START),
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
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
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    TexturedColorParallaxMappingEmissiveMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32x2
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START),
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
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
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
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
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        emissive_color_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 5),
    })
);
