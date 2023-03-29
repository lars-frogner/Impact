//! Generation of shaders for Blinn-Phong materials.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
    insert_in_arena, new_name, push_to_block, InputStruct, InputStructBuilder,
    LightShaderGenerator, LightVertexOutputFieldIndices, MeshVertexOutputFieldIndices,
    OmnidirectionalLightShaderGenerator, OutputStructBuilder, SampledTexture, SourceCode,
    TextureType, UnidirectionalLightShaderGenerator, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE,
    VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Expression, Function, Handle, LocalVariable, MathFunction, Module, Statement};

/// Input description specifying the vertex attribute locations
/// of Blinn-Phong material properties, reqired for generating a
/// shader for a [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing diffuse color. If [`None`], diffuse
    /// color is obtained from a texture instead.
    pub diffuse_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing specular color. If [`None`], specular
    /// color is obtained from a texture instead.
    pub specular_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing shininess.
    pub shininess_location: u32,
    /// Vertex attribute location for the instance feature representing the
    /// height scale for parallax mapping, or [`None`] parallax mapping is not
    /// used.
    pub parallax_height_scale_location: u32,
}

/// Input description specifying the bindings of textures
/// for Blinn-Phong properties, required for generating a
/// shader for a
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and
    /// its sampler.
    pub diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular color texture and
    /// its sampler. If [`None`], specular color is an instance
    /// feature instead.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub normal_map_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub height_map_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a
/// [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug)]
pub struct BlinnPhongShaderGenerator<'a> {
    feature_input: &'a BlinnPhongFeatureShaderInput,
    texture_input: &'a BlinnPhongTextureShaderInput,
}

/// Indices of the fields holding the various Blinn-Phong
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct BlinnPhongVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    shininess: usize,
    parallax_height_scale: usize,
}

impl<'a> BlinnPhongShaderGenerator<'a> {
    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(
        feature_input: &'a BlinnPhongFeatureShaderInput,
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
            self.feature_input.shininess_location,
            F32_WIDTH,
        );

