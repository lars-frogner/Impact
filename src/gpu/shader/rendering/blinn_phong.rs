//! Generation of shaders for Blinn-Phong materials.

use super::{
    super::{
        append_unity_component_to_vec3, insert_in_arena, InputStruct, InputStructBuilder,
        LightMaterialFeatureShaderInput, OutputStructBuilder, SampledTexture, SourceCode,
        TextureType, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE, VECTOR_3_TYPE, VECTOR_4_SIZE,
        VECTOR_4_TYPE,
    },
    LightShaderGenerator, LightVertexOutputFieldIndices, MeshVertexOutputFieldIndices,
    OmnidirectionalLightShaderGenerator, PushConstantExpressions,
    UnidirectionalLightShaderGenerator,
};
use crate::gpu::{
    push_constant::PushConstantVariant,
    texture::attachment::{RenderAttachmentQuantity, RenderAttachmentQuantitySet},
};
use naga::{Function, Module, SampleLevel};

/// Input description specifying the bindings of textures for Blinn-Phong
/// material properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongTextureShaderInput {
    /// Bind group bindings of the albedo texture and
    /// its sampler.
    pub albedo_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular reflectance texture and its sampler.
    pub specular_reflectance_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a Blinn-Phong material.
#[derive(Clone, Debug)]
pub(super) struct BlinnPhongShaderGenerator<'a> {
    feature_input: &'a LightMaterialFeatureShaderInput,
    texture_input: &'a BlinnPhongTextureShaderInput,
}

/// Indices of the fields holding the various Blinn-Phong
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub(super) struct BlinnPhongVertexOutputFieldIndices {
    albedo: Option<usize>,
    specular_reflectance: Option<usize>,
    shininess: usize,
}

impl<'a> BlinnPhongShaderGenerator<'a> {
    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(
        feature_input: &'a LightMaterialFeatureShaderInput,
        texture_input: &'a BlinnPhongTextureShaderInput,
    ) -> Self {
        Self {
            feature_input,
            texture_input,
        }
    }

    /// Generates the vertex shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The struct of vertex buffered Blinn-Phong properties is added
    /// as an input argument to the main vertex shader function and its
    /// fields are assigned to the output struct returned from the function.
    ///
    /// # Returns
    /// The indices of the Blinn-Phong property fields in the output
    /// struct, required for accessing the properties in
    /// [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> BlinnPhongVertexOutputFieldIndices {
        let float_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("MaterialProperties", "material");

        let input_albedo_field_idx = self.feature_input.albedo_location.map(|location| {
            input_struct_builder.add_field("albedo", vec3_type, location, VECTOR_3_SIZE)
        });

