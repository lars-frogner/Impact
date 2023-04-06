//! Generation of shaders for microfacet materials.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func,
    generate_fragment_normal_vector_and_texture_coord_expr, include_expr_in_func, insert_in_arena,
    new_name, push_to_block, InputStruct, InputStructBuilder, LightShaderGenerator,
    LightVertexOutputFieldIndices, MeshVertexOutputFieldIndices,
    OmnidirectionalLightShaderGenerator, OutputStructBuilder, SampledTexture, SourceCode,
    TextureType, UnidirectionalLightShaderGenerator, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE,
    VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Expression, Function, LocalVariable, MathFunction, Module, Statement};

/// Describes the combination of models used for diffuse and specular reflection
/// as part of a microfacet based reflection model.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetShadingModel {
    pub diffuse: DiffuseMicrofacetShadingModel,
    pub specular: SpecularMicrofacetShadingModel,
}

/// Models describing diffuse reflection as part of a microfacet based
/// reflection model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiffuseMicrofacetShadingModel {
    /// No diffuse reflection.
    None,
    /// Diffuse reflection is modelled with a Lambertian BRDF.
    Lambertian,
    /// Diffuse reflection is modelled with a microfacet BRDF using the GGX
    /// distribution of normals.
    GGX,
}

/// Models describing specular reflection as part of a microfacet based
/// reflection model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpecularMicrofacetShadingModel {
    /// No specular reflection.
    None,
    /// Specular reflection is modelled with a microfacet BRDF using the GGX
    /// distribution of normals.
    GGX,
}

/// Input description specifying the vertex attribute locations of microfacet
/// material properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetFeatureShaderInput {
    /// Vertex attribute location for the instance feature representing diffuse
    /// color.
    pub diffuse_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing specular
    /// color.
    pub specular_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing
    /// roughness.
    pub roughness_location: u32,
    /// Vertex attribute location for the instance feature representing the
    /// displacement scale for parallax mapping.
    pub parallax_displacement_scale_location: u32,
}

/// Input description specifying the bindings of textures for microfacet
/// properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and its sampler.
    pub diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular color texture and its sampler.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the roughness texture and its sampler.
    pub roughness_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the normal map texture and its sampler.
    pub normal_map_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the height map texture and its sampler.
    pub height_map_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a microfacet material.
#[derive(Clone, Debug)]
pub struct MicrofacetShaderGenerator<'a> {
    model: &'a MicrofacetShadingModel,
    feature_input: &'a MicrofacetFeatureShaderInput,
    texture_input: &'a MicrofacetTextureShaderInput,
}

/// Indices of the fields holding the various microfacet properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MicrofacetVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    roughness: usize,
    parallax_displacement_scale: usize,
}

impl MicrofacetShadingModel {
    pub const NO_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::None,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const LAMBERTIAN_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::Lambertian,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const GGX_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::GGX,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const GGX_DIFFUSE_NO_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::GGX,
        specular: SpecularMicrofacetShadingModel::None,
    };
}

impl<'a> MicrofacetShaderGenerator<'a> {
    /// Creates a new shader generator using the given input description.
    pub fn new(
        model: &'a MicrofacetShadingModel,
        feature_input: &'a MicrofacetFeatureShaderInput,
        texture_input: &'a MicrofacetTextureShaderInput,
    ) -> Self {
        Self {
            model,
            feature_input,
            texture_input,
        }
    }

    /// Generates the vertex shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The struct of vertex buffered microfacet properties is added as an
    /// input argument to the main vertex shader function and its fields are
    /// assigned to the output struct returned from the function.
    ///
    /// # Returns
    /// The indices of the microfacet property fields in the output struct,
    /// required for accessing the properties in [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> MicrofacetVertexOutputFieldIndices {
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

        let input_roughness_field_idx = input_struct_builder.add_field(
            "roughness",
            float_type,
            self.feature_input.roughness_location,
            F32_WIDTH,
        );

        let input_parallax_displacement_scale_field_idx = input_struct_builder.add_field(
            "parallaxDisplacementScale",
            float_type,
            self.feature_input.parallax_displacement_scale_location,
            F32_WIDTH,
        );

        let input_struct =
            input_struct_builder.generate_input_code(&mut module.types, vertex_function);

