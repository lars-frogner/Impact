//! Generation of shaders executed as preparation for a main shading pass.

use super::{
    super::{
        append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
        include_named_expr_in_func, insert_in_arena, swizzle_z_expr, InputStruct,
        InputStructBuilder, LightMaterialFeatureShaderInput, OutputStructBuilder, SampledTexture,
        SourceCode, TextureType, F32_TYPE, F32_WIDTH, VECTOR_2_SIZE, VECTOR_2_TYPE, VECTOR_3_SIZE,
        VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    CameraProjectionVariable, LightShaderGenerator, MeshVertexOutputFieldIndices,
    PushConstantExpressions,
};
use crate::gpu::{
    push_constant::PushConstantVariant, texture::attachment::RenderAttachmentQuantitySet,
};
use naga::{BinaryOperator, Expression, Function, Handle, Module, SampleLevel};

/// Input description specifying the bindings of textures for prepass material
/// properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrepassTextureShaderInput {
    /// Bind group bindings of the albedo texture and its sampler.
    pub albedo_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular reflectance texture and its sampler.
    pub specular_reflectance_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the roughness texture and its sampler.
    pub roughness_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the lookup table texture for specular reflectance
    /// and its sampler.
    pub specular_reflectance_lookup_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub bump_mapping_input: Option<BumpMappingTextureShaderInput>,
}

/// Input description for a material performing some form of bump mapping.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BumpMappingTextureShaderInput {
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
pub(super) struct PrepassShaderGenerator<'a> {
    feature_input: &'a LightMaterialFeatureShaderInput,
    texture_input: &'a PrepassTextureShaderInput,
}

/// Indices of the fields holding the various prepass properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub(super) struct PrepassVertexOutputFieldIndices {
    albedo: Option<usize>,
    specular_reflectance: Option<usize>,
    emissive_luminance: Option<usize>,
    roughness: Option<usize>,
    parallax_displacement_scale: Option<usize>,
    parallax_uv_per_distance: Option<usize>,
}

