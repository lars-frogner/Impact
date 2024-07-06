//! Generation of shaders for ambient occlusion.

use super::{
    super::{
        append_to_arena, append_unity_component_to_vec3, emit, emit_in_func, include_expr_in_func,
        include_named_expr_in_func, insert_in_arena, new_name, push_to_block, ForLoop, InputStruct,
        OutputStructBuilder, SampledTexture, SourceCode, TextureType, F32_TYPE, F32_WIDTH,
        U32_TYPE, U32_WIDTH, VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    CameraProjectionVariable, MeshVertexOutputFieldIndices, PushConstantFieldExpressions,
    RenderShaderTricks,
};
use crate::{
    gpu::rendering::RenderAttachmentQuantity, material::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
};
use naga::{
    AddressSpace, ArraySize, BinaryOperator, Expression, Function, GlobalVariable, Handle, Literal,
    LocalVariable, Module, ResourceBinding, Statement, StructMember, Type, TypeInner,
};
use std::num::NonZeroU32;

/// Input description specifying the stage and uniform bindings for ambient
/// occlusion.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AmbientOcclusionShaderInput {
    Calculation(AmbientOcclusionCalculationShaderInput),
    Application,
}

/// Input description specifying uniform bindings needed for calculating ambient
/// occlusion.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AmbientOcclusionCalculationShaderInput {
    /// Bind group binding of the uniform containing sample offsets.
    pub sample_uniform_binding: u32,
}

/// Generator for an ambient occlusion shader.
#[derive(Clone, Debug)]
pub(super) struct AmbientOcclusionShaderGenerator<'a> {
    input: &'a AmbientOcclusionShaderInput,
}

