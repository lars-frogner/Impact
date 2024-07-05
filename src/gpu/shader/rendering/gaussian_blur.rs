//! Generation of shaders for Gaussian blur.

use super::{
    super::{
        append_to_arena, emit, emit_in_func, include_expr_in_func, include_named_expr_in_func,
        insert_in_arena, new_name, push_to_block, swizzle_xy_expr, ForLoop, InputStruct,
        OutputStructBuilder, SampledTexture, SourceCode, TextureType, U32_TYPE, U32_WIDTH,
        VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    MeshVertexOutputFieldIndices, PushConstantFieldExpressions, RenderShaderTricks,
};
use crate::scene::{GaussianBlurDirection, MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS};
use naga::{
    AddressSpace, ArraySize, BinaryOperator, Expression, Function, GlobalVariable, Literal,
    LocalVariable, Module, ResourceBinding, Statement, StructMember, Type, TypeInner,
};
use std::num::NonZeroU32;

/// Input description specifying the direction and uniform bindings for Gaussian
/// blur.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GaussianBlurShaderInput {
    /// The direction in which to apply the 1D blur.
    pub direction: GaussianBlurDirection,
    /// Bind group binding of the uniform containing sample offsets and weights.
    pub sample_uniform_binding: u32,
    /// Bind group bindings of the input color texture and its sampler.
    pub input_texture_and_sampler_bindings: (u32, u32),
}

/// Generator for a Gaussian blur shader.
#[derive(Clone, Debug)]
pub(super) struct GaussianBlurShaderGenerator<'a> {
    input: &'a GaussianBlurShaderInput,
}

impl GaussianBlurDirection {
    fn compute_single_sample_color_function_name(&self) -> &'static str {
        match self {
            Self::Horizontal => "computeSingleHorizontalGaussianBlurSampleColor",
            Self::Vertical => "computeSingleVerticalGaussianBlurSampleColor",
        }
    }

    fn compute_symmetric_sample_color_function_name(&self) -> &'static str {
        match self {
            Self::Horizontal => "computeSymmetricHorizontalGaussianBlurSampleColor",
            Self::Vertical => "computeSymmetricVerticalGaussianBlurSampleColor",
        }
    }
}

impl<'a> GaussianBlurShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: RenderShaderTricks = RenderShaderTricks::NO_VERTEX_PROJECTION;

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a GaussianBlurShaderInput) -> Self {
        Self { input }
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The input texture is sampled around the current fragment at 1-pixel
    /// intervals in either the horizontal or vertical direction. The samples
    /// are weighted with Gaussian weights and added to produce the output
    /// color.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let sample_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: vec4_type,
                    size: ArraySize::Constant(
                        NonZeroU32::new(u32::try_from(MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS).unwrap())
                            .unwrap(),
                    ),
                    stride: VECTOR_4_SIZE,
                },
            },
        );

        let sample_array_size =
            u32::try_from(MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS).unwrap() * VECTOR_4_SIZE;

        // The last `12` is padding
        let sample_struct_size = sample_array_size + U32_WIDTH + 12;

        let sample_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("GaussianBlurSamples"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("sampleOffsetsAndWeights"),
                            ty: sample_array_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("sampleCount"),
                            ty: u32_type,
                            binding: None,
                            offset: sample_array_size,
                        },
                        // <-- The rest of the struct is for padding an not
                        // needed in the shader
                    ],
                    span: sample_struct_size,
                },
            },
        );

        let sample_struct_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("gaussianBlurSamples"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: self.input.sample_uniform_binding,
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

        let inverse_window_dimensions_expr =
            push_constant_fragment_expressions.inverse_window_dimensions;

        let framebuffer_position_expr =
            fragment_input_struct.get_field_expr(mesh_input_field_indices.framebuffer_position);

        let (sample_array_ptr_expr, sample_count_expr, fragment_coords_expr) =
            emit_in_func(fragment_function, |function| {
                let sample_array_ptr_expr = include_expr_in_func(
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
                let fragment_coords_expr = include_named_expr_in_func(
                    function,
                    "fragmentCoords",
                    swizzle_xy_expr(framebuffer_position_expr),
                );
                (
                    sample_array_ptr_expr,
                    sample_count_expr,
                    fragment_coords_expr,
                )
            });

        let (input_texture_binding, input_sampler_binding) =
            self.input.input_texture_and_sampler_bindings;

        let input_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "input",
            *bind_group_idx,
            input_texture_binding,
            Some(input_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let (input_texture_expr, input_sampler_expr) =
            input_texture.generate_texture_and_sampler_expressions(fragment_function, false);

        let zero_idx_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::Literal(Literal::U32(0)),
        );

        let center_sample_offset_and_weight_expr = emit_in_func(fragment_function, |function| {
            let sample_offset_and_weight_ptr_expr = include_expr_in_func(
                function,
                Expression::Access {
                    base: sample_array_ptr_expr,
                    index: zero_idx_expr,
                },
            );
            include_expr_in_func(
                function,
                Expression::Load {
                    pointer: sample_offset_and_weight_ptr_expr,
                },
            )
        });

        let center_sample_color_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            self.input
                .direction
                .compute_single_sample_color_function_name(),
            vec![
                input_texture_expr,
                input_sampler_expr,
                inverse_window_dimensions_expr,
                fragment_coords_expr,
                center_sample_offset_and_weight_expr,
            ],
        );

        let averaged_color_ptr_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut fragment_function.local_variables,
                LocalVariable {
                    name: new_name("averagedColor"),
                    ty: vec4_type,
                    init: None,
                },
            )),
        );
        push_to_block(
            &mut fragment_function.body,
            Statement::Store {
                pointer: averaged_color_ptr_expr,
                value: center_sample_color_expr,
            },
        );

        let one_idx_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::Literal(Literal::U32(1)),
        );
        let mut sampling_loop = ForLoop::new(
            &mut module.types,
            fragment_function,
            "sample",
            Some(one_idx_expr),
            sample_count_expr,
        );

        let sample_offset_and_weight_expr = emit(
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let sample_offset_and_weight_ptr_expr = append_to_arena(
                    expressions,
                    Expression::Access {
                        base: sample_array_ptr_expr,
                        index: sampling_loop.idx_expr,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: sample_offset_and_weight_ptr_expr,
                    },
                )
            },
        );

        let sample_color_expr = source_code_lib.generate_function_call_in_block(
            module,
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            self.input
                .direction
                .compute_symmetric_sample_color_function_name(),
            vec![
                input_texture_expr,
                input_sampler_expr,
                inverse_window_dimensions_expr,
                fragment_coords_expr,
                sample_offset_and_weight_expr,
            ],
        );

        let summed_sample_colors_expr = emit(
            &mut sampling_loop.body,
            &mut fragment_function.expressions,
            |expressions| {
                let prev_summed_sample_colors_expr = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: averaged_color_ptr_expr,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: prev_summed_sample_colors_expr,
                        right: sample_color_expr,
                    },
                )
            },
        );

        push_to_block(
            &mut sampling_loop.body,
            Statement::Store {
                pointer: averaged_color_ptr_expr,
                value: summed_sample_colors_expr,
            },
        );

        sampling_loop.generate_code(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
        );

        let averaged_color_expr = emit_in_func(fragment_function, |function| {
            include_expr_in_func(
                function,
                Expression::Load {
                    pointer: averaged_color_ptr_expr,
                },
            )
        });

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            averaged_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
