//! Materials using the Blinn-Phong reflection model.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput,
        InstanceFeatureShaderInput, MaterialPropertyTextureManager, MaterialShaderInput,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, InstanceFeatureManager, MaterialComp, MaterialID,
        MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, MicrofacetDiffuseReflection, MicrofacetSpecularReflection,
        NormalMapComp, ParallaxMapComp, RGBColor, RenderResourcesDesynchronized, RoughnessComp,
        SpecularColorComp, SpecularTextureComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

/// Material using the Blinn-Phong reflection model, with fixed diffuse and
/// specular colors and fixed shininess. Also includes a fixed height scale that
/// is needed if the material uses parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformColorBlinnPhongMaterial {
    diffuse_color: RGBColor,
    specular_color: RGBColor,
    shininess: fre,
    parallax_height_scale: fre,
}

/// Material using the Blinn-Phong reflection model, with fixed diffuse or
/// specular color and fixed shininess. Also includes a fixed height scale that
/// is needed if the material uses parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct SingleUniformColorBlinnPhongMaterial {
    color: RGBColor,
    shininess: fre,
    parallax_height_scale: fre,
}

/// Material using the Blinn-Phong reflection model, with textured diffuse
/// and/or specular colors and fixed shininess. Also includes a fixed height
/// scale that is needed if the material uses parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct ColorTexturedBlinnPhongMaterial {
    shininess: fre,
    parallax_height_scale: fre,
}

lazy_static! {
    static ref UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("UniformColoredBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NormalMappedUniformColoredBlinnPhongMaterial"));
    static ref PARALLAX_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("ParallaxMappedUniformColoredBlinnPhongMaterial"));
    static ref UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("UniformDiffuseBlinnPhongMaterial"));
    static ref UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("UniformSpecularBlinnPhongMaterial"));
    static ref UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("UniformDiffuseTexturedSpecularBlinnPhongMaterial"));
    static ref UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("UniformSpecularTexturedDiffuseBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NormalMappedUniformDiffuseBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NormalMappedUniformSpecularBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "NormalMappedUniformDiffuseTexturedSpecularBlinnPhongMaterial"
        ));
    static ref NORMAL_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "NormalMappedUniformSpecularTexturedDiffuseBlinnPhongMaterial"
        ));
    static ref PARALLAX_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("ParallaxMappedUniformDiffuseBlinnPhongMaterial"));
    static ref PARALLAX_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("ParallaxMappedUniformSpecularBlinnPhongMaterial"));
    static ref PARALLAX_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "ParallaxMappedUniformDiffuseTexturedSpecularBlinnPhongMaterial"
        ));
    static ref PARALLAX_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "ParallaxMappedUniformSpecularTexturedDiffuseBlinnPhongMaterial"
        ));
    static ref TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("TexturedDiffuseBlinnPhongMaterial"));
    static ref TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("TexturedSpecularBlinnPhongMaterial"));
    static ref TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("TexturedDiffuseSpecularBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NormalMappedTexturedDiffuseBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("NormalMappedTexturedSpecularBlinnPhongMaterial"));
    static ref NORMAL_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "NormalMappedTexturedDiffuseSpecularBlinnPhongMaterial"
        ));
    static ref PARALLAX_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("ParallaxMappedTexturedDiffuseBlinnPhongMaterial"));
    static ref PARALLAX_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("ParallaxMappedTexturedSpecularBlinnPhongMaterial"));
    static ref PARALLAX_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID: MaterialID =
        MaterialID(hash64!(
            "ParallaxMappedTexturedDiffuseSpecularBlinnPhongMaterial"
        ));
}

