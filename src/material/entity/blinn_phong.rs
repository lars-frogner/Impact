//! Materials using the Blinn-Phong reflection model.

use super::{super::features::create_physical_material_feature, prepass::create_prepass_material};
use crate::{
    assets::Assets,
    gpu::{
        rendering::{RenderAttachmentQuantitySet, RenderPassHints},
        shader::{BlinnPhongTextureShaderInput, MaterialShaderInput},
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
    scene::{InstanceFeatureManager, RenderResourcesDesynchronized},
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has the components for
/// a Blinn-Phong material, and if so, adds the material specification to the
/// material library if not already present, adds the appropriate material
/// property texture set to the material library if not already present,
/// registers the material in the instance feature manager and adds the
/// appropriate material component to the entity.
pub fn add_blinn_phong_material_component_for_entity(
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
         specular_reflectance: Option<&SpecularReflectanceComp>,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         specular_reflectance_texture: Option<&SpecularReflectanceTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_blinn_phong_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                Some(albedo),
                specular_reflectance,
                emissive_luminance,
                None,
                specular_reflectance_texture,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            AlbedoTextureComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |albedo: Option<&AlbedoComp>,
         specular_reflectance: &SpecularReflectanceComp,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: Option<&AlbedoTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_blinn_phong_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                albedo,
                Some(specular_reflectance),
                emissive_luminance,
                albedo_texture,
                None,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            SpecularReflectanceTextureComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |specular_reflectance: Option<&SpecularReflectanceComp>,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: &AlbedoTextureComp,
         specular_reflectance_texture: Option<&SpecularReflectanceTextureComp>,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_blinn_phong_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                None,
                specular_reflectance,
                emissive_luminance,
                Some(albedo_texture),
                specular_reflectance_texture,
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            AlbedoComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ]
    );

    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |albedo: Option<&AlbedoComp>,
         emissive_luminance: Option<&EmissiveLuminanceComp>,
         albedo_texture: Option<&AlbedoTextureComp>,
         specular_reflectance_texture: &SpecularReflectanceTextureComp,
         roughness: Option<&RoughnessComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> MaterialComp {
            setup_blinn_phong_material(
                graphics_device,
                assets,
                &mut material_library,
                &mut instance_feature_manager,
                albedo,
                None,
                emissive_luminance,
                albedo_texture,
                Some(specular_reflectance_texture),
                roughness,
                normal_map,
                parallax_map,
            )
        },
        ![
            MaterialComp,
            SpecularReflectanceComp,
            RoughnessTextureComp,
            MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp
        ]
    );
}

pub fn setup_blinn_phong_material(
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
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
) -> MaterialComp {
    let mut material_name_parts = Vec::new();

    let shininess = roughness.map_or(1.0, |roughness| roughness.to_blinn_phong_shininess());

    let (feature_type_id, feature_id) = create_physical_material_feature(
        instance_feature_manager,
        &mut material_name_parts,
        albedo,
        specular_reflectance,
        emissive_luminance,
        shininess,
        parallax_map,
    );

    let mut vertex_attribute_requirements_for_shader =
        VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR;
    let mut vertex_attribute_requirements_for_mesh = vertex_attribute_requirements_for_shader;

    let mut texture_shader_input = BlinnPhongTextureShaderInput {
        albedo_texture_and_sampler_bindings: None,
        specular_reflectance_texture_and_sampler_bindings: None,
    };

    let mut texture_ids = Vec::with_capacity(4);

    if let Some(albedo_texture) = albedo_texture {
        assert!(
            albedo.is_none(),
            "Tried to create Blinn-Phong material with both uniform and textured albedo"
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
            "Tried to create Blinn-Phong material with both uniform and textured specular reflectance"
        );

        material_name_parts.push("TexturedSpecular");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.specular_reflectance_texture_and_sampler_bindings =
            Some(MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()));
        texture_ids.push(specular_reflectance_texture.0);
    }

    let mut input_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

    let prepass_material_handle = create_prepass_material(
        graphics_device,
        assets,
        material_library,
        &mut input_render_attachment_quantities,
        material_name_parts.clone(),
        feature_type_id,
        feature_id,
        texture_ids.clone(),
        texture_shader_input.albedo_texture_and_sampler_bindings,
        texture_shader_input.specular_reflectance_texture_and_sampler_bindings,
        None,
        normal_map,
        parallax_map,
        false,
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
        "{}{}BlinnPhongMaterial",
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
                RenderAttachmentQuantitySet::LUMINANCE,
                None,
                vec![feature_type_id],
                RenderPassHints::AFFECTED_BY_LIGHT,
                MaterialShaderInput::BlinnPhong(texture_shader_input),
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