impl<'a> PrepassShaderGenerator<'a> {
    /// Creates a new shader generator using the given input descriptions.
    pub fn new(
        feature_input: &'a LightMaterialFeatureShaderInput,
        texture_input: &'a PrepassTextureShaderInput,
    ) -> Self {
        Self {
            feature_input,
            texture_input,
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
        let float_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("MaterialProperties", "material");

        let mut has_material_property = false;

        let input_albedo_field_idx = self.feature_input.albedo_location.map(|location| {
            has_material_property = true;
            input_struct_builder.add_field("albedo", vec3_type, location, VECTOR_3_SIZE)
        });

        let input_specular_reflectance_field_idx = self
            .feature_input
            .specular_reflectance_location
            .map(|location| {
                has_material_property = true;
                input_struct_builder.add_field(
                    "normalIncidenceSpecularReflectance",
                    vec3_type,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_emissive_luminance_field_idx = self
            .feature_input
            .emissive_luminance_location
            .map(|location| {
                has_material_property = true;
                input_struct_builder.add_field(
                    "emissiveLuminance",
                    vec3_type,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_roughness_field_idx = self.feature_input.roughness_location.map(|location| {
            has_material_property = true;
            input_struct_builder.add_field("roughness", float_type, location, F32_WIDTH)
        });

        let input_parallax_displacement_scale_field_idx = self
            .feature_input
            .parallax_displacement_scale_location
            .map(|location| {
                has_material_property = true;
                input_struct_builder.add_field(
                    "parallaxDisplacementScale",
                    float_type,
                    location,
                    F32_WIDTH,
                )
            });

        let input_parallax_uv_per_distance_field_idx = self
            .feature_input
            .parallax_uv_per_distance_location
            .map(|location| {
                has_material_property = true;
                input_struct_builder.add_field(
                    "parallaxUVPerDistance",
                    vec2_type,
                    location,
                    VECTOR_2_SIZE,
                )
            });

        let mut indices = PrepassVertexOutputFieldIndices {
            albedo: None,
            specular_reflectance: None,
            emissive_luminance: None,
            roughness: None,
            parallax_displacement_scale: None,
            parallax_uv_per_distance: None,
        };

        if has_material_property {
            let input_struct =
                input_struct_builder.generate_input_code(&mut module.types, vertex_function);

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
                        "normalIncidenceSpecularReflectance",
                        vec3_type,
                        VECTOR_3_SIZE,
                        input_struct.get_field_expr(idx),
                    ),
                );
            }

            if let Some(idx) = input_emissive_luminance_field_idx {
                indices.emissive_luminance = Some(
                    vertex_output_struct_builder.add_field_with_perspective_interpolation(
                        "emissiveLuminance",
                        vec3_type,
                        VECTOR_3_SIZE,
                        input_struct.get_field_expr(idx),
                    ),
                );
            }

            if let Some(idx) = input_roughness_field_idx {
                indices.roughness = Some(
                    vertex_output_struct_builder.add_field_with_perspective_interpolation(
                        "roughness",
                        float_type,
                        F32_WIDTH,
                        input_struct.get_field_expr(idx),
                    ),
                );
            };

            if let Some(idx) = input_parallax_displacement_scale_field_idx {
                indices.parallax_displacement_scale = Some(
                    vertex_output_struct_builder.add_field_with_perspective_interpolation(
                        "parallaxDisplacementScale",
                        float_type,
                        F32_WIDTH,
                        input_struct.get_field_expr(idx),
                    ),
                );
            }

            if let Some(idx) = input_parallax_uv_per_distance_field_idx {
                indices.parallax_uv_per_distance = Some(
                    vertex_output_struct_builder.add_field_with_perspective_interpolation(
                        "parallaxUVPerDistance",
                        vec2_type,
                        VECTOR_2_SIZE,
                        input_struct.get_field_expr(idx),
                    ),
                );
            }
        }

        indices
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The ambient reflected luminance is computed based on the albedo and
    /// specular reflectance and the ambient illuminance, and is returned from
    /// the function in an output struct. If the prepass involves normal or
    /// parallax mapping, the code for this is generated and the resulting
    /// quantities are included in the output struct. If there is an emissive
    /// luminance, this is also returned directly in the output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constant_fragment_expressions: &PushConstantExpressions,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &PrepassVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
        camera_projection: Option<&CameraProjectionVariable>,
    ) {
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let material_texture_bind_group = if !self.texture_input.is_empty() {
            let bind_group = *bind_group_idx;
            *bind_group_idx += 1;
            bind_group
        } else {
            *bind_group_idx
        };

        let texture_coord_expr = mesh_input_field_indices
            .texture_coords
            .map(|idx| fragment_input_struct.get_field_expr(idx));

        let (normal_vector_expr, texture_coord_expr) =
            generate_normal_vector_and_texture_coord_expr(
                module,
                source_code_lib,
                fragment_function,
                fragment_input_struct,
                mesh_input_field_indices,
                self.texture_input.bump_mapping_input.as_ref(),
                material_input_field_indices.parallax_displacement_scale,
                material_input_field_indices.parallax_uv_per_distance,
                material_texture_bind_group,
                texture_coord_expr,
            );

        let ambient_reflected_luminance_expr = match light_shader_generator {
            None => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "getBlackColor",
                Vec::new(),
            ),
            Some(LightShaderGenerator::AmbientLight(ambient_light_shader_generator)) => {
                let albedo_expr = self
                    .texture_input
                    .albedo_texture_and_sampler_bindings
                    .map(|(albedo_texture_binding, diffuse_sampler_binding)| {
                        let albedo_texture = SampledTexture::declare(
                            &mut module.types,
                            &mut module.global_variables,
                            TextureType::Image2D,
                            "albedo",
                            material_texture_bind_group,
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

                let diffuse_ambient_reflected_luminance = albedo_expr.map(|albedo_expr| {
                    source_code_lib.generate_function_call(
                        module,
                        fragment_function,
                        "computePreExposedAmbientReflectedLuminanceForLambertian",
                        vec![
                            albedo_expr,
                            ambient_light_shader_generator.luminance,
                            push_constant_fragment_expressions
                                .get(PushConstantVariant::Exposure)
                                .expect("Missing exposure push constant for prepass shader"),
                        ],
                    )
                });

                let specular_ambient_reflected_luminance = self
                    .texture_input
                    .specular_reflectance_lookup_texture_and_sampler_bindings
                    .map(
                        |(
                            specular_reflectance_texture_binding,
                            specular_reflectance_sampler_binding,
                        )| {
                            let specular_reflectance_lookup_texture = SampledTexture::declare(
                                &mut module.types,
                                &mut module.global_variables,
                                TextureType::Image2DArray,
                                "specularReflectanceLookup",
                                material_texture_bind_group,
                                specular_reflectance_texture_binding,
                                Some(specular_reflectance_sampler_binding),
                                None,
                            );

                            let (
                                specular_reflectance_lookup_texture_expr,
                                specular_reflectance_lookup_sampler_expr,
                            ) = specular_reflectance_lookup_texture
                                .generate_texture_and_sampler_expressions(fragment_function, false);

                            let specular_reflectance_expr = self
                                .texture_input
                                .specular_reflectance_texture_and_sampler_bindings
                                .map(|(specular_reflectance_texture_binding, specular_sampler_binding)| {
                                    let specular_reflectance_texture = SampledTexture::declare(
                                        &mut module.types,
                                        &mut module.global_variables,
                                        TextureType::Image2D,
                                        "normalIncidenceSpecularReflectance",
                                        material_texture_bind_group,
                                        specular_reflectance_texture_binding,
                                        Some(specular_sampler_binding),
                                        None,
                                    );

                                    specular_reflectance_texture.generate_rgb_sampling_expr(
                                        fragment_function,
                                        texture_coord_expr.unwrap(),SampleLevel::Auto
                                    )
                                })
                                .or_else(|| {
                                    material_input_field_indices
                                        .specular_reflectance
                                        .map(|idx| fragment_input_struct.get_field_expr(idx))
                                });

                            let fixed_roughness_value_expr = fragment_input_struct.get_field_expr(
                                material_input_field_indices
                                    .roughness
                                    .expect("Missing roughness for computing specular ambient reflected luminance"),
                            );

                            let roughness_expr = self
                                .texture_input
                                .roughness_texture_and_sampler_bindings
                                .map_or(
                                    fixed_roughness_value_expr,
                                    |(roughness_texture_binding, roughness_sampler_binding)| {
                                        let roughness_texture = SampledTexture::declare(
                                            &mut module.types,
                                            &mut module.global_variables,
                                            TextureType::Image2D,
                                            "roughness",
                                            material_texture_bind_group,
                                            roughness_texture_binding,
                                            Some(roughness_sampler_binding),
                                            None,
                                        );

                                        let roughness_texture_value_expr = roughness_texture
                                            .generate_single_channel_sampling_expr(
                                                fragment_function,
                                                texture_coord_expr.unwrap(),SampleLevel::Auto,
                                                0,
                                            );

                                        source_code_lib.generate_function_call(
                                            module,
                                            fragment_function,
                                            "computeGGXRoughnessFromSampledRoughness",
                                            // Use fixed roughness as scale for roughness sampled from texture
                                            vec![
                                                roughness_texture_value_expr,
                                                fixed_roughness_value_expr,
                                            ],
                                        )
                                    },
                                );

                            let position_expr = fragment_input_struct.get_field_expr(
                                mesh_input_field_indices.position.expect(
                                    "Missing position for computing specular ambient reflected luminance",
                                ),
                            );

                            let view_dir_expr = source_code_lib.generate_function_call(
                                module,
                                fragment_function,
                                "computeCameraSpaceViewDirection",
                                vec![position_expr],
                            );

                            source_code_lib.generate_function_call(
                                module,
                                fragment_function,
                                "computePreExposedAmbientReflectedLuminanceForSpecularGGX",
                                vec![
                                    specular_reflectance_lookup_texture_expr,
                                    specular_reflectance_lookup_sampler_expr,
                                    view_dir_expr,
                                    normal_vector_expr.expect("Missing normal vector for computing specular ambient reflected luminance"),
                                    specular_reflectance_expr.expect("Missing specular reflectance for computing specular ambient reflected luminance"),
                                    roughness_expr,
                                    ambient_light_shader_generator.luminance,
                                    push_constant_fragment_expressions
                                        .get(PushConstantVariant::Exposure)
                                        .expect("Missing exposure push constant for prepass shader"),
                                ],
                            )
                        },
                    );

                match (
                    diffuse_ambient_reflected_luminance,
                    specular_ambient_reflected_luminance,
                ) {
                    (None, None) => source_code_lib.generate_function_call(
                        module,
                        fragment_function,
                        "getBlackColor",
                        Vec::new(),
                    ),
                    (Some(diffuse_ambient_reflected_luminance), None) => {
                        diffuse_ambient_reflected_luminance
                    }
                    (None, Some(specular_ambient_reflected_luminance)) => {
                        specular_ambient_reflected_luminance
                    }
                    (
                        Some(diffuse_ambient_reflected_luminance),
                        Some(specular_ambient_reflected_luminance),
                    ) => emit_in_func(fragment_function, |function| {
                        include_expr_in_func(
                            function,
                            Expression::Binary {
                                op: BinaryOperator::Add,
                                left: diffuse_ambient_reflected_luminance,
                                right: specular_ambient_reflected_luminance,
                            },
                        )
                    }),
                }
            }
            Some(invalid_shader_generator) => {
                panic!(
                    "Invalid light type for prepass material: {:?}",
                    invalid_shader_generator
                );
            }
        };

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::LINEAR_DEPTH) {
            let position_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .position
                    .expect("Missing position for writing linear depth to render attachment"),
            );

            let inverse_far_plane_z_expr = camera_projection
                .expect("Missing camera projection computing normalized linear depth")
                .generate_inverse_far_plane_z_expr(fragment_function);

            let linear_depth_expr = emit_in_func(fragment_function, |function| {
                let position_z_expr = include_expr_in_func(function, swizzle_z_expr(position_expr));

                include_named_expr_in_func(
                    function,
                    "linearDepth",
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: inverse_far_plane_z_expr,
                        right: position_z_expr,
                    },
                )
            });

            output_struct_builder.add_field(
                "linearDepth",
                f32_type,
                None,
                None,
                F32_WIDTH,
                linear_depth_expr,
            );
        }

        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::NORMAL_VECTOR)
        {
            let normal_vector_expr =
                normal_vector_expr.expect("Missing normal vector for writing to render attachment");

            let normal_color_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertNormalVectorToNormalColor",
                vec![normal_vector_expr],
            );

            let normal_rgba_color_expr = append_unity_component_to_vec3(
                &mut module.types,
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

        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::TEXTURE_COORDS)
        {
            let texture_coord_expr = texture_coord_expr
                .expect("Missing texture coordinates for writing to render attachment");

            output_struct_builder.add_field(
                "textureCoords",
                vec2_type,
                None,
                None,
                VECTOR_2_SIZE,
                texture_coord_expr,
            );
        }

        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::MOTION_VECTOR)
        {
            let framebuffer_position_expr =
                fragment_input_struct.get_field_expr(mesh_input_field_indices.framebuffer_position);

            let inverse_window_dimensions_expr = push_constant_fragment_expressions
                .get(PushConstantVariant::InverseWindowDimensions)
                .expect("Missing inverse window dimensions push constant for computing motion vector in shading prepass");

            let screen_space_texture_coord_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertFramebufferPositionToScreenTextureCoords",
                vec![inverse_window_dimensions_expr, framebuffer_position_expr],
            );

            let motion_vector_expr = if let Some(previous_clip_space_position_expr) =
                mesh_input_field_indices
                    .previous_clip_space_position
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            {
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "computeMotionVector",
                    vec![
                        screen_space_texture_coord_expr,
                        previous_clip_space_position_expr,
                    ],
                )
            } else {
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "zeroMotionVector",
                    vec![],
                )
            };

