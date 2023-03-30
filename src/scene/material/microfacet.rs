//! Materials using a microfacet reflection model.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, InstanceFeatureID, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, DiffuseMicrofacetShadingModel, InstanceFeatureShaderInput,
        MaterialPropertyTextureManager, MaterialShaderInput, MicrofacetFeatureShaderInput,
        MicrofacetShadingModel, MicrofacetTextureShaderInput, SpecularMicrofacetShadingModel,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, InstanceFeatureManager, MaterialComp, MaterialID,
        MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, MicrofacetDiffuseReflection, MicrofacetSpecularReflection,
        NormalMapComp, ParallaxMapComp, RGBColor, RenderResourcesDesynchronized, RoughnessComp,
        RoughnessTextureComp, SpecularColorComp, SpecularTextureComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use std::sync::RwLock;

/// Fixed material properties for a microfacet material with uniform diffuse and
/// specular color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorMicrofacetMaterialFeature {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    roughness: fre,
    parallax_height_scale: fre,
}

/// Fixed material properties for a microfacet material with uniform diffuse
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseMicrofacetMaterialFeature {
    diffuse_color: RGBColor,
    roughness: fre,
    parallax_height_scale: fre,
}

/// Fixed material properties for a microfacet material with uniform specular
/// color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformSpecularMicrofacetMaterialFeature {
    specular_color: RGBColor,
    roughness: fre,
    parallax_height_scale: fre,
}

/// Fixed material properties for a microfacet material with no uniform color.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct TexturedColorMicrofacetMaterialFeature {
    roughness: fre,
    parallax_height_scale: fre,
}

