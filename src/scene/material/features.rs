//! Instance features representing material properties.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, InstanceFeatureID, InstanceFeatureTypeID},
    impl_InstanceFeature,
    rendering::{fre, InstanceFeatureShaderInput, LightMaterialFeatureShaderInput},
    scene::{
        DiffuseColorComp, InstanceFeatureManager, ParallaxMapComp, RGBColor, SpecularColorComp,
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
    roughness: fre,
    parallax_map: Option<&ParallaxMapComp>,
) -> (InstanceFeatureTypeID, InstanceFeatureID) {
    match (diffuse_color, specular_color, parallax_map) {
        (None, None, None) => (
            TexturedColorMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorMaterialFeature::add_feature(instance_feature_manager, roughness),
        ),
        (Some(diffuse_color), None, None) => {
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
        (None, Some(specular_color), None) => {
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
        (Some(diffuse_color), Some(specular_color), None) => {
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
        (None, None, Some(parallax_map)) => {
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
        (Some(diffuse_color), None, Some(parallax_map)) => {
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
        (None, Some(specular_color), Some(parallax_map)) => {
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
        (Some(diffuse_color), Some(specular_color), Some(parallax_map)) => {
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

impl_InstanceFeature!(
    TexturedColorMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
    ],
    InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
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
        roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 3),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 4),
    })
);
