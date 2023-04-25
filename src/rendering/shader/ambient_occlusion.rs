//! Generation of shaders for ambient occlusion.

use super::{
    append_to_arena, append_unity_component_to_vec3, define_constant_if_missing, emit_in_func,
    include_expr_in_func, include_named_expr_in_func, insert_in_arena, new_name, u32_constant,
    CameraProjectionVariable, InputStruct, MeshVertexOutputFieldIndices, OutputStructBuilder,
    PushConstantFieldExpressions, SampledTexture, ShaderTricks, SourceCode, TextureType, F32_WIDTH,
    U32_TYPE, U32_WIDTH, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use crate::{
    rendering::{shader::F32_TYPE, RenderAttachmentQuantity, RENDER_ATTACHMENT_BINDINGS},
    scene::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
};
use naga::{
    AddressSpace, ArraySize, Expression, Function, GlobalVariable, Handle, Module, ResourceBinding,
    StructMember, Type, TypeInner,
};

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
pub struct AmbientOcclusionShaderGenerator<'a> {
    input: &'a AmbientOcclusionShaderInput,
}

impl<'a> AmbientOcclusionShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: ShaderTricks = ShaderTricks::NO_VERTEX_PROJECTION;

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
    /// ambient color and occlusion factor are taken as input. The ambient color
    /// is weighted with an averaged occlusion factor and written to the surface
    /// color attachment.
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
        let framebuffer_position_expr =
            fragment_input_struct.get_field_expr(mesh_input_field_indices.framebuffer_position);

        let screen_space_texture_coord_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "convertFramebufferPositionToScreenTextureCoords",
            vec![
                push_constant_fragment_expressions.inverse_window_dimensions,
                framebuffer_position_expr,
            ],
        );

        match self.input {
            AmbientOcclusionShaderInput::Calculation(input) => {
                Self::generate_fragment_code_for_computing_ambient_occlusion(
                    input,
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
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
                    framebuffer_position_expr,
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
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        camera_projection: Option<&CameraProjectionVariable>,
        framebuffer_position_expr: Handle<Expression>,
        screen_space_texture_coord_expr: Handle<Expression>,
    ) {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!((MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT % 2) == 0);

        const HALF_MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize =
            MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT / 2;

        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let half_max_sample_count_constant = define_constant_if_missing(
            &mut module.constants,
            u32_constant(HALF_MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u64),
        );

        let sample_offset_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: vec4_type,
                    size: ArraySize::Constant(half_max_sample_count_constant),
                    stride: VECTOR_4_SIZE,
                },
            },
        );

        let sample_offset_array_size =
            u32::try_from(HALF_MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT).unwrap() * VECTOR_4_SIZE;

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

        let (sample_offset_array_expr, sample_count_expr) =
            emit_in_func(fragment_function, |function| {
                let sample_offset_array_ptr_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: sample_struct_ptr_expr,
                        index: 0,
                    },
                );

                let sample_offset_array_expr = include_named_expr_in_func(
                    function,
                    "sampleOffsets",
                    Expression::Load {
                        pointer: sample_offset_array_ptr_expr,
                    },
                );

                let sample_count_ptr_expr = include_named_expr_in_func(
                    function,
                    "sampleCount",
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

                (sample_offset_array_expr, sample_count_expr)
            });

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

        let (position_texture_expr, position_sampler_expr) =
            position_texture.generate_texture_and_sampler_expressions(fragment_function, false);

        let position_expr = position_texture
            .generate_rgb_sampling_expr(fragment_function, screen_space_texture_coord_expr);

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

        let occlusion_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            &format!(
                "computeAmbientOcclusionMax{}Samples",
                MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT
            ),
            vec![
                position_texture_expr,
                position_sampler_expr,
                push_constant_fragment_expressions.inverse_window_dimensions,
                projection_matrix_expr,
                sample_offset_array_expr,
                sample_count_expr,
                position_expr,
                normal_vector_expr,
                random_angle_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "occlusion",
            f32_type,
            None,
            None,
            F32_WIDTH,
            occlusion_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }

    fn generate_fragment_code_for_applying_ambient_occlusion(
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        framebuffer_position_expr: Handle<Expression>,
        screen_space_texture_coord_expr: Handle<Expression>,
    ) {
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let (ambient_color_texture_binding, ambient_color_sampler_binding) =
            RENDER_ATTACHMENT_BINDINGS[RenderAttachmentQuantity::Color as usize];

        let ambient_color_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "ambientColor",
            *bind_group_idx,
            ambient_color_texture_binding,
            Some(ambient_color_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let ambient_color_expr = ambient_color_texture
            .generate_rgb_sampling_expr(fragment_function, screen_space_texture_coord_expr);

        let (occlusion_texture_binding, occlusion_sampler_binding) =
            RENDER_ATTACHMENT_BINDINGS[RenderAttachmentQuantity::Occlusion as usize];

        let occlusion_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "occlusion",
            *bind_group_idx,
            occlusion_texture_binding,
            Some(occlusion_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let (occlusion_texture_expr, occlusion_sampler_expr) =
            occlusion_texture.generate_texture_and_sampler_expressions(fragment_function, false);

        let noise_factor_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "generateInterleavedGradientNoiseFactor",
            vec![framebuffer_position_expr],
        );

        let occluded_ambient_color_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeOccludedAmbientColor",
            vec![
                occlusion_texture_expr,
                occlusion_sampler_expr,
                screen_space_texture_coord_expr,
                ambient_color_expr,
                noise_factor_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        let output_rgba_color_expr = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            occluded_ambient_color_expr,
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
