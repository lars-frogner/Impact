//! Shader template for model geometry passes.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantitySet},
    },
    material::{
        entity::physical::{
            PhysicalMaterialBumpMappingTextureBindings, PhysicalMaterialTextureBindings,
        },
        MaterialInstanceFeatureFlags, MaterialInstanceFeatureLocation, MaterialShaderInput,
        MaterialSpecification,
    },
    mesh::{buffer::MeshVertexAttributeLocation, VertexAttributeSet},
    model::transform::InstanceModelViewTransformWithPrevious,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Input for a specific instance of the model geometry shader template.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelGeometryShaderInput {
    /// The set of vertex attributes the model has.
    pub vertex_attributes: VertexAttributeSet,
    /// The instance feature flags for the model's material.
    pub material_instance_feature_flags: MaterialInstanceFeatureFlags,
    /// The texture bindings for the model's material (which should be
    /// physically based).
    pub material_texture_bindings: PhysicalMaterialTextureBindings,
}

/// Shader template for model geometry passes, which extract the relevant
/// geometrical information and material properties from the visible model
/// instances and write them to the corresponding render attachments (the
/// G-buffer).
#[derive(Clone, Debug)]
pub struct ModelGeometryShaderTemplate {
    input: ModelGeometryShaderInput,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("model_geometry")).unwrap());

impl ModelGeometryShaderInput {
    /// Returns the model geometry shader input corresponding to the given
    /// material specification, or [`None`] if the material is not compatible
    /// with the geometry shader (it is not physically based).
    pub fn for_material(specification: &MaterialSpecification) -> Option<Self> {
        if let MaterialShaderInput::Physical(material_texture_bindings) =
            &specification.shader_input()
        {
            Some(Self {
                vertex_attributes: specification.vertex_attribute_requirements(),
                material_instance_feature_flags: specification.instance_feature_flags(),
                material_texture_bindings: material_texture_bindings.clone(),
            })
        } else {
            None
        }
    }
}

impl ModelGeometryShaderTemplate {
    /// Creates a new model geometry shader template for the given input.
    pub fn new(input: ModelGeometryShaderInput) -> Self {
        Self { input }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> PushConstantGroup {
        PushConstantGroup::for_vertex_fragment([
            PushConstantVariant::InverseWindowDimensions,
            PushConstantVariant::FrameCounter,
        ])
    }

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    pub fn output_render_attachments() -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::with_defaults(RenderAttachmentQuantitySet::g_buffer())
    }

    /// Returns the input used for this instance of the shader template.
    pub fn input(&self) -> &ModelGeometryShaderInput {
        &self.input
    }
}

impl SpecificShaderTemplate for ModelGeometryShaderTemplate {
    fn resolve(&self) -> String {
        let mut flags_to_set = Vec::new();

        let mut replacements =
            template_replacements!(
                "jitter_count" => CameraProjectionUniform::jitter_count(),
                "model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::current_rotation_location(),
                "model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::current_translation_and_scaling_location(),
                "previous_model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::previous_rotation_location(),
                "previous_model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::previous_translation_and_scaling_location(),
                "specular_reflectance_location" => MaterialInstanceFeatureLocation::SpecularReflectance as u32,
                "roughness_location" => MaterialInstanceFeatureLocation::Roughness as u32,
                "metalness_location" => MaterialInstanceFeatureLocation::Metalness as u32,
                "emissive_luminance_location" => MaterialInstanceFeatureLocation::EmissiveLuminance as u32,
                "color_location" => MaterialInstanceFeatureLocation::Color as u32,
                "parallax_displacement_scale_location" => MaterialInstanceFeatureLocation::ParallaxDisplacementScale as u32,
                "parallax_uv_per_distance_location" => MaterialInstanceFeatureLocation::ParallaxUVPerDistance as u32,
                "position_location" => MeshVertexAttributeLocation::Position as u32,
                "normal_vector_location" => MeshVertexAttributeLocation::NormalVector as u32,
                "texture_coords_location" => MeshVertexAttributeLocation::TextureCoords as u32,
                "tangent_space_quaternion_location" => MeshVertexAttributeLocation::TangentSpaceQuaternion as u32,
                "projection_uniform_group" => 0,
                "projection_uniform_binding" => CameraProjectionUniform::binding(),
                "material_texture_group" => 1,
            ).to_vec();

        if self
            .input
            .vertex_attributes
            .contains(VertexAttributeSet::NORMAL_VECTOR)
        {
            flags_to_set.push("has_normal_vector");
        }

        if self
            .input
            .vertex_attributes
            .contains(VertexAttributeSet::TEXTURE_COORDS)
        {
            flags_to_set.push("has_texture_coords");
        }

        if self
            .input
            .vertex_attributes
            .contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION)
        {
            flags_to_set.push("has_tangent_space_quaternion");
        }

        if self
            .input
            .material_instance_feature_flags
            .contains(MaterialInstanceFeatureFlags::HAS_COLOR)
        {
            flags_to_set.push("has_color_value");
        }

        if self
            .input
            .material_instance_feature_flags
            .contains(MaterialInstanceFeatureFlags::USES_PARALLAX_MAPPING)
        {
            flags_to_set.push("uses_parallax_mapping");
        }

        if let Some((texture_binding, sampler_binding)) = self
            .input
            .material_texture_bindings
            .color_texture_and_sampler_bindings
        {
            flags_to_set.push("has_color_texture");
            replacements.push((
                "material_color_texture_binding",
                texture_binding.to_string(),
            ));
            replacements.push((
                "material_color_sampler_binding",
                sampler_binding.to_string(),
            ));
        }