            output_struct_builder.add_field(
                "motionVector",
                vec2_type,
                None,
                None,
                VECTOR_2_SIZE,
                motion_vector_expr,
            );
        }

        // Write ambient reflected luminance to the ambient reflected luminance
        // attachment (will be used when applying ambient occlusion).
        if output_render_attachment_quantities
            .contains(RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE)
        {
            let output_ambient_reflected_luminance_expr = append_unity_component_to_vec3(
                &mut module.types,
                fragment_function,
                ambient_reflected_luminance_expr,
            );

            output_struct_builder.add_field(
                "ambientReflectedLuminance",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                output_ambient_reflected_luminance_expr,
            );
        }

        // Write emissive luminance to the emissive luminance attachment.
        if let Some(emissive_luminance_idx) = material_input_field_indices.emissive_luminance {
            let emissive_luminance_expr =
                fragment_input_struct.get_field_expr(emissive_luminance_idx);

            let pre_exposed_emissive_luminance_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "preExposeEmissiveLuminance",
                vec![
                    emissive_luminance_expr,
                    push_constant_fragment_expressions
                        .get(PushConstantVariant::Exposure)
                        .expect("Missing exposure push constant for prepass shader"),
                ],
            );

            let output_emissive_luminance_expr = append_unity_component_to_vec3(
                &mut module.types,
                fragment_function,
                pre_exposed_emissive_luminance_expr,
            );

            output_struct_builder.add_field(
                "emissiveLuminance",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                output_emissive_luminance_expr,
            );
        // If we do not write an emissive luminance, we need to write a clear
        // color in case we are obscuring an emissive object, otherwise the
        // emissive object will shine through
        } else {
            let clear_color_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "getBlackColor",
                Vec::new(),
            );

            let clear_rgba_color_expr = append_unity_component_to_vec3(
                &mut module.types,
                fragment_function,
                clear_color_expr,
            );

            output_struct_builder.add_field(
                "clearColor",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                clear_rgba_color_expr,
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
    bump_mapping_input: Option<&BumpMappingTextureShaderInput>,
    displacement_scale_idx: Option<usize>,
    uv_per_distance_idx: Option<usize>,
    bind_group: u32,
    texture_coord_expr: Option<Handle<Expression>>,
) -> (Option<Handle<Expression>>, Option<Handle<Expression>>) {
    match bump_mapping_input {
        None => (
            mesh_input_field_indices
                .normal_vector
                .map(|idx| fragment_input_struct.get_field_expr(idx)),
            texture_coord_expr,
        ),
        Some(BumpMappingTextureShaderInput::NormalMapping(normal_mapping_input)) => {
            let (normal_map_texture_binding, normal_map_sampler_binding) =
                normal_mapping_input.normal_map_texture_and_sampler_bindings;

            let normal_map_texture = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::Image2D,
                "normalMap",
                bind_group,
                normal_map_texture_binding,
                Some(normal_map_sampler_binding),
                None,
            );

            let normal_map_color_expr = normal_map_texture.generate_rgb_sampling_expr(
                fragment_function,
                texture_coord_expr.unwrap(),
                SampleLevel::Auto,
            );

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
                Some(source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "transformVectorFromTangentSpace",
                    vec![tangent_space_quaternion_expr, tangent_space_normal_expr],
                )),
                texture_coord_expr,
            )
        }
        Some(BumpMappingTextureShaderInput::ParallaxMapping(parallax_mapping_input)) => {
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
                TextureType::Image2D,
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
                Some(source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "transformVectorFromTangentSpace",
                    vec![
                        tangent_space_quaternion_expr,
                        tangent_space_normal_vector_expr,
                    ],
                )),
                Some(parallax_mapped_texture_coord_expr),
            )
        }
    }
}

impl PrepassTextureShaderInput {
    fn is_empty(&self) -> bool {
        self.albedo_texture_and_sampler_bindings.is_none()
            && self
                .specular_reflectance_texture_and_sampler_bindings
                .is_none()
            && self.roughness_texture_and_sampler_bindings.is_none()
            && self
                .specular_reflectance_lookup_texture_and_sampler_bindings
                .is_none()
            && self.bump_mapping_input.is_none()
    }
}
