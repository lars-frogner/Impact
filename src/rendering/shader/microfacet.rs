//! Generation of shaders for microfacet materials.

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
}

/// Shader generator for a microfacet material.
#[derive(Clone, Debug)]
pub struct MicrofacetShaderGenerator<'a> {
    model: &'a MicrofacetShadingModel,
    feature_input: &'a LightMaterialFeatureShaderInput,
    texture_input: &'a MicrofacetTextureShaderInput,
}

/// Indices of the fields holding the various microfacet properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MicrofacetVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    roughness: usize,
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
        feature_input: &'a LightMaterialFeatureShaderInput,
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

        let input_struct =
            input_struct_builder.generate_input_code(&mut module.types, vertex_function);

        let output_roughness_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "roughness",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_roughness_field_idx),
            );

        let mut indices = MicrofacetVertexOutputFieldIndices {
            diffuse_color: None,
            specular_color: None,
            roughness: output_roughness_field_idx,
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
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
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

        let screen_space_texture_coord_expr = if input_render_attachment_quantities.is_empty() {
            None
        } else {
            Some(source_code_lib.generate_function_call(
                module,
                fragment_function,
                "convertFramebufferPositionToScreenTextureCoords",
                vec![push_constant_fragment_expressions.inverse_window_dimensions,
                fragment_input_struct
                        .get_field_expr(mesh_input_field_indices.framebuffer_position)],
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
                        .expect("Missing position for microfacet shading"),
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
                        .expect("Missing normal vector for microfacet shading"),
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
                        .expect("Missing texture coordinates for microfacet shading"),
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

        let fixed_roughness_value_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.roughness);

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
                let framebuffer_position_expr = fragment_input_struct
                    .get_field_expr(mesh_input_field_indices.framebuffer_position);

                omnidirectional_light_shader_generator.generate_fragment_shading_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    framebuffer_position_expr,
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
                )
            }
            _ => {
                panic!("Invalid variant of light shader generator and/or light vertex output field indices for microfacet shading");
            }
        };

        let view_dir_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeCameraSpaceViewDirection",
            vec![position_expr],
        );

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
    }
}
