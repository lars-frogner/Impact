//! Generation of shaders for Blinn-Phong materials.

use super::{
    append_to_arena, emit, emit_in_func, include_expr_in_func, insert_in_arena, new_name,
    push_to_block, ForLoop, InputStruct, InputStructBuilder, LightExpressions,
    MeshVertexOutputFieldIndices, OutputStructBuilder, SampledTexture, SourceCodeFunctions,
    F32_TYPE, F32_WIDTH, VECTOR_3_SIZE, VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{
    BinaryOperator, Expression, Function, Handle, LocalVariable, MathFunction, Module, Statement,
};

/// Input description specifying the vertex attribute locations
/// of Blinn-Phong material properties, reqired for generating a
/// shader for a [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing ambient color.
    pub ambient_color_location: u32,
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
    /// Vertex attribute location for the instance feature
    /// representing alpha.
    pub alpha_location: u32,
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
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    /// Bind group bindings of the specular color texture and
    /// its sampler. If [`None`], specular color is an instance
    /// feature instead.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a
/// [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug)]
pub struct BlinnPhongShaderGenerator<'a> {
    feature_input: &'a BlinnPhongFeatureShaderInput,
    texture_input: Option<&'a BlinnPhongTextureShaderInput>,
}

/// Indices of the fields holding the various Blinn-Phong
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct BlinnPhongVertexOutputFieldIndices {
    ambient_color: usize,
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    shininess: usize,
    alpha: usize,
}