/// Checks if the entity-to-be with the given components has the components for
/// a microfacet material, and if so, adds the material specification to the
/// material library if not already present, adds the appropriate material
/// property texture set to the material library if not already present,
/// registers the material in the instance feature manager and adds the
/// appropriate material component to the entity.
pub fn add_microfacet_material_component_for_entity(
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                None,
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::None,
            )
        },
        [MicrofacetDiffuseReflection],
        ![
            MaterialComp,
            SpecularColorComp,
            DiffuseTextureComp,
            SpecularTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                Some(diffuse_texture),
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::None,
            )
        },
        [MicrofacetDiffuseReflection],
        ![
            MaterialComp,
            SpecularColorComp,
            DiffuseColorComp,
            SpecularTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_color: &SpecularColorComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                None,
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::None,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            DiffuseColorComp,
            DiffuseTextureComp,
            SpecularTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                None,
                Some(specular_texture),
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::None,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            DiffuseColorComp,
            SpecularColorComp,
            DiffuseTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         specular_color: &SpecularColorComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                Some(specular_color),
                None,
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::Lambertian,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            DiffuseTextureComp,
            SpecularTextureComp,
            MicrofacetDiffuseReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         specular_color: &SpecularColorComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                Some(specular_color),
                None,
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
        ![MaterialComp, DiffuseTextureComp, SpecularTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                None,
                Some(specular_texture),
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::Lambertian,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            SpecularColorComp,
            DiffuseTextureComp,
            MicrofacetDiffuseReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_color: &DiffuseColorComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                None,
                Some(specular_texture),
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
        ![MaterialComp, SpecularColorComp, DiffuseTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_color: &SpecularColorComp,
         diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                Some(diffuse_texture),
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::Lambertian,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            DiffuseColorComp,
            SpecularTextureComp,
            MicrofacetDiffuseReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_color: &SpecularColorComp,
         diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                Some(diffuse_texture),
                None,
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
        ![MaterialComp, DiffuseColorComp, SpecularTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_texture: &DiffuseTextureComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                Some(diffuse_texture),
                Some(specular_texture),
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::Lambertian,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetSpecularReflection],
        ![
            MaterialComp,
            DiffuseColorComp,
            SpecularColorComp,
            MicrofacetDiffuseReflection
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |diffuse_texture: &DiffuseTextureComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            execute_material_setup(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                Some(diffuse_texture),
                Some(specular_texture),
                roughness,
                roughness_texture,
                normal_map,
                parallax_map,
                DiffuseMicrofacetShadingModel::GGX,
                SpecularMicrofacetShadingModel::GGX,
            )
        },
        [MicrofacetDiffuseReflection, MicrofacetSpecularReflection],
        ![MaterialComp, DiffuseColorComp, SpecularColorComp]
    );
}

fn execute_material_setup(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    diffuse_color: Option<&DiffuseColorComp>,
    specular_color: Option<&SpecularColorComp>,
    diffuse_texture: Option<&DiffuseTextureComp>,
    specular_texture: Option<&SpecularTextureComp>,
    roughness: Option<&RoughnessComp>,
    roughness_texture: Option<&RoughnessTextureComp>,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
    diffuse_shading_model: DiffuseMicrofacetShadingModel,
    specular_shading_model: SpecularMicrofacetShadingModel,
) -> MaterialComp {
    let diffuse_shading_model_name = match diffuse_shading_model {
        DiffuseMicrofacetShadingModel::None => "",
        DiffuseMicrofacetShadingModel::Lambertian => "LambertianDiffuse",
        DiffuseMicrofacetShadingModel::GGX => "GGXDiffuse",
    };
    let specular_shading_model_name = match specular_shading_model {
        SpecularMicrofacetShadingModel::None => "",
        SpecularMicrofacetShadingModel::GGX => "GGXSpecular",
    };

    let mut material_name_parts = vec![diffuse_shading_model_name, specular_shading_model_name];

    let roughness_value = if let Some(roughness) = roughness {
        roughness.to_ggx_roughness()
    } else if let Some(roughness_texture) = roughness_texture {
        roughness_texture.roughness_scale
    } else {
        1.0
    };

    let parallax_height_scale = parallax_map.map_or(0.0, |parallax_map| parallax_map.height_scale);

    let (feature_type_id, feature_id) = match (diffuse_color, specular_color) {
        (Some(diffuse_color), Some(specular_color)) => {
            material_name_parts.push("UniformDiffuseUniformSpecular");

            (
                UniformColorMicrofacetMaterialFeature::FEATURE_TYPE_ID,
                UniformColorMicrofacetMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    roughness_value,
                    parallax_height_scale,
                ),
            )
        }
        (Some(diffuse_color), None) => {
            material_name_parts.push("UniformDiffuse");

            (
                UniformDiffuseMicrofacetMaterialFeature::FEATURE_TYPE_ID,
                UniformDiffuseMicrofacetMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                    roughness_value,
                    parallax_height_scale,
                ),
            )
        }
        (None, Some(specular_color)) => {
            material_name_parts.push("UniformSpecular");

            (
                UniformSpecularMicrofacetMaterialFeature::FEATURE_TYPE_ID,
                UniformSpecularMicrofacetMaterialFeature::add_feature(
                    instance_feature_manager,
                    specular_color,
                    roughness_value,
                    parallax_height_scale,
                ),
            )
        }
        (None, None) => (
            TexturedColorMicrofacetMaterialFeature::FEATURE_TYPE_ID,
            TexturedColorMicrofacetMaterialFeature::add_feature(
                instance_feature_manager,
                roughness_value,
                parallax_height_scale,
            ),
        ),
    };

    let mut vertex_attribute_requirements = VertexAttributeSet::FOR_LIGHT_SHADING;

    let mut texture_shader_input = MicrofacetTextureShaderInput {
        diffuse_texture_and_sampler_bindings: None,
        specular_texture_and_sampler_bindings: None,
        roughness_texture_and_sampler_bindings: None,
        normal_map_texture_and_sampler_bindings: None,
        height_map_texture_and_sampler_bindings: None,
    };

    let mut texture_ids = Vec::with_capacity(5);

    if let Some(diffuse_texture) = diffuse_texture {
        assert!(
            diffuse_color.is_none(),
            "Tried to create microfacet material with both uniform and textured diffuse color"
        );

        material_name_parts.push("TexturedDiffuse");

        vertex_attribute_requirements |= VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING;

        texture_shader_input.diffuse_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(diffuse_texture.0);
    }

    if let Some(specular_texture) = specular_texture {
        assert!(
            diffuse_color.is_none(),
            "Tried to create microfacet material with both uniform and textured specular color"
        );

        material_name_parts.push("TexturedSpecular");

        vertex_attribute_requirements |= VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING;

        texture_shader_input.specular_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(specular_texture.0);
    }

    if let Some(roughness_texture) = roughness_texture {
        assert!(
            roughness.is_none(),
            "Tried to create microfacet material with both uniform and textured roughness"
        );

        material_name_parts.push("TexturedRoughness");

        vertex_attribute_requirements |= VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING;

        texture_shader_input.roughness_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(roughness_texture.texture_id);
    }

    if let Some(normal_map) = normal_map {
        assert!(
            diffuse_color.is_none(),
            "Tried to create microfacet material that uses both normal mapping and parallax mapping"
        );

        material_name_parts.push("NormalMapped");

        vertex_attribute_requirements |= VertexAttributeSet::FOR_BUMP_MAPPED_SHADING;

        texture_shader_input.normal_map_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(normal_map.0);
    } else if let Some(parallax_map) = parallax_map {
        material_name_parts.push("ParallaxMapped");

        vertex_attribute_requirements |= VertexAttributeSet::FOR_BUMP_MAPPED_SHADING;

        texture_shader_input.normal_map_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(parallax_map.normal_map_texture_id);

        texture_shader_input.height_map_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(parallax_map.height_map_texture_id);
    }

    let material_id = MaterialID(hash64!(format!(
        "{}MicrofacetMaterial",
        material_name_parts.join("")
    )));

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements,
                None,
                vec![feature_type_id],
                MaterialShaderInput::Microfacet((
                    MicrofacetShadingModel {
                        diffuse: diffuse_shading_model,
                        specular: specular_shading_model,
                    },
                    texture_shader_input,
                )),
            )
        });

    let texture_set_id = if !texture_ids.is_empty() {
        let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

        // Add a new texture set if none with the same textures already exist
        material_library
            .material_property_texture_set_entry(texture_set_id)
            .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

        Some(texture_set_id)
    } else {
        None
    };

    MaterialComp::new(material_id, Some(feature_id), texture_set_id)
}

