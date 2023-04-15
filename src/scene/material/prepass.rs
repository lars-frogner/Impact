//! Materials for use in shading prepasses.

use super::NormalMapComp;
use crate::{
    geometry::{InstanceFeatureID, InstanceFeatureTypeID, VertexAttributeSet},
    rendering::{
        Assets, BumpMappingTextureShaderInput, MaterialPropertyTextureManager, MaterialShaderInput,
        NormalMappingShaderInput, ParallaxMappingShaderInput, PrepassTextureShaderInput,
        RenderAttachmentQuantitySet, TextureID,
    },
    scene::{
        MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureSet,
        MaterialPropertyTextureSetID, MaterialSpecification, ParallaxMapComp,
    },
};
use impact_utils::hash64;

/// Creates a prepass material based on the given information about the main
/// material. The given set of render attachment quantities that the prepass
/// material produces (and the main material consumes) may be extended. The
/// prepass material will use the same instance feature as the main material,
/// and a superset of the main material's texture set. The specification for the
/// perpass material is added to the material library if it does not already
/// exist.
///
/// # Returns
/// A [`MaterialHandle`] containing the ID of the material and the IDs of the
/// created instance feature and texture set, as well as a
/// [`RenderAttachmentQuantitySet`] encoding the quantities the prepass
/// material's shader will write into their dedicated render attachments.
///
/// # Panics
/// If both a normal map and a parallax map component is provided.
pub fn create_prepass_material(
    material_library: &mut MaterialLibrary,
    output_render_attachment_quantities: &mut RenderAttachmentQuantitySet,
    mut material_name_parts: Vec<&str>,
    feature_type_id: InstanceFeatureTypeID,
    feature_id: InstanceFeatureID,
    mut texture_ids: Vec<TextureID>,
    diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    specular_texture_and_sampler_bindings: Option<(u32, u32)>,
    roughness_texture_and_sampler_bindings: Option<(u32, u32)>,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
    uses_specular_microfacet_model: bool,
) -> MaterialHandle {
    let mut vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
    let mut vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

    if !texture_ids.is_empty() {
        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;
    }

    let mut texture_shader_input = PrepassTextureShaderInput {
        diffuse_texture_and_sampler_bindings,
        specular_texture_and_sampler_bindings,
        roughness_texture_and_sampler_bindings,
        specular_reflectance_lookup_texture_and_sampler_bindings: None,
        bump_mapping_input: None,
    };

    if let Some(normal_map) = normal_map {
        assert!(
            parallax_map.is_none(),
            "Tried to create prepass material that uses both normal mapping and parallax mapping"
        );

        material_name_parts.push("NormalMapping");

        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::NORMAL_VECTOR
            | VertexAttributeSet::TEXTURE_COORDS
            | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

        vertex_attribute_requirements_for_shader |=
            VertexAttributeSet::TEXTURE_COORDS | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

        *output_render_attachment_quantities |= RenderAttachmentQuantitySet::NORMAL_VECTOR;

        texture_shader_input.bump_mapping_input = Some(
            BumpMappingTextureShaderInput::NormalMapping(NormalMappingShaderInput {
                normal_map_texture_and_sampler_bindings:
                    MaterialPropertyTextureManager::get_texture_and_sampler_bindings(
                        texture_ids.len(),
                    ),
            }),
        );

        texture_ids.push(normal_map.0);
    } else if let Some(parallax_map) = parallax_map {
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::NORMAL_VECTOR
            | VertexAttributeSet::TEXTURE_COORDS
            | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::POSITION
            | VertexAttributeSet::TEXTURE_COORDS
            | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

        *output_render_attachment_quantities |= RenderAttachmentQuantitySet::NORMAL_VECTOR
            | RenderAttachmentQuantitySet::TEXTURE_COORDS;

        texture_shader_input.bump_mapping_input = Some(
            BumpMappingTextureShaderInput::ParallaxMapping(ParallaxMappingShaderInput {
                height_map_texture_and_sampler_bindings:
                    MaterialPropertyTextureManager::get_texture_and_sampler_bindings(
                        texture_ids.len(),
                    ),
            }),
        );

        texture_ids.push(parallax_map.height_map_texture_id);
    }

    if uses_specular_microfacet_model {
        material_name_parts.push("GGXAmbient");

        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::NORMAL_VECTOR;
        vertex_attribute_requirements_for_shader |=
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR;

        texture_shader_input.specular_reflectance_lookup_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );

        texture_ids.push(Assets::specular_ggx_reflectance_lookup_table_texture_id());
    }

    let material_id = MaterialID(hash64!(format!(
        "{}PrepassMaterial",
        material_name_parts.join("")
    )));

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements_for_mesh,
                vertex_attribute_requirements_for_shader,
                RenderAttachmentQuantitySet::empty(),
                *output_render_attachment_quantities,
                None,
                vec![feature_type_id],
                MaterialShaderInput::Prepass(texture_shader_input),
            )
        });

    let texture_set_id = if texture_ids.is_empty() {
        None
    } else {
        let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

        // Add a new texture set if none with the same textures already exist
        material_library
            .material_property_texture_set_entry(texture_set_id)
            .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

        Some(texture_set_id)
    };

    MaterialHandle::new(material_id, Some(feature_id), texture_set_id)
}