impl<'a> BlinnPhongShaderGenerator<'a> {
    /// Whether the material requires light sources.
    pub const fn requires_lights() -> bool {
        true
    }

    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(
        feature_input: &'a BlinnPhongFeatureShaderInput,
        texture_input: Option<&'a BlinnPhongTextureShaderInput>,
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
        let float_type_handle = insert_in_arena(&mut module.types, F32_TYPE);
        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("MaterialProperties", "material");

        let input_ambient_color_field_idx = input_struct_builder.add_field(
            "ambientColor",
            vec3_type_handle,
            self.feature_input.ambient_color_location,
            VECTOR_3_SIZE,
        );

        let input_diffuse_color_field_idx =
            self.feature_input.diffuse_color_location.map(|location| {
                input_struct_builder.add_field(
                    "diffuseColor",
                    vec3_type_handle,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_specular_color_field_idx =
            self.feature_input.specular_color_location.map(|location| {
                input_struct_builder.add_field(
                    "specularColor",
                    vec3_type_handle,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_shininess_field_idx = input_struct_builder.add_field(
            "shininess",
            float_type_handle,
            self.feature_input.shininess_location,
            F32_WIDTH,
        );

        let input_alpha_field_idx = input_struct_builder.add_field(
            "alpha",
            float_type_handle,
            self.feature_input.alpha_location,
            F32_WIDTH,
        );

        let input_struct =
            input_struct_builder.generate_input_code(&mut module.types, vertex_function);

        let output_ambient_color_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "ambientColor",
                vec3_type_handle,
                VECTOR_3_SIZE,
                input_struct.get_field_expr_handle(input_ambient_color_field_idx),
            );

        let output_shininess_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "shininess",
                float_type_handle,
                F32_WIDTH,
                input_struct.get_field_expr_handle(input_shininess_field_idx),
            );

        let output_alpha_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "alpha",
                float_type_handle,
                F32_WIDTH,
                input_struct.get_field_expr_handle(input_alpha_field_idx),
            );

        let mut indices = BlinnPhongVertexOutputFieldIndices {
            ambient_color: output_ambient_color_field_idx,
            diffuse_color: None,
            specular_color: None,
            shininess: output_shininess_field_idx,
            alpha: output_alpha_field_idx,
        };

        if let Some(idx) = input_diffuse_color_field_idx {
            indices.diffuse_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "diffuseColor",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        if let Some(idx) = input_specular_color_field_idx {
            indices.specular_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "specularColor",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        indices
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The texture and sampler for any material properties sampled
    /// from textured are declared as global variables, and sampling
    /// expressions are generated in the main fragment shader function.
    /// These are used together with material properties passed from the
    /// main vertex shader to generate the Blinn-Phong shading equation,
    /// whose output color is returned from the main fragment shader
    /// function in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &BlinnPhongVertexOutputFieldIndices,
        light_expressions: Option<&LightExpressions>,
    ) {
        let function_handles = SourceCodeFunctions::from_wgsl_source(
            "\
            fn computeViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
                return normalize(-vertexPosition);
            }

            fn computeBlinnPhongColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                shininess: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));
                let diffuseFactor = max(0.0, dot(lightDirection, normalVector));
                let specularFactor = pow(max(0.0, dot(halfVector, normalVector)), shininess);
                return lightRadiance * (diffuseFactor * diffuseColor + specularFactor * specularColor);
            }
            
            fn computeBlinnPhongColorForPointLight(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                vertexPosition: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                shininess: f32,
                lightPosition: vec3<f32>,
                lightRadiance: vec3<f32>
            ) -> vec3<f32> {
                 let lightDisplacement = lightPosition - vertexPosition;
                 let inverseSquaredLightDistance = 1.0 / dot(lightDisplacement, lightDisplacement);
                 let lightDirection = lightDisplacement * sqrt(inverseSquaredLightDistance);
                 let attenuatedLightRadiance = lightRadiance * inverseSquaredLightDistance;
                 return computeBlinnPhongColor(
                    viewDirection,
                    normalVector,
                    diffuseColor,
                    specularColor,
                    shininess,
                    lightDirection,
                    attenuatedLightRadiance
                );
            }
            
            fn computeBlinnPhongColorForDirectionalLight(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                shininess: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>
            ) -> vec3<f32> {
                 return computeBlinnPhongColor(
                    viewDirection,
                    normalVector,
                    diffuseColor,
                    specularColor,
                    shininess,
                    -lightDirection,
                    lightRadiance
                );
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let view_direction_function_handle = function_handles[0];
        let point_light_color_function_handle = function_handles[2];
        let directional_light_color_function_handle = function_handles[3];

        let light_expressions = light_expressions.expect("Missing lights for Blinn-Phong shading");

        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let position_expr_handle = fragment_input_struct.get_field_expr_handle(
            mesh_input_field_indices
                .position
                .expect("Missing positions for Blinn-Phong shading"),
        );

        let normal_vector_expr_handle = fragment_input_struct.get_field_expr_handle(
            mesh_input_field_indices
                .normal_vector
                .expect("Missing normal vectors for Blinn-Phong shading"),
        );

        let ambient_color_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.ambient_color);

        let shininess_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.shininess);

        let alpha_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.alpha);

        let (diffuse_color_expr_handle, specular_color_expr_handle) =
            if let Some(texture_input) = self.texture_input {
                let (diffuse_color_expr_handle, specular_color_expr_handle) =
                    Self::generate_texture_fragment_code(
                        texture_input,
                        module,
                        fragment_function,
                        bind_group_idx,
                        fragment_input_struct,
                        mesh_input_field_indices,
                    );
                (
                    diffuse_color_expr_handle,
                    specular_color_expr_handle.unwrap_or_else(|| {
                        fragment_input_struct.get_field_expr_handle(
                            material_input_field_indices.specular_color.expect(
                                "Missing `specular_color` feature for Blinn-Phong material",
                            ),
                        )
                    }),
                )
            } else {
                (
                    fragment_input_struct.get_field_expr_handle(
                        material_input_field_indices
                            .diffuse_color
                            .expect("Missing `diffuse_color` feature for Blinn-Phong material"),
                    ),
                    fragment_input_struct.get_field_expr_handle(
                        material_input_field_indices
                            .specular_color
                            .expect("Missing `specular_color` feature for Blinn-Phong material"),
                    ),
                )
            };

        let color_ptr_expr_handle = append_to_arena(
            &mut fragment_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut fragment_function.local_variables,
                LocalVariable {
                    name: new_name("color"),
                    ty: vec3_type_handle,
                    init: None,
                },
            )),
        );

        push_to_block(
            &mut fragment_function.body,
            Statement::Store {
                pointer: color_ptr_expr_handle,
                value: ambient_color_expr_handle,
            },
        );

        let view_dir_expr_handle = SourceCodeFunctions::generate_call(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
            view_direction_function_handle,
            vec![position_expr_handle],
        );

        let point_light_count_expr_handle =
            light_expressions.generate_point_light_count_expr(fragment_function);

        let mut point_light_loop = ForLoop::new(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            "point_light",
            point_light_count_expr_handle,
        );

        let (light_position_expr_handle, light_radiance_expr_handle) = light_expressions
            .generate_point_light_field_expressions(
                &mut point_light_loop.body,
                &mut fragment_function.expressions,
                point_light_loop.idx_expr_handle,
            );

        let returned_point_light_color_expr_handle = SourceCodeFunctions::generate_call(
            &mut point_light_loop.body,
            &mut fragment_function.expressions,
            point_light_color_function_handle,
            vec![
                view_dir_expr_handle,
                normal_vector_expr_handle,
                position_expr_handle,
                diffuse_color_expr_handle,
                specular_color_expr_handle,
                shininess_expr_handle,
                light_position_expr_handle,
                light_radiance_expr_handle,
            ],
        );

        let accumulated_color_expr_handle = emit(
            &mut point_light_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let color_expr_handle = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: color_ptr_expr_handle,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: color_expr_handle,
                        right: returned_point_light_color_expr_handle,
                    },
                )
            },
        );

        push_to_block(
            &mut point_light_loop.body,
            Statement::Store {
                pointer: color_ptr_expr_handle,
                value: accumulated_color_expr_handle,
            },
        );

        point_light_loop.generate_code(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
        );

        let directional_light_count_expr_handle =
            light_expressions.generate_directional_light_count_expr(fragment_function);

        let mut directional_light_loop = ForLoop::new(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            "directional_light",
            directional_light_count_expr_handle,
        );

        let (light_direction_expr_handle, light_radiance_expr_handle) = light_expressions
            .generate_directional_light_field_expressions(
                &mut directional_light_loop.body,
                &mut fragment_function.expressions,
                directional_light_loop.idx_expr_handle,
            );

        let returned_directional_light_color_expr_handle = SourceCodeFunctions::generate_call(
            &mut directional_light_loop.body,
            &mut fragment_function.expressions,
            directional_light_color_function_handle,
            vec![
                view_dir_expr_handle,
                normal_vector_expr_handle,
                diffuse_color_expr_handle,
                specular_color_expr_handle,
                shininess_expr_handle,
                light_direction_expr_handle,
                light_radiance_expr_handle,
            ],
        );

        let accumulated_color_expr_handle = emit(
            &mut directional_light_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let color_expr_handle = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: color_ptr_expr_handle,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: color_expr_handle,
                        right: returned_directional_light_color_expr_handle,
                    },
                )
            },
        );

        push_to_block(
            &mut directional_light_loop.body,
            Statement::Store {
                pointer: color_ptr_expr_handle,
                value: accumulated_color_expr_handle,
            },
        );

        directional_light_loop.generate_code(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
        );

        let output_color_expr_handle = emit_in_func(fragment_function, |function| {
            let color_expr_handle = include_expr_in_func(
                function,
                Expression::Load {
                    pointer: color_ptr_expr_handle,
                },
            );

            let saturated_color_expr_handle = include_expr_in_func(
                function,
                Expression::Math {
                    fun: MathFunction::Saturate,
                    arg: color_expr_handle,
                    arg1: None,
                    arg2: None,
                    arg3: None,
                },
            );

            include_expr_in_func(
                function,
                Expression::Compose {
                    ty: vec4_type_handle,
                    components: vec![saturated_color_expr_handle, alpha_expr_handle],
                },
            )
        });

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            output_color_expr_handle,
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
    ) -> (Handle<Expression>, Option<Handle<Expression>>) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let (diffuse_texture_binding, diffuse_sampler_binding) =
            texture_input.diffuse_texture_and_sampler_bindings;

        let diffuse_color_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            "diffuseColor",
            bind_group,
            diffuse_texture_binding,
            diffuse_sampler_binding,
        );

        let texture_coord_expr_handle = fragment_input_struct.get_field_expr_handle(
            mesh_input_field_indices
                .texture_coords
                .expect("No `texture_coords` passed to fixed texture fragment shader"),
        );

        let diffuse_color_sampling_expr_handle = diffuse_color_texture
            .generate_rgb_sampling_expr(fragment_function, texture_coord_expr_handle);

        let specular_color_sampling_expr_handle = texture_input
            .specular_texture_and_sampler_bindings
            .map(|(specular_texture_binding, specular_sampler_binding)| {
                let specular_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    "specularColor",
                    bind_group,
                    specular_texture_binding,
                    specular_sampler_binding,
                );

                specular_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr_handle)
            });

        (
            diffuse_color_sampling_expr_handle,
            specular_color_sampling_expr_handle,
        )
    }
}