        let input_specular_reflectance_field_idx = self
            .feature_input
            .specular_reflectance_location
            .map(|location| {
                input_struct_builder.add_field(
                    "specularReflectance",
                    vec3_type,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_shininess_field_idx = input_struct_builder.add_field(
            "shininess",
            float_type,
            self.feature_input
                .roughness_location
                .expect("Missing shininess for Blinn-Phong shading"),
            F32_WIDTH,
        );

        let input_struct =
            input_struct_builder.generate_input_code(&mut module.types, vertex_function);

        let output_shininess_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "shininess",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_shininess_field_idx),
            );

        let mut indices = BlinnPhongVertexOutputFieldIndices {
            albedo: None,
            specular_reflectance: None,
            shininess: output_shininess_field_idx,
        };

        if let Some(idx) = input_albedo_field_idx {
            indices.albedo = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "albedo",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr(idx),
                ),
            );
        }

        if let Some(idx) = input_specular_reflectance_field_idx {
            indices.specular_reflectance = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "specularReflectance",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr(idx),
                ),
            );
        }

        indices
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The texture and sampler for any material properties sampled from
    /// textured are declared as global variables, and sampling expressions are
    /// generated in the main fragment shader function. These are used together
    /// with material properties passed from the main vertex shader to evaluate
    /// the Blinn-Phong shading equation for the active light, and the output
    /// color is returned from the main fragment shader function in an output
    /// struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constant_fragment_expressions: &PushConstantExpressions,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &BlinnPhongVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let light_shader_generator =
            light_shader_generator.expect("Missing light for Blinn-Phong shading");

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let screen_space_texture_coord_expr = if input_render_attachment_quantities.is_empty() {
            None
        } else {
            let inverse_window_dimensions_expr = push_constant_fragment_expressions
                .get(PushConstantVariant::InverseWindowDimensions)
                .expect("Missing inverse window dimensions push constant for Blinn-Phong shading");
            Some(source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertFramebufferPositionToScreenTextureCoords",
                vec![
                    inverse_window_dimensions_expr,
                    fragment_input_struct
                        .get_field_expr(mesh_input_field_indices.framebuffer_position),
                ],
            ))
        };

        // The bind group for material textures comes before the bind groups for
        // render attachments
        let material_texture_bind_group = if !self.texture_input.is_empty() {
            let material_texture_bind_group = *bind_group_idx;
            *bind_group_idx += 1;
            Some(material_texture_bind_group)
        } else {
            None
        };

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for Blinn-Phong shading"),
        );

        let normal_vector_expr = if input_render_attachment_quantities
            .contains(RenderAttachmentQuantitySet::NORMAL_VECTOR)
        {
            let (normal_vector_texture_binding, normal_vector_sampler_binding) =
                RenderAttachmentQuantity::NormalVector.bindings();

            let normal_vector_texture = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::Image2D,
                "normalVector",
                *bind_group_idx,
                normal_vector_texture_binding,
                Some(normal_vector_sampler_binding),
                None,
            );

            *bind_group_idx += 1;

            let normal_color_expr = normal_vector_texture.generate_rgb_sampling_expr(
                fragment_function,
                screen_space_texture_coord_expr.unwrap(),
                SampleLevel::Zero,
            );

            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertNormalColorToNormalizedNormalVector",
                vec![normal_color_expr],
            )
        } else {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "normalizeVector",
                vec![fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .normal_vector
                        .expect("Missing normal vector for Blinn-Phong shading"),
                )],
            )
        };

        let texture_coord_expr = if !self.texture_input.is_empty() {
            let texture_coord_expr = if input_render_attachment_quantities
                .contains(RenderAttachmentQuantitySet::TEXTURE_COORDS)
            {
                let (texture_coord_texture_binding, texture_coord_sampler_binding) =
                    RenderAttachmentQuantity::TextureCoords.bindings();

                let texture_coord_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image2D,
                    "textureCoord",
                    *bind_group_idx,
                    texture_coord_texture_binding,
                    Some(texture_coord_sampler_binding),
                    None,
                );

                *bind_group_idx += 1;

                texture_coord_texture.generate_rg_sampling_expr(
                    fragment_function,
                    screen_space_texture_coord_expr.unwrap(),
                    SampleLevel::Zero,
                )
            } else {
                fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .texture_coords
                        .expect("Missing texture coordinates for Blinn-Phong shading"),
                )
            };

            Some(texture_coord_expr)
        } else {
            None
        };

        let albedo_expr = self
            .texture_input
            .albedo_texture_and_sampler_bindings
            .map(|(albedo_texture_binding, diffuse_sampler_binding)| {
                let albedo_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image2D,
                    "albedo",
                    material_texture_bind_group.unwrap(),
                    albedo_texture_binding,
                    Some(diffuse_sampler_binding),
                    None,
                );

                albedo_texture.generate_rgb_sampling_expr(
                    fragment_function,
                    texture_coord_expr.unwrap(),
                    SampleLevel::Auto,
                )
            })
            .or_else(|| {
                material_input_field_indices
                    .albedo
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let specular_reflectance_expr = self
            .texture_input
            .specular_reflectance_texture_and_sampler_bindings
            .map(
                |(specular_reflectance_texture_binding, specular_sampler_binding)| {
                    let specular_reflectance_texture = SampledTexture::declare(
                        &mut module.types,
                        &mut module.global_variables,
                        TextureType::Image2D,
                        "specularReflectance",
                        material_texture_bind_group.unwrap(),
                        specular_reflectance_texture_binding,
                        Some(specular_sampler_binding),
                        None,
                    );

                    specular_reflectance_texture.generate_rgb_sampling_expr(
                        fragment_function,
                        texture_coord_expr.unwrap(),
                        SampleLevel::Auto,
                    )
                },
            )
            .or_else(|| {
                material_input_field_indices
                    .specular_reflectance
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let shininess_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.shininess);

        let view_dir_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeCameraSpaceViewDirection",
            vec![position_expr],
        );

        let (reflection_dot_products_expr, incident_luminance_expr) = match (
            light_shader_generator,
            light_input_field_indices,
        ) {
            (
                LightShaderGenerator::OmnidirectionalLight(
                    OmnidirectionalLightShaderGenerator::ForShading(
                        omnidirectional_light_shader_generator,
                    ),
                ),
                None,
            ) => {
                let framebuffer_position_expr = fragment_input_struct
                    .get_field_expr(mesh_input_field_indices.framebuffer_position);

                omnidirectional_light_shader_generator.generate_fragment_shading_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    push_constant_fragment_expressions,
                    framebuffer_position_expr,
                    position_expr,
                    normal_vector_expr,
                    view_dir_expr,
                    None,
                    false,
                )
            }
            (
                LightShaderGenerator::UnidirectionalLight(
                    UnidirectionalLightShaderGenerator::ForShading(
                        unidirectional_light_shader_generator,
                    ),
                ),
                Some(LightVertexOutputFieldIndices::UnidirectionalLight(
                    unidirectional_light_input_field_indices,
                )),
            ) => {
                let framebuffer_position_expr = fragment_input_struct
                    .get_field_expr(mesh_input_field_indices.framebuffer_position);

                unidirectional_light_shader_generator.generate_fragment_shading_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    fragment_input_struct,
                    unidirectional_light_input_field_indices,
                    push_constant_fragment_expressions,
                    framebuffer_position_expr,
                    normal_vector_expr,
                    view_dir_expr,
                    None,
                    false,
                )
            }
            _ => {
                panic!("Invalid variant of light shader generator and/or light vertex output field indices for Blinn-Phong shading");
            }
        };

        let reflected_luminance_expr = match (albedo_expr, specular_reflectance_expr) {
            (Some(albedo_expr), None) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeDiffuseBlinnPhongReflectedLuminance",
                vec![
                    reflection_dot_products_expr,
                    albedo_expr,
                    incident_luminance_expr,
                ],
            ),
            (None, Some(specular_reflectance_expr)) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeSpecularBlinnPhongReflectedLuminance",
                vec![
                    reflection_dot_products_expr,
                    specular_reflectance_expr,
                    shininess_expr,
                    incident_luminance_expr,
                ],
            ),
            (Some(albedo_expr), Some(specular_reflectance_expr)) => source_code_lib
                .generate_function_call(
                    module,
                    fragment_function,
                    "computeBlinnPhongReflectedLuminance",
                    vec![
                        reflection_dot_products_expr,
                        albedo_expr,
                        specular_reflectance_expr,
                        shininess_expr,
                        incident_luminance_expr,
                    ],
                ),
            (None, None) => panic!("No albedo or specular reflectance for Blinn-Phong shader"),
        };

        let output_reflected_luminance_expr = append_unity_component_to_vec3(
            &mut module.types,
            fragment_function,
            reflected_luminance_expr,
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "reflectedLuminance",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            output_reflected_luminance_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}

impl BlinnPhongTextureShaderInput {
    fn is_empty(&self) -> bool {
        self.albedo_texture_and_sampler_bindings.is_none()
            && self
                .specular_reflectance_texture_and_sampler_bindings
                .is_none()
    }
}