impl<'a> AmbientOcclusionShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: RenderShaderTricks = RenderShaderTricks::NO_VERTEX_PROJECTION;

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a AmbientOcclusionShaderInput) -> Self {
        Self { input }
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The stage is determined from the shader input. In the calculation stage,
    /// position and normal vector are sampled from their respective render
    /// attachment textures. This is used together with sample offsets from a
    /// uniform, a noise factor and the depth attachment to calculate the
    /// ambient occlusion factor, which is written to a dedicated render
    /// attachment. In the application stage, the render attachment textures for
    /// ambient reflected luminance and occlusion factor are taken as input. The
    /// ambient reflected luminance is weighted with an averaged occlusion
    /// factor and written to the ambient reflected luminance attachment.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        camera_projection: Option<&CameraProjectionVariable>,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let inverse_window_dimensions_expr =
            push_constant_fragment_expressions.inverse_window_dimensions;

        let framebuffer_position_expr =
            fragment_input_struct.get_field_expr(mesh_input_field_indices.framebuffer_position);

        let screen_space_texture_coord_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "convertFramebufferPositionToScreenTextureCoords",
            vec![inverse_window_dimensions_expr, framebuffer_position_expr],
        );

        match self.input {
            AmbientOcclusionShaderInput::Calculation(input) => {
                Self::generate_fragment_code_for_computing_ambient_occlusion(
                    input,
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    camera_projection,
                    framebuffer_position_expr,
                    screen_space_texture_coord_expr,
                );
            }
            AmbientOcclusionShaderInput::Application => {
                Self::generate_fragment_code_for_applying_ambient_occlusion(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    inverse_window_dimensions_expr,
                    screen_space_texture_coord_expr,
                );
            }
        }
    }

    fn generate_fragment_code_for_computing_ambient_occlusion(
        input: &AmbientOcclusionCalculationShaderInput,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        camera_projection: Option<&CameraProjectionVariable>,
        framebuffer_position_expr: Handle<Expression>,
        screen_space_texture_coord_expr: Handle<Expression>,
    ) {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let sample_offset_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: vec4_type,
                    size: ArraySize::Constant(
                        NonZeroU32::new(MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32).unwrap(),
                    ),
                    stride: VECTOR_4_SIZE,
                },
            },
        );

        let sample_offset_array_size =
            u32::try_from(MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT).unwrap() * VECTOR_4_SIZE;

        let sample_struct_size = sample_offset_array_size + U32_WIDTH + 3 * F32_WIDTH;

        let sample_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("AmbientOcclusionSamples"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("sampleOffsets"),
                            ty: sample_offset_array_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("sampleCount"),
                            ty: u32_type,
                            binding: None,
                            offset: sample_offset_array_size,
                        },
                        StructMember {
                            name: new_name("sampleRadius"),
                            ty: f32_type,
                            binding: None,
                            offset: sample_offset_array_size + U32_WIDTH,
                        },
                        StructMember {
                            name: new_name("sampleNormalization"),
                            ty: f32_type,
                            binding: None,
                            offset: sample_offset_array_size + U32_WIDTH + F32_WIDTH,
                        },
                        StructMember {
                            name: new_name("contrast"),
                            ty: f32_type,
                            binding: None,
                            offset: sample_offset_array_size + U32_WIDTH + 2 * F32_WIDTH,
                        },
                    ],
                    span: sample_struct_size,
                },
            },
        );

        let sample_struct_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("ambientOcclusionSamples"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: input.sample_uniform_binding,
                }),
                ty: sample_struct_type,
                init: None,
            },
        );

        *bind_group_idx += 1;

        let sample_struct_ptr_expr = include_expr_in_func(
            fragment_function,
            Expression::GlobalVariable(sample_struct_var),
        );

        let (
            sample_offset_array_ptr_expr,
            sample_count_expr,
            sample_radius_expr,
            sample_normalization_expr,
            contrast_expr,
        ) = emit_in_func(fragment_function, |function| {
            let sample_offset_array_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sample_struct_ptr_expr,
                    index: 0,
                },
            );

            let sample_count_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sample_struct_ptr_expr,
                    index: 1,
                },
            );

            let sample_count_expr = include_named_expr_in_func(
                function,
                "sampleCount",
                Expression::Load {
                    pointer: sample_count_ptr_expr,
                },
            );

            let sample_radius_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sample_struct_ptr_expr,
                    index: 2,
                },
            );

            let sample_radius_expr = include_named_expr_in_func(
                function,
                "sampleRadius",
                Expression::Load {
                    pointer: sample_radius_ptr_expr,
                },
            );

            let sample_normalization_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sample_struct_ptr_expr,
                    index: 3,
                },
            );

            let sample_normalization_expr = include_named_expr_in_func(
                function,
                "sampleNormalization",
                Expression::Load {
                    pointer: sample_normalization_ptr_expr,
                },
            );

            let contrast_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sample_struct_ptr_expr,
                    index: 4,
                },
            );

            let contrast_expr = include_named_expr_in_func(
                function,
                "sampleNormalization",
                Expression::Load {
                    pointer: contrast_ptr_expr,
                },
            );

            (
                sample_offset_array_ptr_expr,
                sample_count_expr,
                sample_radius_expr,
                sample_normalization_expr,
                contrast_expr,
            )
        });

        let (position_texture_binding, position_sampler_binding) =
            RenderAttachmentQuantity::Position.bindings();

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

        let (position_texture_expr, position_sampler_expr) =
            position_texture.generate_texture_and_sampler_expressions(fragment_function, false);

        let position_expr = position_texture
            .generate_rgb_sampling_expr(fragment_function, screen_space_texture_coord_expr);

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

        let normal_color_expr = normal_vector_texture
            .generate_rgb_sampling_expr(fragment_function, screen_space_texture_coord_expr);

        let normal_vector_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "convertNormalColorToNormalizedNormalVector",
            vec![normal_color_expr],
        );

        let projection_matrix_expr = camera_projection
            .expect("Missing camera projection matrix for computing ambient occlusion")
            .generate_projection_matrix_expr(fragment_function);

        let random_angle_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "generateRandomAngle",
            vec![framebuffer_position_expr],
        );

        let rotation_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeAmbientOcclusionSampleRotation",
            vec![random_angle_expr],
        );

        let squared_sample_radius_expr = emit_in_func(fragment_function, |function| {
            include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: sample_radius_expr,
                    right: sample_radius_expr,
                },
            )
        });

        let zero_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::Literal(Literal::F32(0.0)),
        );

        let summed_occlusion_sample_values_ptr_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut fragment_function.local_variables,
                LocalVariable {
                    name: new_name("summedOcclusionSampleValues"),
                    ty: f32_type,
                    init: Some(zero_expr),
                },
            )),
        );

        let mut sampling_loop = ForLoop::new(
            &mut module.types,
            fragment_function,
            "sample",
            None,
            sample_count_expr,
        );

        let sample_offset_expr = emit(
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let sample_offset_ptr_expr = append_to_arena(
                    expressions,
                    Expression::Access {
                        base: sample_offset_array_ptr_expr,
                        index: sampling_loop.idx_expr,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: sample_offset_ptr_expr,
                    },
                )
            },
        );

        let occlusion_sample_value_expr = source_code_lib.generate_function_call_in_block(
            module,
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            "computeAmbientOcclusionSampleValue",
            vec![
                position_texture_expr,
                position_sampler_expr,
                projection_matrix_expr,
                squared_sample_radius_expr,
                position_expr,
                normal_vector_expr,
                rotation_expr,
                sample_offset_expr,
            ],
        );

        let summed_occlusion_sample_values_expr = emit(
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let prev_summed_occlusion_sample_value_expr = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: summed_occlusion_sample_values_ptr_expr,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: prev_summed_occlusion_sample_value_expr,
                        right: occlusion_sample_value_expr,
                    },
                )
            },
        );

        push_to_block(
            &mut sampling_loop.body,
            Statement::Store {
                pointer: summed_occlusion_sample_values_ptr_expr,
                value: summed_occlusion_sample_values_expr,
            },
        );

        sampling_loop.generate_code(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
        );

        let summed_occlusion_sample_values_expr = emit_in_func(fragment_function, |function| {
            include_expr_in_func(
                function,
                Expression::Load {
                    pointer: summed_occlusion_sample_values_ptr_expr,
                },
            )
        });

        let ambient_visibility_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeAmbientVisibility",
            vec![
                sample_normalization_expr,
                contrast_expr,
                summed_occlusion_sample_values_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "ambientVisibility",
            f32_type,
            None,
            None,
            F32_WIDTH,
            ambient_visibility_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }

    fn generate_fragment_code_for_applying_ambient_occlusion(
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        texel_dimensions_expr: Handle<Expression>,
        screen_space_texture_coord_expr: Handle<Expression>,
    ) {
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let (position_texture_binding, position_sampler_binding) =
            RenderAttachmentQuantity::Position.bindings();

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

        let (position_texture_expr, position_sampler_expr) =
            position_texture.generate_texture_and_sampler_expressions(fragment_function, false);

        let (
            ambient_reflected_luminance_texture_binding,
            ambient_reflected_luminance_sampler_binding,
        ) = RenderAttachmentQuantity::AmbientReflectedLuminance.bindings();

        let ambient_reflected_luminance_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "ambientReflectedLuminance",
            *bind_group_idx,
            ambient_reflected_luminance_texture_binding,
            Some(ambient_reflected_luminance_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let ambient_reflected_luminance_expr = ambient_reflected_luminance_texture
            .generate_rgb_sampling_expr(fragment_function, screen_space_texture_coord_expr);

        let (ambient_visibility_texture_binding, ambient_visibility_sampler_binding) =
            RenderAttachmentQuantity::Occlusion.bindings();

        let ambient_visibility_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "ambientVisibility",
            *bind_group_idx,
            ambient_visibility_texture_binding,
            Some(ambient_visibility_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let (ambient_visibility_texture_expr, ambient_visibility_sampler_expr) =
            ambient_visibility_texture
                .generate_texture_and_sampler_expressions(fragment_function, false);

        let occluded_ambient_reflected_luminance_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeOccludedAmbientReflectedLuminance",
            vec![
                position_texture_expr,
                position_sampler_expr,
                ambient_visibility_texture_expr,
                ambient_visibility_sampler_expr,
                texel_dimensions_expr,
                screen_space_texture_coord_expr,
                ambient_reflected_luminance_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        let output_rgba_color_expr = append_unity_component_to_vec3(
            &mut module.types,
            fragment_function,
            occluded_ambient_reflected_luminance_expr,
        );

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