        let input_parallax_height_scale_field_idx = input_struct_builder.add_field(
            "parallaxHeightScale",
            float_type,
            self.feature_input.parallax_height_scale_location,
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

        let output_parallax_height_scale_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "parallaxHeightScale",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_parallax_height_scale_field_idx),
            );

        let mut indices = BlinnPhongVertexOutputFieldIndices {
            diffuse_color: None,
            specular_color: None,
            shininess: output_shininess_field_idx,
            parallax_height_scale: output_parallax_height_scale_field_idx,
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
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &BlinnPhongVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn computeViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
                return normalize(-vertexPosition);
            }

            fn computeDiffuseBlinnPhongColor(
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let diffuseFactor = max(0.0, dot(lightDirection, normalVector));
                return lightRadiance * diffuseFactor * diffuseColor;
            }

            fn computeSpecularBlinnPhongColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                specularColor: vec3<f32>,
                shininess: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));
                let specularFactor = pow(max(0.0, dot(halfVector, normalVector)), shininess);
                return lightRadiance * specularFactor * specularColor;
            }

            fn computeBlinnPhongColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                shininess: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));
                let diffuseFactor = max(0.0, dot(lightDirection, normalVector));
                let specularFactor = pow(max(0.0, dot(halfVector, normalVector)), shininess);
                return lightRadiance * (diffuseFactor * diffuseColor + specularFactor * specularColor);
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let light_shader_generator =
            light_shader_generator.expect("Missing light for Blinn-Phong shading");

        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for Blinn-Phong shading"),
        );

        let normal_vector_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .normal_vector
                .expect("Missing normal vector for Blinn-Phong shading"),
        );

        let shininess_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.shininess);

        let (diffuse_texture_color_expr, specular_texture_color_expr) =
            Self::generate_texture_fragment_code(
                self.texture_input,
                module,
                fragment_function,
                bind_group_idx,
                fragment_input_struct,
                mesh_input_field_indices,
            );

        let diffuse_color_expr = diffuse_texture_color_expr.or_else(|| {
            material_input_field_indices
                .diffuse_color
                .map(|idx| fragment_input_struct.get_field_expr(idx))
        });

        let specular_color_expr = specular_texture_color_expr.or_else(|| {
            material_input_field_indices
                .specular_color
                .map(|idx| fragment_input_struct.get_field_expr(idx))
        });

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

        let compute_color = |fragment_function, light_dir_expr, light_radiance_expr| match (
            diffuse_color_expr,
            specular_color_expr,
        ) {
            (Some(diffuse_color_expr), None) => SourceCode::generate_call_named(
                fragment_function,
                "lightColor",
                source_code.functions["computeDiffuseBlinnPhongColor"],
                vec![
                    normal_vector_expr,
                    diffuse_color_expr,
                    light_dir_expr,
                    light_radiance_expr,
                ],
            ),
            (None, Some(specular_color_expr)) => {
                let view_dir_expr = SourceCode::generate_call_named(
                    fragment_function,
                    "viewDirection",
                    source_code.functions["computeViewDirection"],
                    vec![position_expr],
                );
                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    source_code.functions["computeSpecularBlinnPhongColor"],
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        specular_color_expr,
                        shininess_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (Some(diffuse_color_expr), Some(specular_color_expr)) => {
                let view_dir_expr = SourceCode::generate_call_named(
                    fragment_function,
                    "viewDirection",
                    source_code.functions["computeViewDirection"],
                    vec![position_expr],
                );
                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    source_code.functions["computeBlinnPhongColor"],
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        diffuse_color_expr,
                        specular_color_expr,
                        shininess_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (None, None) => panic!("No diffuse or specular color for Blinn-Phong shader"),
        };

        let light_color_expr = match (light_shader_generator, light_input_field_indices) {
            (
                LightShaderGenerator::OmnidirectionalLight(
                    OmnidirectionalLightShaderGenerator::ForShading(
                        omnidirectional_light_shader_generator,
                    ),
                ),
                None,
            ) => {
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                let (light_dir_expr, light_radiance_expr) = omnidirectional_light_shader_generator
                    .generate_fragment_shading_code(
                        module,
                        fragment_function,
                        camera_clip_position_expr,
                        position_expr,
                        normal_vector_expr,
                    );

                compute_color(fragment_function, light_dir_expr, light_radiance_expr)
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
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                let light_space_position_expr = fragment_input_struct
                    .get_field_expr(unidirectional_light_input_field_indices.light_space_position);

                let light_space_normal_vector_expr = fragment_input_struct.get_field_expr(
                    unidirectional_light_input_field_indices.light_space_normal_vector,
                );

                let (light_dir_expr, light_radiance_expr) = unidirectional_light_shader_generator
                    .generate_fragment_shading_code(
                        module,
                        fragment_function,
                        camera_clip_position_expr,
                        light_space_position_expr,
                        light_space_normal_vector_expr,
                    );

                compute_color(fragment_function, light_dir_expr, light_radiance_expr)
            }
            _ => {
                panic!("Invalid variant of light shader generator and/or light vertex output field indices for Blinn-Phong shading");
            }
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

    fn generate_texture_fragment_code(
        texture_input: &BlinnPhongTextureShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) -> (Option<Handle<Expression>>, Option<Handle<Expression>>) {
        if texture_input.diffuse_texture_and_sampler_bindings.is_none()
            && texture_input
                .specular_texture_and_sampler_bindings
                .is_none()
            && texture_input
                .normal_map_texture_and_sampler_bindings
                .is_none()
            && texture_input
                .height_map_texture_and_sampler_bindings
                .is_none()
        {
            return (None, None);
        }

        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let texture_coord_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .texture_coords
                .expect("No `texture_coords` passed to textured Blinn-Phong fragment shader"),
        );

        let diffuse_color_sampling_expr = texture_input.diffuse_texture_and_sampler_bindings.map(
            |(diffuse_texture_binding, diffuse_sampler_binding)| {
                let diffuse_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image,
                    "diffuseColor",
                    bind_group,
                    diffuse_texture_binding,
                    Some(diffuse_sampler_binding),
                    None,
                );

                diffuse_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr)
            },
        );

        let specular_color_sampling_expr = texture_input.specular_texture_and_sampler_bindings.map(
            |(specular_texture_binding, specular_sampler_binding)| {
                let specular_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image,
                    "specularColor",
                    bind_group,
                    specular_texture_binding,
                    Some(specular_sampler_binding),
                    None,
                );

                specular_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr)
            },
        );

        let tangent_space_normal_sampling_expr = texture_input
            .normal_map_texture_and_sampler_bindings
            .map(|(normal_map_texture_binding, normal_map_sampler_binding)| {
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

                normal_map_texture.generate_rgb_sampling_expr(fragment_function, texture_coord_expr)
            });

        (diffuse_color_sampling_expr, specular_color_sampling_expr)
    }
}