impl UniformColorBlinnPhongMaterial {
    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (material_id, vertex_attribute_requirements, texture_shader_input) in [
            (
                *UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *PARALLAX_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                },
            ),
        ] {
            let specification = MaterialSpecification::new(
                vertex_attribute_requirements,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::BlinnPhong(texture_shader_input),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for a variant of this material, and if so, adds the appropriate material
    /// property texture set to the material library if not present, registers
    /// the material in the given instance feature manager and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        material_library: &RwLock<MaterialLibrary>,
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            material_library: &RwLock<MaterialLibrary>,
            instance_feature_manager: &mut InstanceFeatureManager,
            diffuse_color: &DiffuseColorComp,
            specular_color: &SpecularColorComp,
            roughness: Option<&RoughnessComp>,
            normal_map: Option<&NormalMapComp>,
            parallax_map: Option<&ParallaxMapComp>,
        ) -> MaterialComp {
            let (material_id, texture_ids) = if let Some(parallax_map) = parallax_map {
                (
                    *PARALLAX_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID,
                    vec![
                        parallax_map.normal_map_texture_id,
                        parallax_map.height_map_texture_id,
                    ],
                )
            } else if let Some(normal_map) = normal_map {
                (
                    *NORMAL_MAPPED_UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID,
                    vec![normal_map.0],
                )
            } else {
                (*UNIFORM_COLORED_BLINN_PHONG_MATERIAL_ID, Vec::new())
            };

            let texture_set_id = if !texture_ids.is_empty() {
                let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

                // Add a new texture set if none with the same textures already exist
                material_library
                    .write()
                    .unwrap()
                    .material_property_texture_set_entry(texture_set_id)
                    .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

                Some(texture_set_id)
            } else {
                None
            };

            let material = UniformColorBlinnPhongMaterial {
                diffuse_color: diffuse_color.0,
                specular_color: specular_color.0,
                shininess: roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
                parallax_height_scale: parallax_map
                    .map_or(0.0, |parallax_map| parallax_map.height_scale),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<UniformColorBlinnPhongMaterial>()
                .expect("Missing storage for UniformColorBlinnPhongMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), texture_set_id)
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: &DiffuseColorComp,
             specular_color: &SpecularColorComp,
             roughness: Option<&RoughnessComp>,
             normal_map: Option<&NormalMapComp>,
             parallax_map: Option<&ParallaxMapComp>|
             -> MaterialComp {
                execute_setup(
                    material_library,
                    &mut instance_feature_manager,
                    diffuse_color,
                    specular_color,
                    roughness,
                    normal_map,
                    parallax_map,
                )
            },
            ![
                MaterialComp,
                DiffuseTextureComp,
                SpecularTextureComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl SingleUniformColorBlinnPhongMaterial {
    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (material_id, vertex_attribute_requirements, texture_shader_input) in [
            (
                *UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *PARALLAX_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
        ] {
            let specification = MaterialSpecification::new(
                vertex_attribute_requirements,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::BlinnPhong(texture_shader_input),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for a variant of this material, and if so, adds the appropriate material
    /// property texture set to the material library if not present, registers
    /// the material in the given instance feature manager and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            material_library: &RwLock<MaterialLibrary>,
            instance_feature_manager: &mut InstanceFeatureManager,
            diffuse_color: Option<&DiffuseColorComp>,
            specular_color: Option<&SpecularColorComp>,
            diffuse_texture: Option<&DiffuseTextureComp>,
            specular_texture: Option<&SpecularTextureComp>,
            roughness: Option<&RoughnessComp>,
            normal_map: Option<&NormalMapComp>,
            parallax_map: Option<&ParallaxMapComp>,
        ) -> MaterialComp {
            let (material_id, texture_ids, color) = match (
                diffuse_color,
                specular_color,
                diffuse_texture,
                specular_texture,
                normal_map,
                parallax_map
            ) {
                (Some(diffuse_color), None, None, None, None, None) => (
                    *UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    Vec::new(),
                    diffuse_color.0,
                ),
                (None, Some(specular_color), None, None, None, None) => (
                    *UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    Vec::new(),
                    specular_color.0,
                ),
                (Some(diffuse_color), None, None, Some(specular_texture), None, None) => (
                    *UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0],
                    diffuse_color.0,
                ),
                (None, Some(specular_color), Some(diffuse_texture), None, None, None) => (
                    *UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0],
                    specular_color.0,
                ),
                (Some(diffuse_color), None, None, None, Some(normal_map), None) => (
                    *NORMAL_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![normal_map.0],
                    diffuse_color.0,
                ),
                (None, Some(specular_color), None, None, Some(normal_map), None) => (
                    *NORMAL_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![normal_map.0],
                    specular_color.0,
                ),
                (Some(diffuse_color), None, None, Some(specular_texture), Some(normal_map), None) => (
                    *NORMAL_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0, normal_map.0],
                    diffuse_color.0,
                ),
                (None, Some(specular_color), Some(diffuse_texture), None, Some(normal_map), None) => (
                    *NORMAL_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, normal_map.0],
                    specular_color.0,
                ),
                (Some(diffuse_color), None, None, None, _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_UNIFORM_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                    diffuse_color.0,
                ),
                (None, Some(specular_color), None, None, _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_UNIFORM_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                    specular_color.0,
                ),
                (Some(diffuse_color), None, None, Some(specular_texture), _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_UNIFORM_DIFFUSE_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0, parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                    diffuse_color.0,
                ),
                (None, Some(specular_color), Some(diffuse_texture), None, _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_UNIFORM_SPECULAR_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                    specular_color.0,
                ),
                components => panic!("Invalid combination of material components for SingleUniformColorBlinnPhongMaterial: {:?}", components),
            };

            let texture_set_id = if !texture_ids.is_empty() {
                let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

                // Add a new texture set if none with the same textures already exist
                material_library
                    .write()
                    .unwrap()
                    .material_property_texture_set_entry(texture_set_id)
                    .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

                Some(texture_set_id)
            } else {
                None
            };

            let material = SingleUniformColorBlinnPhongMaterial {
                color,
                shininess: roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
                parallax_height_scale: parallax_map
                    .map_or(0.0, |parallax_map| parallax_map.height_scale),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<SingleUniformColorBlinnPhongMaterial>()
                .expect("Missing storage for SingleUniformColorBlinnPhongMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), texture_set_id)
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_color: &DiffuseColorComp,
             specular_texture: Option<&SpecularTextureComp>,
             roughness: Option<&RoughnessComp>,
             normal_map: Option<&NormalMapComp>,
             parallax_map: Option<&ParallaxMapComp>|
             -> MaterialComp {
                execute_setup(
                    material_library,
                    &mut instance_feature_manager,
                    Some(diffuse_color),
                    None,
                    None,
                    specular_texture,
                    roughness,
                    normal_map,
                    parallax_map,
                )
            },
            ![
                MaterialComp,
                SpecularColorComp,
                DiffuseTextureComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );

        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |specular_color: &SpecularColorComp,
             diffuse_texture: Option<&DiffuseTextureComp>,
             roughness: Option<&RoughnessComp>,
             normal_map: Option<&NormalMapComp>,
             parallax_map: Option<&ParallaxMapComp>|
             -> MaterialComp {
                execute_setup(
                    material_library,
                    &mut instance_feature_manager,
                    None,
                    Some(specular_color),
                    diffuse_texture,
                    None,
                    roughness,
                    normal_map,
                    parallax_map,
                )
            },
            ![
                MaterialComp,
                DiffuseColorComp,
                SpecularTextureComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl ColorTexturedBlinnPhongMaterial {
    /// Registers this material as a feature type in the given instance feature
    /// manager and adds the material specifications for the variants of the
    /// material to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        for (material_id, vertex_attribute_requirements, texture_shader_input) in [
            (
                *TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    normal_map_texture_and_sampler_bindings: None,
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *NORMAL_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                    height_map_texture_and_sampler_bindings: None,
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(3),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: None,
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: None,
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                },
            ),
            (
                *PARALLAX_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                VertexAttributeSet::FOR_BUMP_MAPPED_SHADING,
                BlinnPhongTextureShaderInput {
                    diffuse_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
                    ),
                    specular_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(1),
                    ),
                    normal_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(2),
                    ),
                    height_map_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(3),
                    ),
                },
            ),
        ] {
            let specification = MaterialSpecification::new(
                vertex_attribute_requirements,
                None,
                vec![Self::FEATURE_TYPE_ID],
                MaterialShaderInput::BlinnPhong(texture_shader_input),
            );
            material_library.add_material_specification(material_id, specification);
        }
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for a variant of this material, and if so, adds the appropriate material
    /// property texture set to the material library if not present, registers
    /// the material in the given instance feature manager and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        material_library: &RwLock<MaterialLibrary>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        fn execute_setup(
            material_library: &mut MaterialLibrary,
            instance_feature_manager: &mut InstanceFeatureManager,
            diffuse_texture: Option<&DiffuseTextureComp>,
            specular_texture: Option<&SpecularTextureComp>,
            roughness: Option<&RoughnessComp>,
            normal_map: Option<&NormalMapComp>,
            parallax_map: Option<&ParallaxMapComp>,
        ) -> MaterialComp {
            let (material_id, texture_ids) = match (
                diffuse_texture,
                specular_texture,
                normal_map,
                parallax_map,
            ) {
                (Some(diffuse_texture), None, None, None) => (
                    *TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0],
                ),
                (None, Some(specular_texture), None, None) => (
                    *TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0],
                ),
                (Some(diffuse_texture), Some(specular_texture), None, None) => (
                    *TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, specular_texture.0],
                ),
                (Some(diffuse_texture), None, Some(normal_map), None) => (
                    *NORMAL_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, normal_map.0],
                ),
                (None, Some(specular_texture), Some(normal_map), None) => (
                    *NORMAL_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0, normal_map.0],
                ),
                (Some(diffuse_texture), Some(specular_texture), Some(normal_map), None) => (
                    *NORMAL_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, specular_texture.0, normal_map.0],
                ),
                (Some(diffuse_texture), None, _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_TEXTURED_DIFFUSE_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                ),
                (None, Some(specular_texture), _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_TEXTURED_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![specular_texture.0, parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                ),
                (Some(diffuse_texture), Some(specular_texture), _, Some(parallax_map)) => (
                    *PARALLAX_MAPPED_TEXTURED_DIFFUSE_SPECULAR_BLINN_PHONG_MATERIAL_ID,
                    vec![diffuse_texture.0, specular_texture.0, parallax_map.normal_map_texture_id, parallax_map.height_map_texture_id],
                ),
                components => panic!("Invalid combination of material components for ColorTexturedBlinnPhongMaterial: {:?}", components),
            };

            let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

            // Add a new texture set if none with the same textures already exist
            material_library
                .material_property_texture_set_entry(texture_set_id)
                .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

            let material = ColorTexturedBlinnPhongMaterial {
                shininess: roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess()),
                parallax_height_scale: parallax_map
                    .map_or(0.0, |parallax_map| parallax_map.height_scale),
            };

            let feature_id = instance_feature_manager
                .get_storage_mut::<ColorTexturedBlinnPhongMaterial>()
                .expect("Missing storage for ColorTexturedBlinnPhongMaterial features")
                .add_feature(&material);

            MaterialComp::new(material_id, Some(feature_id), Some(texture_set_id))
        }

        setup!(
            {
                desynchronized.set_yes();
                let mut material_library = material_library.write().unwrap();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |diffuse_texture: &DiffuseTextureComp,
             specular_texture: Option<&SpecularTextureComp>,
             roughness: Option<&RoughnessComp>,
             normal_map: Option<&NormalMapComp>,
             parallax_map: Option<&ParallaxMapComp>|
             -> MaterialComp {
                execute_setup(
                    &mut material_library,
                    &mut instance_feature_manager,
                    Some(diffuse_texture),
                    specular_texture,
                    roughness,
                    normal_map,
                    parallax_map,
                )
            },
            ![
                MaterialComp,
                DiffuseColorComp,
                SpecularColorComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
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
             normal_map: Option<&NormalMapComp>,
             parallax_map: Option<&ParallaxMapComp>|
             -> MaterialComp {
                execute_setup(
                    &mut material_library,
                    &mut instance_feature_manager,
                    None,
                    Some(specular_texture),
                    roughness,
                    normal_map,
                    parallax_map,
                )
            },
            ![
                MaterialComp,
                DiffuseColorComp,
                DiffuseTextureComp,
                SpecularColorComp,
                MicrofacetDiffuseReflection,
                MicrofacetSpecularReflection
            ]
        );
    }
}

impl_InstanceFeature!(
    UniformColorBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
        MATERIAL_VERTEX_BINDING_START + 3 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 2,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 3,
    })
);

impl_InstanceFeature!(
    SingleUniformColorBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 2,
    })
);

impl_InstanceFeature!(
    ColorTexturedBlinnPhongMaterial,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
    ],
    InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
        diffuse_color_location: None,
        specular_color_location: None,
        shininess_location: MATERIAL_VERTEX_BINDING_START,
        parallax_height_scale_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);
