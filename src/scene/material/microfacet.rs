//! Materials using a microfacet reflection model.

use super::{create_material_feature, create_prepass_material};
use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        DiffuseMicrofacetShadingModel, MaterialPropertyTextureManager, MaterialShaderInput,
        MicrofacetShadingModel, MicrofacetTextureShaderInput, RenderAttachmentQuantitySet,
        RenderPassHints, SpecularMicrofacetShadingModel,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, EmissiveColorComp, InstanceFeatureManager,
        MaterialComp, MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureSet,
        MaterialPropertyTextureSetID, MaterialSpecification, MicrofacetDiffuseReflectionComp,
        MicrofacetSpecularReflectionComp, NormalMapComp, ParallaxMapComp,
        RenderResourcesDesynchronized, RoughnessComp, RoughnessTextureComp, SpecularColorComp,
        SpecularTextureComp,
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use std::sync::RwLock;

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
         emissive_color: Option<&EmissiveColorComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                emissive_color,
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
        [MicrofacetDiffuseReflectionComp],
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
        |emissive_color: Option<&EmissiveColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_color,
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
        [MicrofacetDiffuseReflectionComp],
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
         emissive_color: Option<&EmissiveColorComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
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
        |emissive_color: Option<&EmissiveColorComp>,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
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
         emissive_color: Option<&EmissiveColorComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                Some(specular_color),
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
        ![
            MaterialComp,
            DiffuseTextureComp,
            SpecularTextureComp,
            MicrofacetDiffuseReflectionComp
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
         emissive_color: Option<&EmissiveColorComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                Some(specular_color),
                emissive_color,
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
        [
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ],
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
         emissive_color: Option<&EmissiveColorComp>,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
        ![
            MaterialComp,
            SpecularColorComp,
            DiffuseTextureComp,
            MicrofacetDiffuseReflectionComp
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
         emissive_color: Option<&EmissiveColorComp>,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                Some(diffuse_color),
                None,
                emissive_color,
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
        [
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ],
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
         emissive_color: Option<&EmissiveColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
        ![
            MaterialComp,
            DiffuseColorComp,
            SpecularTextureComp,
            MicrofacetDiffuseReflectionComp
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
         emissive_color: Option<&EmissiveColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_color),
                emissive_color,
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
        [
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ],
        ![MaterialComp, DiffuseColorComp, SpecularTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |emissive_color: Option<&EmissiveColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_color,
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
        [MicrofacetSpecularReflectionComp],
        ![
            MaterialComp,
            DiffuseColorComp,
            SpecularColorComp,
            MicrofacetDiffuseReflectionComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |emissive_color: Option<&EmissiveColorComp>,
         diffuse_texture: &DiffuseTextureComp,
         specular_texture: &SpecularTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_color,
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
        [
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ],
        ![MaterialComp, DiffuseColorComp, SpecularColorComp]
    );
}

pub fn setup_microfacet_material(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    diffuse_color: Option<&DiffuseColorComp>,
    specular_color: Option<&SpecularColorComp>,
    emissive_color: Option<&EmissiveColorComp>,
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

    let (feature_type_id, feature_id) = create_material_feature(
        instance_feature_manager,
        &mut material_name_parts,
        diffuse_color,
        specular_color,
        emissive_color,
        roughness_value,
        parallax_map,
    );

    let mut vertex_attribute_requirements_for_shader =
        VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR;
    let mut vertex_attribute_requirements_for_mesh = vertex_attribute_requirements_for_shader;

    let mut texture_shader_input = MicrofacetTextureShaderInput {
        diffuse_texture_and_sampler_bindings: None,
        specular_texture_and_sampler_bindings: None,
        roughness_texture_and_sampler_bindings: None,
    };

    let mut texture_ids = Vec::with_capacity(5);

    if let Some(diffuse_texture) = diffuse_texture {
        assert!(
            diffuse_color.is_none(),
            "Tried to create microfacet material with both uniform and textured diffuse color"
        );

        material_name_parts.push("TexturedDiffuse");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.diffuse_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(diffuse_texture.0);
    }

    if let Some(specular_texture) = specular_texture {
        assert!(
            specular_color.is_none(),
            "Tried to create microfacet material with both uniform and textured specular color"
        );

        material_name_parts.push("TexturedSpecular");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

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

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.roughness_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(roughness_texture.texture_id);
    }

    let mut input_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

    let prepass_material_handle = create_prepass_material(
        material_library,
        &mut input_render_attachment_quantities,
        material_name_parts.clone(),
        feature_type_id,
        feature_id,
        texture_ids.clone(),
        texture_shader_input.diffuse_texture_and_sampler_bindings,
        texture_shader_input.specular_texture_and_sampler_bindings,
        texture_shader_input.roughness_texture_and_sampler_bindings,
        normal_map,
        parallax_map,
        specular_color.is_some() || specular_texture.is_some(),
    );

    if normal_map.is_some() || parallax_map.is_some() {
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TANGENT_SPACE_QUATERNION;
    }

    if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::NORMAL_VECTOR) {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::NORMAL_VECTOR;
    }

    if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::TEXTURE_COORDS) {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::TEXTURE_COORDS;
    }

    let material_id = MaterialID(hash64!(format!(
        "{}{}MicrofacetMaterial",
        material_name_parts.join(""),
        input_render_attachment_quantities,
    )));

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements_for_mesh,
                vertex_attribute_requirements_for_shader,
                input_render_attachment_quantities,
                RenderAttachmentQuantitySet::COLOR,
                None,
                vec![feature_type_id],
                RenderPassHints::AFFECTED_BY_LIGHT,
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

    MaterialComp::new(
        MaterialHandle::new(material_id, Some(feature_id), texture_set_id),
        Some(prepass_material_handle),
    )
}
