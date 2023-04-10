//! Generation of shaders executed as preparation for a main shading pass.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
    include_named_expr_in_func, insert_in_arena, new_name, InputStruct, InputStructBuilder,
    MeshVertexOutputFieldIndices, OutputStructBuilder, SampledTexture, SourceCode, TextureType,
    F32_TYPE, F32_WIDTH, VECTOR_2_SIZE, VECTOR_2_TYPE, VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{AddressSpace, Expression, Function, GlobalVariable, Handle, Module, ResourceBinding};

/// Input description specifying the vertex attribute locations of parallax
/// mapping parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParallaxMappingFeatureShaderInput {
    /// Vertex attribute location for the instance feature representing the
    /// displacement scale.
    pub displacement_scale_location: u32,
    /// Vertex attribute location for the instance feature representing the
    /// change in UV texture coordinates per world space distance.
    pub uv_per_distance_location: u32,
}

/// Input description specifying the uniform binding reqired for shading with a
/// global ambient color.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalAmbientColorShaderInput {
    /// Bind group binding of the uniform buffer holding the global ambient color.
    pub uniform_binding: u32,
}

/// Input description for a material performing some form of bump mapping.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BumpMappingShaderInput {
    NormalMapping(NormalMappingShaderInput),
    ParallaxMapping(ParallaxMappingShaderInput),
}

/// Input description specifying the bindings of textures for normal mapping.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NormalMappingShaderInput {
    /// Bind group bindings of the normal map texture and its sampler.
    pub normal_map_texture_and_sampler_bindings: (u32, u32),
}

/// Input description specifying the bindings of textures for parallax mapping.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParallaxMappingShaderInput {
    /// Bind group bindings of the height map texture and its sampler.
    pub height_map_texture_and_sampler_bindings: (u32, u32),
}

/// Shader generator for a prepass material.
#[derive(Clone, Debug)]
pub struct PrepassShaderGenerator<'a> {
    global_ambient_color_input: &'a GlobalAmbientColorShaderInput,
    bump_mapping_input: Option<&'a BumpMappingShaderInput>,
    parallax_mapping_feature_input: Option<&'a ParallaxMappingFeatureShaderInput>,
}

/// Indices of the fields holding the various prepass properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct PrepassVertexOutputFieldIndices {
    displacement_scale: Option<usize>,
    uv_per_distance: Option<usize>,
}

impl<'a> PrepassShaderGenerator<'a> {
    /// Creates a new shader generator using the given input descriptions.
    pub fn new(
        global_ambient_color_input: &'a GlobalAmbientColorShaderInput,
        bump_mapping_input: Option<&'a BumpMappingShaderInput>,
        parallax_mapping_feature_input: Option<&'a ParallaxMappingFeatureShaderInput>,
    ) -> Self {
        Self {
            global_ambient_color_input,
            bump_mapping_input,
            parallax_mapping_feature_input,
        }
    }

    /// Generates the vertex shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The struct of vertex buffered prepass properties is added as an input
    /// argument to the main vertex shader function and its fields are assigned
    /// to the output struct returned from the function.
    ///
    /// # Returns
    /// The indices of the prepass property fields in the output struct,
    /// required for accessing the properties in [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> PrepassVertexOutputFieldIndices {
        let mut indices = PrepassVertexOutputFieldIndices {
            displacement_scale: None,
            uv_per_distance: None,
        };

        if let Some(parallax_mapping_feature_input) = self.parallax_mapping_feature_input {
            let float_type = insert_in_arena(&mut module.types, F32_TYPE);
            let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);

            let mut input_struct_builder =
                InputStructBuilder::new("MaterialProperties", "material");

            let input_displacement_scale_field_idx = input_struct_builder.add_field(
                "displacementScale",
                float_type,
                parallax_mapping_feature_input.displacement_scale_location,
                F32_WIDTH,
            );

            let input_uv_per_distance_field_idx = input_struct_builder.add_field(
                "uvPerDistance",
                vec2_type,
                parallax_mapping_feature_input.uv_per_distance_location,
                VECTOR_2_SIZE,
            );

            let input_struct =
                input_struct_builder.generate_input_code(&mut module.types, vertex_function);

