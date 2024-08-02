//! Materials using a microfacet reflection model.

use super::{super::features::create_physical_material_feature, prepass::create_prepass_material};
use crate::{
    assets::Assets,
    gpu::{
        rendering::render_command::{Blending, RenderPipelineHints},
        shader::{
            DiffuseMicrofacetShadingModel, MaterialShaderInput, MicrofacetShadingModel,
            MicrofacetTextureShaderInput, SpecularMicrofacetShadingModel,
        },
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescription,
            RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
            RenderAttachmentQuantitySet,
        },
        GraphicsDevice,
    },
    material::{
        components::{
            AlbedoComp, AlbedoTextureComp, EmissiveLuminanceComp, MaterialComp,
            MicrofacetDiffuseReflectionComp, MicrofacetSpecularReflectionComp, NormalMapComp,
            ParallaxMapComp, RoughnessComp, RoughnessTextureComp, SpecularReflectanceComp,
            SpecularReflectanceTextureComp,
        },
        MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
        MaterialPropertyTextureGroupID, MaterialSpecification,
    },
    mesh::VertexAttributeSet,
    model::InstanceFeatureManager,
    scene::RenderResourcesDesynchronized,
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
pub fn setup_microfacet_material_for_new_entity(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
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
        |albedo: &AlbedoComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                None,
                emissive_luminance,
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
            SpecularReflectanceComp,
            AlbedoTextureComp,
            SpecularReflectanceTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_luminance,
                Some(albedo_texture),
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
            SpecularReflectanceComp,
            AlbedoComp,
            SpecularReflectanceTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_reflectance),
                emissive_luminance,
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
            AlbedoComp,
            AlbedoTextureComp,
            SpecularReflectanceTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |emissive_luminance: Option<&EmissiveLuminanceComp>,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_luminance,
                None,
                Some(specular_reflectance_texture),
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
            AlbedoComp,
            SpecularReflectanceComp,
            AlbedoTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |albedo: &AlbedoComp,
         specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                Some(specular_reflectance),
                emissive_luminance,
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
            AlbedoTextureComp,
            SpecularReflectanceTextureComp,
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
        |albedo: &AlbedoComp,
         specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                Some(specular_reflectance),
                emissive_luminance,
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
        ![
            MaterialComp,
            AlbedoTextureComp,
            SpecularReflectanceTextureComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |albedo: &AlbedoComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                None,
                emissive_luminance,
                None,
                Some(specular_reflectance_texture),
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
            SpecularReflectanceComp,
            AlbedoTextureComp,
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
        |albedo: &AlbedoComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                None,
                emissive_luminance,
                None,
                Some(specular_reflectance_texture),
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
        ![MaterialComp, SpecularReflectanceComp, AlbedoTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_reflectance),
                emissive_luminance,
                Some(albedo_texture),
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
            AlbedoComp,
            SpecularReflectanceTextureComp,
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
        |specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                Some(specular_reflectance),
                emissive_luminance,
                Some(albedo_texture),
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
        ![MaterialComp, AlbedoComp, SpecularReflectanceTextureComp]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_luminance,
                Some(albedo_texture),
                Some(specular_reflectance_texture),
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
            AlbedoComp,
            SpecularReflectanceComp,
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
        |emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         roughness_texture: Option<&RoughnessTextureComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_microfacet_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                None,
                emissive_luminance,
                Some(albedo_texture),
                Some(specular_reflectance_texture),
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
        ![MaterialComp, AlbedoComp, SpecularReflectanceComp]
    );
}

