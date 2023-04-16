//! Generation of shaders for Blinn-Phong materials.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
    insert_in_arena, new_name, push_to_block, InputStruct, InputStructBuilder,
    LightMaterialFeatureShaderInput, LightShaderGenerator, LightVertexOutputFieldIndices,
    MeshVertexOutputFieldIndices, OmnidirectionalLightShaderGenerator, OutputStructBuilder,
    PushConstantFieldExpressions, SampledTexture, SourceCode, TextureType,
    UnidirectionalLightShaderGenerator, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE, VECTOR_3_TYPE,
    VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use crate::rendering::{
    RenderAttachmentQuantity, RenderAttachmentQuantitySet, RENDER_ATTACHMENT_BINDINGS,
};
use naga::{Expression, Function, LocalVariable, MathFunction, Module, Statement};

/// Input description specifying the bindings of textures for Blinn-Phong
/// material properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and
    /// its sampler.
    pub diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular color texture and
    /// its sampler.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a Blinn-Phong material.
#[derive(Clone, Debug)]
pub struct BlinnPhongShaderGenerator<'a> {
    feature_input: &'a LightMaterialFeatureShaderInput,
    texture_input: &'a BlinnPhongTextureShaderInput,
}

/// Indices of the fields holding the various Blinn-Phong
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct BlinnPhongVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
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

        let input_diffuse_color_field_idx =
            self.feature_input.diffuse_color_location.map(|location| {
                input_struct_builder.add_field("diffuseColor", vec3_type, location, VECTOR_3_SIZE)
            });

        let input_specular_color_field_idx =
            self.feature_input.specular_color_location.map(|location| {
                input_struct_builder.add_field("specularColor", vec3_type, location, VECTOR_3_SIZE)
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
            diffuse_color: None,
            specular_color: None,
            shininess: output_shininess_field_idx,
        };

        if let Some(idx) = input_diffuse_color_field_idx {
            indices.diffuse_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "diffuseColor",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr(idx),
                ),
            );
        }

        if let Some(idx) = input_specular_color_field_idx {
            indices.specular_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "specularColor",
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
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &BlinnPhongVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let light_shader_generator =
            light_shader_generator.expect("Missing light for Blinn-Phong shading");

        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let screen_space_texture_coord_expr = if input_render_attachment_quantities.is_empty() {
            None
        } else {
            Some(source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertFramebufferPositionToScreenTextureCoords",
                vec![
                        push_constant_fragment_expressions.inverse_window_dimensions,
                        fragment_input_struct
                            .get_field_expr(mesh_input_field_indices.framebuffer_position),
                    ],
            ))
        };

        let position_expr =
            if input_render_attachment_quantities.contains(RenderAttachmentQuantitySet::POSITION) {
                let (position_texture_binding, position_sampler_binding) =
                    RENDER_ATTACHMENT_BINDINGS[RenderAttachmentQuantity::Position as usize];

                let position_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image2D,
                    "position",
                    *bind_group_idx,
                    position_texture_binding,
                    Some(position_sampler_binding),
                    None,
                );

                *bind_group_idx += 1;

                position_texture.generate_rgb_sampling_expr(
                    fragment_function,
                    screen_space_texture_coord_expr.unwrap(),
                )
            } else {
                fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .position
                        .expect("Missing position for Blinn-Phong shading"),
                )
            };

        let normal_vector_expr = if input_render_attachment_quantities
            .contains(RenderAttachmentQuantitySet::NORMAL_VECTOR)
        {
            let (normal_vector_texture_binding, normal_vector_sampler_binding) =
                RENDER_ATTACHMENT_BINDINGS[RenderAttachmentQuantity::NormalVector as usize];

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

        let (material_texture_bind_group, texture_coord_expr) = if !self.texture_input.is_empty() {
            let texture_coord_expr = if input_render_attachment_quantities
                .contains(RenderAttachmentQuantitySet::TEXTURE_COORDS)
            {
                let (texture_coord_texture_binding, texture_coord_sampler_binding) =
                    RENDER_ATTACHMENT_BINDINGS[RenderAttachmentQuantity::TextureCoords as usize];

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
                )
            } else {
                fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .texture_coords
                        .expect("Missing texture coordinates for Blinn-Phong shading"),
                )
            };

            let material_texture_bind_group = *bind_group_idx;
            *bind_group_idx += 1;

            (material_texture_bind_group, Some(texture_coord_expr))
        } else {
            (*bind_group_idx, None)
        };

        let diffuse_color_expr = self
            .texture_input
            .diffuse_texture_and_sampler_bindings
            .map(|(diffuse_texture_binding, diffuse_sampler_binding)| {
                let diffuse_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image2D,
                    "diffuseColor",
                    material_texture_bind_group,
                    diffuse_texture_binding,
                    Some(diffuse_sampler_binding),
                    None,
                );

                diffuse_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap())
            })
            .or_else(|| {
                material_input_field_indices
                    .diffuse_color
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let specular_color_expr = self
            .texture_input
            .specular_texture_and_sampler_bindings
            .map(|(specular_texture_binding, specular_sampler_binding)| {
                let specular_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image2D,
                    "specularColor",
                    material_texture_bind_group,
                    specular_texture_binding,
                    Some(specular_sampler_binding),
                    None,
                );

                specular_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap())
            })
            .or_else(|| {
                material_input_field_indices
                    .specular_color
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let shininess_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.shininess);

        let color_ptr_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut fragment_function.local_variables,
                LocalVariable {
                    name: new_name("color"),
                    ty: vec3_type,
                    init: None,
                },
            )),
        );

        let view_dir_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeCameraSpaceViewDirection",
            vec![position_expr],
        );

        let (reflection_dot_products_expr, light_radiance_expr) = match (
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

        let light_color_expr = match (diffuse_color_expr, specular_color_expr) {
            (Some(diffuse_color_expr), None) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeDiffuseBlinnPhongColor",
                vec![
                    reflection_dot_products_expr,
                    diffuse_color_expr,
                    light_radiance_expr,
                ],
            ),
            (None, Some(specular_color_expr)) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeSpecularBlinnPhongColor",
                vec![
                    reflection_dot_products_expr,
                    specular_color_expr,
                    shininess_expr,
                    light_radiance_expr,
                ],
            ),
            (Some(diffuse_color_expr), Some(specular_color_expr)) => source_code_lib
                .generate_function_call(
                    module,
                    fragment_function,
                    "computeBlinnPhongColor",
                    vec![
                        reflection_dot_products_expr,
                        diffuse_color_expr,
                        specular_color_expr,
                        shininess_expr,
                        light_radiance_expr,
                    ],
                ),
            (None, None) => panic!("No diffuse or specular color for Blinn-Phong shader"),
        };

        push_to_block(
            &mut fragment_function.body,
            Statement::Store {
                pointer: color_ptr_expr,
                value: light_color_expr,
            },
        );

        let output_color_expr = emit_in_func(fragment_function, |function| {
            let color_expr = include_expr_in_func(
                function,
                Expression::Load {
                    pointer: color_ptr_expr,
                },
            );

            include_expr_in_func(
                function,
                Expression::Math {
                    fun: MathFunction::Saturate,
                    arg: color_expr,
                    arg1: None,
                    arg2: None,
                    arg3: None,
                },
            )
        });

        let output_rgba_color_expr = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            output_color_expr,
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            output_rgba_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}

impl BlinnPhongTextureShaderInput {
    fn is_empty(&self) -> bool {
        self.diffuse_texture_and_sampler_bindings.is_none()
            && self.specular_texture_and_sampler_bindings.is_none()
    }
}