            indices.displacement_scale = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "parallaxDisplacementScale",
                    float_type,
                    F32_WIDTH,
                    input_struct.get_field_expr(input_displacement_scale_field_idx),
                ),
            );

            indices.uv_per_distance = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "uvPerDistance",
                    vec2_type,
                    VECTOR_2_SIZE,
                    input_struct.get_field_expr(input_uv_per_distance_field_idx),
                ),
            );
        }

        indices
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The global ambient color is declared as a global uniform variable, and
    /// is returned from the function in an output struct. If the prepass
    /// involves normal or parallax mapping, the code for this is generated and
    /// the resulting quantities are included in the output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &PrepassVertexOutputFieldIndices,
    ) {
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let ambient_color_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("ambientColor"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: bind_group,
                    binding: self.global_ambient_color_input.uniform_binding,
                }),
                ty: vec3_type,
                init: None,
            },
        );

        let ambient_color_ptr_expr = include_expr_in_func(
            fragment_function,
            Expression::GlobalVariable(ambient_color_var),
        );

        let ambient_color_expr = emit_in_func(fragment_function, |function| {
            include_named_expr_in_func(
                function,
                "ambientColor",
                Expression::Load {
                    pointer: ambient_color_ptr_expr,
                },
            )
        });

        let (normal_vector_expr, texture_coord_expr) =
            if let Some(bump_mapping_input) = self.bump_mapping_input {
                let texture_coord_expr = fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .texture_coords
                        .expect("Missing texture coordinates for shading prepass"),
                );

                let bind_group = *bind_group_idx;
                *bind_group_idx += 1;

                let (normal_vector_expr, texture_coord_expr) =
                    generate_normal_vector_and_texture_coord_expr(
                        module,
                        source_code_lib,
                        fragment_function,
                        fragment_input_struct,
                        mesh_input_field_indices,
                        Some(bump_mapping_input),
                        material_input_field_indices.displacement_scale,
                        material_input_field_indices.uv_per_distance,
                        bind_group,
                        Some(texture_coord_expr),
                    );

                (Some(normal_vector_expr), texture_coord_expr)
            } else {
                (None, None)
            };

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        let ambient_rgba_color_expr = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            ambient_color_expr,
        );

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            ambient_rgba_color_expr,
        );

        if let Some(normal_vector_expr) = normal_vector_expr {
            let normal_color_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertNormalVectorToNormalColor",
                vec![normal_vector_expr],
            );

            let normal_rgba_color_expr = append_unity_component_to_vec3(
                &mut module.types,
                &mut module.constants,
                fragment_function,
                normal_color_expr,
            );

            output_struct_builder.add_field(
                "normalVector",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                normal_rgba_color_expr,
            );
        }

        if let Some(texture_coord_expr) = texture_coord_expr {
            output_struct_builder.add_field(
                "textureCoords",
                vec2_type,
                None,
                None,
                VECTOR_2_SIZE,
                texture_coord_expr,
            );
        }

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}

fn generate_normal_vector_and_texture_coord_expr(
    module: &mut Module,
    source_code_lib: &mut SourceCode,
    fragment_function: &mut Function,
    fragment_input_struct: &InputStruct,
    mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    bump_mapping_input: Option<&BumpMappingShaderInput>,
    displacement_scale_idx: Option<usize>,
    uv_per_distance_idx: Option<usize>,
    bind_group: u32,
    texture_coord_expr: Option<Handle<Expression>>,
) -> (Handle<Expression>, Option<Handle<Expression>>) {
    match bump_mapping_input {
        None => (
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "normalizeVector",
                vec![fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .normal_vector
                        .expect("Missing normal vector for shading prepass"),
                )],
            ),
            None,
        ),
        Some(BumpMappingShaderInput::NormalMapping(normal_mapping_input)) => {
            let (normal_map_texture_binding, normal_map_sampler_binding) =
                normal_mapping_input.normal_map_texture_and_sampler_bindings;

            let normal_map_texture = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::Image,
                "normalMap",
                bind_group,
                normal_map_texture_binding,
                Some(normal_map_sampler_binding),
                None,
            );

            let normal_map_color_expr = normal_map_texture
                .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap());

            let tangent_space_normal_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertNormalColorToNormalizedNormalVector",
                vec![normal_map_color_expr],
            );

            let unnormalized_tangent_space_quaternion_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .tangent_space_quaternion
                    .expect("Missing tangent space quaternion for normal mapping"),
            );

            let tangent_space_quaternion_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "normalizeQuaternion",
                vec![unnormalized_tangent_space_quaternion_expr],
            );

            (
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "transformVectorFromTangentSpace",
                    vec![tangent_space_quaternion_expr, tangent_space_normal_expr],
                ),
                texture_coord_expr,
            )
        }
        Some(BumpMappingShaderInput::ParallaxMapping(parallax_mapping_input)) => {
            let position_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .position
                    .expect("Missing position for parallax mapping"),
            );

            let view_dir_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeCameraSpaceViewDirection",
                vec![position_expr],
            );

            let (height_map_texture_binding, height_map_sampler_binding) =
                parallax_mapping_input.height_map_texture_and_sampler_bindings;

            let height_map_texture = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::Image,
                "heightMap",
                bind_group,
                height_map_texture_binding,
                Some(height_map_sampler_binding),
                None,
            );

            let (height_map_texture_expr, height_map_sampler_expr) = height_map_texture
                .generate_texture_and_sampler_expressions(fragment_function, false);

            let displacement_scale_expr = fragment_input_struct.get_field_expr(
                displacement_scale_idx.expect("Missing displacement scale for parallax mapping"),
            );

            let uv_per_distance_expr = fragment_input_struct.get_field_expr(
                uv_per_distance_idx.expect("Missing UV per distance for parallax mapping"),
            );

            let unnormalized_tangent_space_quaternion_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .tangent_space_quaternion
                    .expect("Missing tangent space quaternion for parallax mapping"),
            );

            let tangent_space_quaternion_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "normalizeQuaternion",
                vec![unnormalized_tangent_space_quaternion_expr],
            );

            let parallax_mapped_texture_coord_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeParallaxMappedTextureCoordinates",
                vec![
                    height_map_texture_expr,
                    height_map_sampler_expr,
                    displacement_scale_expr,
                    texture_coord_expr.unwrap(),
                    tangent_space_quaternion_expr,
                    view_dir_expr,
                ],
            );

            let tangent_space_normal_vector_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "obtainNormalFromHeightMap",
                vec![
                    height_map_texture_expr,
                    height_map_sampler_expr,
                    displacement_scale_expr,
                    uv_per_distance_expr,
                    parallax_mapped_texture_coord_expr,
                    position_expr,
                ],
            );

            (
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "transformVectorFromTangentSpace",
                    vec![
                        tangent_space_quaternion_expr,
                        tangent_space_normal_vector_expr,
                    ],
                ),
                Some(parallax_mapped_texture_coord_expr),
            )
        }
    }
}
