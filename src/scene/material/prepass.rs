//! Materials for use in shading prepasses.

use super::{NormalMapComp, MATERIAL_VERTEX_BINDING_START};
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        create_uniform_buffer_bind_group_layout_entry, fre, BumpMappingShaderInput,
        GlobalAmbientColorShaderInput, InstanceFeatureShaderInput, MaterialPropertyTextureManager,
        MaterialShaderInput, NormalMappingShaderInput, ParallaxMappingFeatureShaderInput,
        ParallaxMappingShaderInput, RenderAttachmentQuantitySet, UniformBufferable,
    },
    scene::{
        FixedMaterialResources, InstanceFeatureManager, MaterialHandle, MaterialID,
        MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, ParallaxMapComp, RGBColor,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use nalgebra::Vector2;

/// Material with a fixed ambient color that is the same for all uses of the
/// material.
///
/// This object is intended to be stored in a uniform buffer, so it is padded to
/// 16 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct GlobalAmbientColorUniform {
    color: RGBColor,
    _padding: f32,
}

/// Fixed material properties for a prepass material using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct ParallaxMappingPrepassMaterialFeature {
    displacement_scale: fre,
    uv_per_distance: Vector2<fre>,
}

/// Determines the prepass material to use for the given combination of relevant
/// components, adds the material specification to the material library if it
/// does not already exist, then creates instance features and a texture set as
/// required and stores them in the instance feature manager and material
/// library.
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
    instance_feature_manager: &mut InstanceFeatureManager,
    material_library: &mut MaterialLibrary,
    ambient_color: RGBColor,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
) -> (MaterialHandle, RenderAttachmentQuantitySet) {
    let mut material_name_parts = vec!["GlobalAmbientColor"];

    let mut vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
    let mut vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

    let mut output_render_attachment_quantities = RenderAttachmentQuantitySet::empty();

    let mut bump_mapping_shader_input = None;

    let mut feature_type_ids = Vec::new();
    let mut feature_id = None;

    let mut texture_ids = Vec::new();

    match (normal_map, parallax_map) {
        (None, None) => {}
        (Some(normal_map), None) => {
            material_name_parts.push("NormalMapping");

            vertex_attribute_requirements_for_mesh |= VertexAttributeSet::NORMAL_VECTOR
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

            vertex_attribute_requirements_for_shader |=
                VertexAttributeSet::TEXTURE_COORDS | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

            output_render_attachment_quantities = RenderAttachmentQuantitySet::NORMAL_VECTOR;

            bump_mapping_shader_input = Some(BumpMappingShaderInput::NormalMapping(
                NormalMappingShaderInput {
                    normal_map_texture_and_sampler_bindings:
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(
                            texture_ids.len(),
                        ),
                },
            ));

            texture_ids.push(normal_map.0);
        }
        (None, Some(parallax_map)) => {
            material_name_parts.push("ParallaxMapping");

            vertex_attribute_requirements_for_mesh |= VertexAttributeSet::NORMAL_VECTOR
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

            vertex_attribute_requirements_for_shader |= VertexAttributeSet::POSITION
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION;

            output_render_attachment_quantities = RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::TEXTURE_COORDS;

            bump_mapping_shader_input = Some(BumpMappingShaderInput::ParallaxMapping(
                ParallaxMappingShaderInput {
                    height_map_texture_and_sampler_bindings:
                        MaterialPropertyTextureManager::get_texture_and_sampler_bindings(
                            texture_ids.len(),
                        ),
                },
            ));

            let feature = ParallaxMappingPrepassMaterialFeature {
                displacement_scale: parallax_map.displacement_scale,
                uv_per_distance: parallax_map.uv_per_distance,
            };

            feature_type_ids.push(ParallaxMappingPrepassMaterialFeature::FEATURE_TYPE_ID);

            feature_id = Some(
                instance_feature_manager
                    .get_storage_mut::<ParallaxMappingPrepassMaterialFeature>()
                    .expect("Missing storage for ParallaxMappingPrepassMaterialFeature")
                    .add_feature(&feature),
            );

            texture_ids.push(parallax_map.height_map_texture_id);
        }
        (Some(_), Some(_)) => {
            panic!("Tried to create prepass material that uses both normal mapping and parallax mapping");
        }
    }

    let material_id = MaterialID(hash64!(format!(
        "{}PrepassMaterial",
        material_name_parts.join("")
    )));

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            let fixed_resources = FixedMaterialResources::new(&GlobalAmbientColorUniform {
                color: ambient_color,
                _padding: 0.0,
            });

            MaterialSpecification::new(
                vertex_attribute_requirements_for_mesh,
                vertex_attribute_requirements_for_shader,
                RenderAttachmentQuantitySet::empty(),
                output_render_attachment_quantities,
                Some(fixed_resources),
                feature_type_ids,
                MaterialShaderInput::Prepass((
                    GlobalAmbientColorShaderInput {
                        uniform_binding: FixedMaterialResources::UNIFORM_BINDING,
                    },
                    bump_mapping_shader_input,
                )),
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

    (
        MaterialHandle::new(material_id, feature_id, texture_set_id),
        output_render_attachment_quantities,
    )
}

impl UniformBufferable for GlobalAmbientColorUniform {
    const ID: ConstStringHash64 = ConstStringHash64::new("GlobalAmbientColor");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}

impl_InstanceFeature!(
    ParallaxMappingPrepassMaterialFeature,
    wgpu::vertex_attr_array![MATERIAL_VERTEX_BINDING_START => Float32, MATERIAL_VERTEX_BINDING_START + 1 => Float32x2],
    InstanceFeatureShaderInput::ParallaxMappingPrepassMaterial(ParallaxMappingFeatureShaderInput {
        displacement_scale_location: MATERIAL_VERTEX_BINDING_START,
        uv_per_distance_location: MATERIAL_VERTEX_BINDING_START + 1,
    })
);
