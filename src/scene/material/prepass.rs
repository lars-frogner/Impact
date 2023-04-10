//! Materials for use in shading prepasses.

use super::{NormalMapComp, MATERIAL_VERTEX_BINDING_START};
use crate::{
    geometry::{InstanceFeature, InstanceFeatureID, VertexAttributeSet},
    impl_InstanceFeature,
    rendering::{
        fre, BumpMappingTextureShaderInput, InstanceFeatureShaderInput,
        MaterialPropertyTextureManager, MaterialShaderInput, NormalMappingShaderInput,
        ParallaxMappingShaderInput, PrepassFeatureShaderInput, PrepassTextureShaderInput,
        RenderAttachmentQuantitySet,
    },
    scene::{
        DiffuseColorComp, DiffuseTextureComp, InstanceFeatureManager, MaterialHandle, MaterialID,
        MaterialLibrary, MaterialPropertyTextureSet, MaterialPropertyTextureSetID,
        MaterialSpecification, ParallaxMapComp, RGBColor,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_utils::hash64;
use nalgebra::Vector2;

/// Fixed material properties for a uniformly diffuse prepass material.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffusePrepassMaterialFeature {
    diffuse_color: RGBColor,
}

/// Fixed material properties for a prepass material using parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct ParallaxMappingPrepassMaterialFeature {
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
}

/// Fixed material properties for a uniformly diffuse prepass material using
/// parallax mapping.
///
/// This type stores the material's per-instance data that will be sent to the
/// GPU. It implements [`InstanceFeature`], and can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct UniformDiffuseParallaxMappingPrepassMaterialFeature {
    diffuse_color: RGBColor,
    parallax_displacement_scale: fre,
    parallax_uv_per_distance: Vector2<fre>,
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
    output_render_attachment_quantities: &mut RenderAttachmentQuantitySet,
    diffuse_color: Option<&DiffuseColorComp>,
    diffuse_texture: Option<&DiffuseTextureComp>,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
) -> MaterialHandle {
    let mut material_name_parts = Vec::new();

    let mut vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
    let mut vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

    let mut texture_shader_input = PrepassTextureShaderInput {
        diffuse_texture_and_sampler_bindings: None,
        bump_mapping_input: None,
    };

    let (feature_type_ids, feature_id) = match (diffuse_color, parallax_map) {
        (None, None) => (Vec::new(), None),
        (Some(diffuse_color), None) => {
            material_name_parts.push("UniformDiffuse");

            (
                vec![UniformDiffusePrepassMaterialFeature::FEATURE_TYPE_ID],
                Some(UniformDiffusePrepassMaterialFeature::add_feature(
                    instance_feature_manager,
                    diffuse_color,
                )),
            )
        }
        (None, Some(parallax_map)) => {
            material_name_parts.push("ParallaxMapping");

            (
                vec![ParallaxMappingPrepassMaterialFeature::FEATURE_TYPE_ID],
                Some(ParallaxMappingPrepassMaterialFeature::add_feature(
                    instance_feature_manager,
                    parallax_map,
                )),
            )
        }
        (Some(diffuse_color), Some(parallax_map)) => {
            material_name_parts.push("UniformDiffuseParallaxMapping");

            (
                vec![UniformDiffuseParallaxMappingPrepassMaterialFeature::FEATURE_TYPE_ID],
                Some(
                    UniformDiffuseParallaxMappingPrepassMaterialFeature::add_feature(
                        instance_feature_manager,
                        diffuse_color,
                        parallax_map,
                    ),
                ),
            )
        }
    };

    let mut texture_ids = Vec::new();

    if let Some(diffuse_texture) = diffuse_texture {
        assert!(
            diffuse_color.is_none(),
            "Tried to create prepass material with both uniform and textured diffuse color"
        );

        material_name_parts.push("TexturedDiffuse");

        vertex_attribute_requirements_for_shader |= VertexAttributeSet::TEXTURE_COORDS;
        vertex_attribute_requirements_for_mesh |= VertexAttributeSet::TEXTURE_COORDS;

        texture_shader_input.diffuse_texture_and_sampler_bindings = Some(
            MaterialPropertyTextureManager::get_texture_and_sampler_bindings(texture_ids.len()),
        );
        texture_ids.push(diffuse_texture.0);
    }

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
                feature_type_ids,
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

    MaterialHandle::new(material_id, feature_id, texture_set_id)
}

impl UniformDiffusePrepassMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for UniformDiffusePrepassMaterialFeature features")
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
            })
    }
}

impl ParallaxMappingPrepassMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect("Missing storage for ParallaxMappingPrepassMaterialFeature features")
            .add_feature(&Self {
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl UniformDiffuseParallaxMappingPrepassMaterialFeature {
    fn add_feature(
        instance_feature_manager: &mut InstanceFeatureManager,
        diffuse_color: &DiffuseColorComp,
        parallax_map: &ParallaxMapComp,
    ) -> InstanceFeatureID {
        instance_feature_manager
            .get_storage_mut::<Self>()
            .expect(
                "Missing storage for UniformDiffuseParallaxMappingPrepassMaterialFeature features",
            )
            .add_feature(&Self {
                diffuse_color: diffuse_color.0,
                parallax_displacement_scale: parallax_map.displacement_scale,
                parallax_uv_per_distance: parallax_map.uv_per_distance,
            })
    }
}

impl_InstanceFeature!(
    UniformDiffusePrepassMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
    ],
    InstanceFeatureShaderInput::PrepassMaterial(PrepassFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        parallax_displacement_scale_location: None,
        parallax_uv_per_distance_location: None,
    })
);

impl_InstanceFeature!(
    ParallaxMappingPrepassMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32x2
    ],
    InstanceFeatureShaderInput::PrepassMaterial(PrepassFeatureShaderInput {
        diffuse_color_location: None,
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
    })
);

impl_InstanceFeature!(
    UniformDiffuseParallaxMappingPrepassMaterialFeature,
    wgpu::vertex_attr_array![
        MATERIAL_VERTEX_BINDING_START => Float32x3,
        MATERIAL_VERTEX_BINDING_START + 1 => Float32,
        MATERIAL_VERTEX_BINDING_START + 2 => Float32x2
    ],
    InstanceFeatureShaderInput::PrepassMaterial(PrepassFeatureShaderInput {
        diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
        parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
        parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
    })
);
