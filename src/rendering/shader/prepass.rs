//! Generation of shaders executed as preparation for a main shading pass.

use super::{
    append_unity_component_to_vec3, emit_in_func, include_expr_in_func, insert_in_arena,
    InputStruct, InputStructBuilder, LightMaterialFeatureShaderInput, LightShaderGenerator,
    MeshVertexOutputFieldIndices, OutputStructBuilder, SampledTexture, SourceCode, TextureType,
    F32_TYPE, F32_WIDTH, VECTOR_2_SIZE, VECTOR_2_TYPE, VECTOR_3_SIZE, VECTOR_3_TYPE, VECTOR_4_SIZE,
    VECTOR_4_TYPE,
};
use crate::rendering::RenderAttachmentQuantitySet;
use naga::{BinaryOperator, Expression, Function, Handle, Module};

/// Input description specifying the bindings of textures for prepass material
/// properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrepassTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and its sampler.
    pub diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular color texture and its sampler.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
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
pub struct PrepassShaderGenerator<'a> {
    feature_input: &'a LightMaterialFeatureShaderInput,
    texture_input: &'a PrepassTextureShaderInput,
}

/// Indices of the fields holding the various prepass properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct PrepassVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    emissive_color: Option<usize>,
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

        let input_diffuse_color_field_idx =
            self.feature_input.diffuse_color_location.map(|location| {
                has_material_property = true;
                input_struct_builder.add_field("diffuseColor", vec3_type, location, VECTOR_3_SIZE)
            });

        let input_specular_color_field_idx =
            self.feature_input.specular_color_location.map(|location| {
                has_material_property = true;
                input_struct_builder.add_field("specularColor", vec3_type, location, VECTOR_3_SIZE)
            });

        let input_emissive_color_field_idx =
            self.feature_input.emissive_color_location.map(|location| {
                has_material_property = true;
                input_struct_builder.add_field("emissiveColor", vec3_type, location, VECTOR_3_SIZE)
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
            diffuse_color: None,
            specular_color: None,
            emissive_color: None,
            roughness: None,
            parallax_displacement_scale: None,
            parallax_uv_per_distance: None,
        };

        if has_material_property {
            let input_struct =
                input_struct_builder.generate_input_code(&mut module.types, vertex_function);

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

            if let Some(idx) = input_emissive_color_field_idx {
                indices.emissive_color = Some(
                    vertex_output_struct_builder.add_field_with_perspective_interpolation(
                        "emissiveColor",
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
    /// The ambient color is computed based on the diffuse and specular color
    /// and the ambient radiance, and is returned from the function in an output
    /// struct. If the prepass involves normal or parallax mapping, the code for
    /// this is generated and the resulting quantities are included in the
    /// output struct. If there is an emissive color, this is also returned
    /// directly in the output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &PrepassVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let (bind_group, texture_coord_expr) = if !self.texture_input.is_empty() {
            let bind_group = *bind_group_idx;
            *bind_group_idx += 1;

            (
                bind_group,
                Some(
                    fragment_input_struct.get_field_expr(
                        mesh_input_field_indices
                            .texture_coords
                            .expect("Missing texture coordinates for shading prepass"),
                    ),
                ),
            )
        } else {
            (*bind_group_idx, None)
        };

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
                bind_group,
                texture_coord_expr,
            );

        let ambient_color_expr = match light_shader_generator {
            None => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "getBaseAmbientColor",
                Vec::new(),
            ),
            Some(LightShaderGenerator::AmbientLight(ambient_light_shader_generator)) => {
                let diffuse_color_expr = self
                    .texture_input
                    .diffuse_texture_and_sampler_bindings
                    .map(|(diffuse_texture_binding, diffuse_sampler_binding)| {
                        let diffuse_color_texture = SampledTexture::declare(
                            &mut module.types,
                            &mut module.global_variables,
                            TextureType::Image2D,
                            "diffuseColor",
                            bind_group,
                            diffuse_texture_binding,
                            Some(diffuse_sampler_binding),
                            None,
                        );

                        diffuse_color_texture.generate_rgb_sampling_expr(
                            fragment_function,
                            texture_coord_expr.unwrap(),
                        )
                    })
                    .or_else(|| {
                        material_input_field_indices
                            .diffuse_color
                            .map(|idx| fragment_input_struct.get_field_expr(idx))
                    });

                let diffuse_ambient_color = diffuse_color_expr.map(|diffuse_color_expr| {
                    source_code_lib.generate_function_call(
                        module,
                        fragment_function,
                        "computeAmbientColorForLambertian",
                        vec![diffuse_color_expr, ambient_light_shader_generator.radiance],
                    )
                });

                let specular_ambient_color = self
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
                                bind_group,
                                specular_reflectance_texture_binding,
                                Some(specular_reflectance_sampler_binding),
                                None,
                            );

                            let (
                                specular_reflectance_lookup_texture_expr,
                                specular_reflectance_lookup_sampler_expr,
                            ) = specular_reflectance_lookup_texture
                                .generate_texture_and_sampler_expressions(fragment_function, false);

                            let specular_color_expr = self
                                .texture_input
                                .specular_texture_and_sampler_bindings
                                .map(|(specular_texture_binding, specular_sampler_binding)| {
                                    let specular_color_texture = SampledTexture::declare(
                                        &mut module.types,
                                        &mut module.global_variables,
                                        TextureType::Image2D,
                                        "specularColor",
                                        bind_group,
                                        specular_texture_binding,
                                        Some(specular_sampler_binding),
                                        None,
                                    );

                                    specular_color_texture.generate_rgb_sampling_expr(
                                        fragment_function,
                                        texture_coord_expr.unwrap(),
                                    )
                                })
                                .or_else(|| {
                                    material_input_field_indices
                                        .specular_color
                                        .map(|idx| fragment_input_struct.get_field_expr(idx))
                                });

                            let fixed_roughness_value_expr = fragment_input_struct.get_field_expr(
                                material_input_field_indices
                                    .roughness
                                    .expect("Missing roughness for computing specular ambient color"),
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
                                            bind_group,
                                            roughness_texture_binding,
                                            Some(roughness_sampler_binding),
                                            None,
                                        );

                                        let roughness_texture_value_expr = roughness_texture
                                            .generate_single_channel_sampling_expr(
                                                fragment_function,
                                                texture_coord_expr.unwrap(),
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
                                    "Missing position for computing specular ambient color",
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
                                "computeAmbientColorForSpecularGGX",
                                vec![
                                    specular_reflectance_lookup_texture_expr,
                                    specular_reflectance_lookup_sampler_expr,
                                    view_dir_expr,
                                    normal_vector_expr.expect("Missing normal vector for computing specular ambient color"),
                                    specular_color_expr.expect("Missing specular color for computing specular ambient color"),
                                    roughness_expr,
                                    ambient_light_shader_generator.radiance,
                                ],
                            )
                        },
                    );

                match (diffuse_ambient_color, specular_ambient_color) {
                    (None, None) => source_code_lib.generate_function_call(
                        module,
                        fragment_function,
                        "getBaseAmbientColor",
                        Vec::new(),
                    ),
                    (Some(diffuse_ambient_color), None) => diffuse_ambient_color,
                    (None, Some(specular_ambient_color)) => specular_ambient_color,
                    (Some(diffuse_ambient_color), Some(specular_ambient_color)) => {
                        emit_in_func(fragment_function, |function| {
                            include_expr_in_func(
                                function,
                                Expression::Binary {
                                    op: BinaryOperator::Add,
                                    left: diffuse_ambient_color,
                                    right: specular_ambient_color,
                                },
                            )
                        })
                    }
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

        // Write emissive color to the surface color attachment.
        if let Some(emissive_color_idx) = material_input_field_indices.emissive_color {
            let emissive_color_expr = fragment_input_struct.get_field_expr(emissive_color_idx);

            let emissive_rgba_color_expr = append_unity_component_to_vec3(
                &mut module.types,
                fragment_function,
                emissive_color_expr,
            );

            output_struct_builder.add_field(
                "emissiveColor",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                emissive_rgba_color_expr,
            );
        // If we do not write an emissive color, we need to write a clear color
        // in case we are obscuring an emissive object, otherwise the emissive
        // object will shine through
        } else {
            let clear_color_expr = source_code_lib.generate_function_call(
                module,
                fragment_function,
                "getBaseAmbientColor",
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

        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::POSITION) {
            let position_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .position
                    .expect("Missing position for writing to render attachment"),
            );

            let padded_position_expr =
                append_unity_component_to_vec3(&mut module.types, fragment_function, position_expr);

            output_struct_builder.add_field(
                "position",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                padded_position_expr,
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

        // Write ambient color to the color render attachment (will be used when
        // applying ambient occlusion).
        if output_render_attachment_quantities.contains(RenderAttachmentQuantitySet::COLOR) {
            let ambient_rgba_color_expr = append_unity_component_to_vec3(
                &mut module.types,
                fragment_function,
                ambient_color_expr,
            );

            output_struct_builder.add_field(
                "ambientColor",
                vec4_type,
                None,
                None,
                VECTOR_4_SIZE,
                ambient_rgba_color_expr,
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
        self.diffuse_texture_and_sampler_bindings.is_none() && self.bump_mapping_input.is_none()
    }
}