pub fn setup_microfacet_material(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    albedo: Option<&AlbedoComp>,
    specular_reflectance: Option<&SpecularReflectanceComp>,
    emissive_luminance: Option<&EmissiveLuminanceComp>,
    albedo_texture: Option<&AlbedoTextureComp>,
    specular_reflectance_texture: Option<&SpecularReflectanceTextureComp>,
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

    let (feature_type_id, feature_id) = create_physical_material_feature(
        instance_feature_manager,
        &mut material_name_parts,
        albedo,
        specular_reflectance,
        emissive_luminance,
        roughness_value,
        parallax_map,
    );

    let mut vertex_attribute_requirements_for_shader =
        VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR;
    let mut vertex_attribute_requirements_for_mesh = vertex_attribute_requirements_for_shader;

    let mut texture_shader_input = MicrofacetTextureShaderInput {
        albedo_texture_and_sampler_bindings: None,
        specular_reflectance_texture_and_sampler_bindings: None,
        roughness_texture_and_sampler_bindings: None,
    };

    let mut texture_ids = Vec::with_capacity(5);

    if let Some(albedo_texture) = albedo_texture {
        assert!(
            albedo.is_none(),
            "Tried to create microfacet material with both uniform and textured albedo"
        );

        material_name_parts.push("TexturedDiffuse");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.albedo_texture_and_sampler_bindings =
            Some(MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()));
        texture_ids.push(albedo_texture.0);
    }

    if let Some(specular_reflectance_texture) = specular_reflectance_texture {
        assert!(
            specular_reflectance.is_none(),
            "Tried to create microfacet material with both uniform and textured specular reflectance"
        );

        material_name_parts.push("TexturedSpecular");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.specular_reflectance_texture_and_sampler_bindings =
            Some(MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()));
        texture_ids.push(specular_reflectance_texture.0);
    }

    if let Some(roughness_texture) = roughness_texture {
        assert!(
            roughness.is_none(),
            "Tried to create microfacet material with both uniform and textured roughness"
        );

        material_name_parts.push("TexturedRoughness");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.roughness_texture_and_sampler_bindings =
            Some(MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()));
        texture_ids.push(roughness_texture.texture_id);
    }

    let mut input_render_attachments = RenderAttachmentInputDescriptionSet::empty();

    let prepass_material_handle = create_prepass_material(
        graphics_device,
        assets,
        material_library,
        &mut input_render_attachments,
        material_name_parts.clone(),
        feature_type_id,
        feature_id,
        texture_ids.clone(),
        texture_shader_input.albedo_texture_and_sampler_bindings,
        texture_shader_input.specular_reflectance_texture_and_sampler_bindings,
        texture_shader_input.roughness_texture_and_sampler_bindings,
        normal_map,
        parallax_map,
        specular_reflectance.is_some() || specular_reflectance_texture.is_some(),
    );

    if normal_map.is_some() || parallax_map.is_some() {
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TANGENT_SPACE_QUATERNION;
    }

    if input_render_attachments
        .quantities()
        .contains(RenderAttachmentQuantitySet::NORMAL_VECTOR)
    {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::NORMAL_VECTOR;
    }

    if input_render_attachments
        .quantities()
        .contains(RenderAttachmentQuantitySet::TEXTURE_COORDS)
    {
        vertex_attribute_requirements_for_shader -= VertexAttributeSet::TEXTURE_COORDS;
    }

    let material_id = MaterialID(hash64!(format!(
        "{}{}MicrofacetMaterial",
        material_name_parts.join(""),
        input_render_attachments.quantities(),
    )));

    let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
        RenderAttachmentQuantity::Luminance,
        RenderAttachmentOutputDescription::default().with_blending(Blending::Additive),
    );

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements_for_mesh,
                vertex_attribute_requirements_for_shader,
                input_render_attachments,
                output_render_attachments,
                None,
                vec![feature_type_id],
                RenderPipelineHints::AFFECTED_BY_LIGHT,
                MaterialShaderInput::Microfacet((
                    MicrofacetShadingModel {
                        diffuse: diffuse_shading_model,
                        specular: specular_shading_model,
                    },
                    texture_shader_input,
                )),
            )
        });

    let texture_group_id = if !texture_ids.is_empty() {
        let texture_group_id = MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

        // Add a new texture set if none with the same textures already exist
        material_library
            .material_property_texture_group_entry(texture_group_id)
            .or_insert_with(|| {
                MaterialPropertyTextureGroup::new(
                    graphics_device,
                    assets,
                    texture_ids,
                    texture_group_id.to_string(),
                )
                .expect("Missing textures from assets")
            });

        Some(texture_group_id)
    } else {
        None
    };

    MaterialComp::new(
        MaterialHandle::new(material_id, Some(feature_id), texture_group_id),
        Some(prepass_material_handle),
    )
}