impl UniformColorMicrofacetMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        specular_color: &SpecularColorComp,
        roughness: fre,
        parallax_height_scale: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformColorMicrofacetMaterial features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                roughness,
                parallax_height_scale,
            })
    }
}

impl UniformDiffuseMicrofacetMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        roughness: fre,
        parallax_height_scale: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffuseMicrofacetMaterial features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                roughness,
                parallax_height_scale,
            })
    }
}

impl UniformSpecularMicrofacetMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        specular_color: &SpecularColorComp,
        roughness: fre,
        parallax_height_scale: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformSpecularMicrofacetMaterial features")
            .add_feature(&Self {
                specular_color: specular_color.0,
                roughness,
                parallax_height_scale,
            })
    }
}

impl TexturedColorMicrofacetMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        roughness: fre,
        parallax_height_scale: fre,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for TexturedColorMicrofacetMaterial features")
            .add_feature(&Self {
                roughness,
                parallax_height_scale,
            })
    }
}

impl_InstanceFeature!(
    UniformColorMicrofacetMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        roughness_location: MATERIAL_VERTEX_BINDING_START + 2,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 3,
    })
);

impl_InstanceFeature!(
    UniformDiffuseMicrofacetMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: None,
        roughness_location: MATERIAL_VERTEX_BINDING_START + 1,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);

impl_InstanceFeature!(
    UniformSpecularMicrofacetMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        roughness_location: MATERIAL_VERTEX_BINDING_START + 1,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);

impl_InstanceFeature!(
    TexturedColorMicrofacetMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::MicrofacetMaterial(MicrofacetFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        roughness_location: MATERIAL_VERTEX_BINDING_START,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);