        if let Some((texture_binding, sampler_binding)) = self
            .input
            .material_texture_bindings
            .specular_reflectance_texture_and_sampler_bindings
        {
            flags_to_set.push("has_specular_reflectance_texture");
            replacements.push((
                "specular_reflectance_texture_binding",
                texture_binding.to_string(),
            ));
            replacements.push((
                "specular_reflectance_sampler_binding",
                sampler_binding.to_string(),
            ));
        }

        if let Some((texture_binding, sampler_binding)) = self
            .input
            .material_texture_bindings
            .roughness_texture_and_sampler_bindings
        {
            flags_to_set.push("has_roughness_texture");
            replacements.push(("roughness_texture_binding", texture_binding.to_string()));
            replacements.push(("roughness_sampler_binding", sampler_binding.to_string()));
        }

        if let Some((texture_binding, sampler_binding)) = self
            .input
            .material_texture_bindings
            .metalness_texture_and_sampler_bindings
        {
            flags_to_set.push("has_metalness_texture");
            replacements.push(("metalness_texture_binding", texture_binding.to_string()));
            replacements.push(("metalness_sampler_binding", sampler_binding.to_string()));
        }

        if let Some((texture_binding, sampler_binding)) = self
            .input
            .material_texture_bindings
            .emissive_luminance_texture_and_sampler_bindings
        {
            flags_to_set.push("has_emissive_luminance_texture");
            replacements.push((
                "emissive_luminance_texture_binding",
                texture_binding.to_string(),
            ));
            replacements.push((
                "emissive_luminance_sampler_binding",
                sampler_binding.to_string(),
            ));
        }

        match self.input.material_texture_bindings.bump_mapping.as_ref() {
            Some(PhysicalMaterialBumpMappingTextureBindings::NormalMapping(bindings)) => {
                flags_to_set.push("uses_normal_mapping");
                replacements.push((
                    "normal_map_texture_binding",
                    bindings
                        .normal_map_texture_and_sampler_bindings
                        .0
                        .to_string(),
                ));
                replacements.push((
                    "normal_map_sampler_binding",
                    bindings
                        .normal_map_texture_and_sampler_bindings
                        .1
                        .to_string(),
                ));
            }
            Some(PhysicalMaterialBumpMappingTextureBindings::ParallaxMapping(bindings)) => {
                flags_to_set.push("uses_parallax_mapping");
                replacements.push((
                    "height_map_texture_binding",
                    bindings
                        .height_map_texture_and_sampler_bindings
                        .0
                        .to_string(),
                ));
                replacements.push((
                    "height_map_sampler_binding",
                    bindings
                        .height_map_texture_and_sampler_bindings
                        .1
                        .to_string(),
                ));
            }
            None => {}
        }

        TEMPLATE
            .resolve(flags_to_set, replacements)
            .expect("Shader template resolution failed")
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;
    use crate::material::entity::physical::{
        PhysicalMaterialNormalMappingTextureBindings,
        PhysicalMaterialParallaxMappingTextureBindings,
    };

    #[test]
    fn should_resolve_to_valid_wgsl_for_basic_input() {
        validate_template(&ModelGeometryShaderTemplate::new(
            ModelGeometryShaderInput {
                vertex_attributes: VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
                material_instance_feature_flags: MaterialInstanceFeatureFlags::HAS_COLOR,
                material_texture_bindings: PhysicalMaterialTextureBindings {
                    color_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                    metalness_texture_and_sampler_bindings: None,
                    emissive_luminance_texture_and_sampler_bindings: None,
                    bump_mapping: None,
                },
            },
        ));
    }

    #[test]
    fn should_resolve_to_valid_wgsl_for_fully_textured_input_with_normal_mapping() {
        validate_template(&ModelGeometryShaderTemplate::new(
            ModelGeometryShaderInput {
                vertex_attributes: VertexAttributeSet::POSITION
                    | VertexAttributeSet::TEXTURE_COORDS
                    | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
                material_instance_feature_flags: MaterialInstanceFeatureFlags::empty(),
                material_texture_bindings: PhysicalMaterialTextureBindings {
                    color_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: Some((4, 5)),
                    metalness_texture_and_sampler_bindings: Some((6, 7)),
                    emissive_luminance_texture_and_sampler_bindings: Some((8, 9)),
                    bump_mapping: Some(PhysicalMaterialBumpMappingTextureBindings::NormalMapping(
                        PhysicalMaterialNormalMappingTextureBindings {
                            normal_map_texture_and_sampler_bindings: (10, 11),
                        },
                    )),
                },
            },
        ));
    }

    #[test]
    fn should_resolve_to_valid_wgsl_for_fully_textured_input_with_parallax_mapping() {
        validate_template(&ModelGeometryShaderTemplate::new(
            ModelGeometryShaderInput {
                vertex_attributes: VertexAttributeSet::POSITION
                    | VertexAttributeSet::TEXTURE_COORDS
                    | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
                material_instance_feature_flags:
                    MaterialInstanceFeatureFlags::USES_PARALLAX_MAPPING,
                material_texture_bindings: PhysicalMaterialTextureBindings {
                    color_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: Some((4, 5)),
                    metalness_texture_and_sampler_bindings: Some((6, 7)),
                    emissive_luminance_texture_and_sampler_bindings: Some((8, 9)),
                    bump_mapping: Some(
                        PhysicalMaterialBumpMappingTextureBindings::ParallaxMapping(
                            PhysicalMaterialParallaxMappingTextureBindings {
                                height_map_texture_and_sampler_bindings: (10, 11),
                            },
                        ),
                    ),
                },
            },
        ));
    }
}