        let output_roughness_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "roughness",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_roughness_field_idx),
            );

        let output_parallax_displacement_scale_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "parallaxDisplacementScale",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_parallax_displacement_scale_field_idx),
            );

        let mut indices = MicrofacetVertexOutputFieldIndices {
            diffuse_color: None,
            specular_color: None,
            roughness: output_roughness_field_idx,
            parallax_displacement_scale: output_parallax_displacement_scale_field_idx,
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
    /// with material properties passed from the main vertex shader to compute
    /// the contribution to the fragment color for the active light with the
    /// appropriate microfacet BRDF, and the output color is returned from the
    /// main fragment shader function in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &MicrofacetVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let light_shader_generator =
            light_shader_generator.expect("Missing light for microfacet shading");

        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for microfacet shading"),
        );

        let fixed_roughness_value_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.roughness);

        let view_dir_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeCameraSpaceViewDirection",
            vec![position_expr],
        );

        let (bind_group, texture_coord_expr) = if !self.texture_input.is_empty() {
            let texture_coord_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .texture_coords
                    .expect("No `texture_coords` passed to textured microfacet fragment shader"),
            );

            let bind_group = *bind_group_idx;
            *bind_group_idx += 1;

            (bind_group, Some(texture_coord_expr))
        } else {
            (*bind_group_idx, None)
        };

        let (normal_vector_expr, texture_coord_expr) =
            generate_fragment_normal_vector_and_texture_coord_expr(
                module,
                source_code_lib,
                fragment_function,
                fragment_input_struct,
                mesh_input_field_indices,
                self.texture_input.normal_map_texture_and_sampler_bindings,
                self.texture_input.height_map_texture_and_sampler_bindings,
                material_input_field_indices.parallax_displacement_scale,
                bind_group,
                view_dir_expr,
                texture_coord_expr,
            );

        let diffuse_color_expr = self
            .texture_input
            .diffuse_texture_and_sampler_bindings
            .map(|(diffuse_texture_binding, diffuse_sampler_binding)| {
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
                    TextureType::Image,
                    "specularColor",
                    bind_group,
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

        let roughness_expr = self
            .texture_input
            .roughness_texture_and_sampler_bindings
            .map_or(
                fixed_roughness_value_expr,
                |(roughness_texture_binding, roughness_sampler_binding)| {
                    let roughness_texture = SampledTexture::declare(
                        &mut module.types,
                        &mut module.global_variables,
                        TextureType::Image,
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
                        vec![roughness_texture_value_expr, fixed_roughness_value_expr],
                    )
                },
            );

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

        let (light_dir_expr, light_radiance_expr) = match (
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
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                omnidirectional_light_shader_generator.generate_fragment_shading_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    camera_clip_position_expr,
                    position_expr,
                    normal_vector_expr,
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
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                unidirectional_light_shader_generator.generate_fragment_shading_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    fragment_input_struct,
                    unidirectional_light_input_field_indices,
                    camera_clip_position_expr,
                    normal_vector_expr,
                )
            }
            _ => {
                panic!("Invalid variant of light shader generator and/or light vertex output field indices for microfacet shading");
            }
        };

        let light_color_expr = match (self.model, diffuse_color_expr, specular_color_expr) {
            (&MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR, Some(diffuse_color_expr), None) => {
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "computeGGXDiffuseNoSpecularColor",
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        diffuse_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (&MicrofacetShadingModel::NO_DIFFUSE_GGX_SPECULAR, None, Some(specular_color_expr)) => {
                source_code_lib.generate_function_call(
                    module,
                    fragment_function,
                    "computeNoDiffuseGGXSpecularColor",
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        specular_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (
                &MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                Some(diffuse_color_expr),
                Some(specular_color_expr),
            ) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeLambertianDiffuseGGXSpecularColor",
                vec![
                    view_dir_expr,
                    normal_vector_expr,
                    diffuse_color_expr,
                    specular_color_expr,
                    roughness_expr,
                    light_dir_expr,
                    light_radiance_expr,
                ],
            ),
            (
                &MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                Some(diffuse_color_expr),
                Some(specular_color_expr),
            ) => source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeGGXDiffuseGGXSpecularColor",
                vec![
                    view_dir_expr,
                    normal_vector_expr,
                    diffuse_color_expr,
                    specular_color_expr,
                    roughness_expr,
                    light_dir_expr,
                    light_radiance_expr,
                ],
            ),
            (_, None, None) => panic!("No diffuse or specular color for microfacet shader"),
            _ => panic!("Invalid combinations of microfacet shading models"),
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

impl MicrofacetTextureShaderInput {
    fn is_empty(&self) -> bool {
        self.diffuse_texture_and_sampler_bindings.is_none()
            && self.specular_texture_and_sampler_bindings.is_none()
            && self.roughness_texture_and_sampler_bindings.is_none()
            && self.normal_map_texture_and_sampler_bindings.is_none()
            && self.height_map_texture_and_sampler_bindings.is_none()
    }
}
