//! Generation of shaders for sampling an input texture and passing the sampled
//! color to an output attachment.

use super::{
    insert_in_arena, InputStruct, MeshVertexOutputFieldIndices, OutputStructBuilder,
    PushConstantFieldExpressions, SampledTexture, ShaderTricks, SourceCode, TextureType,
    VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Function, Module};

/// Input description specifying the bindings for the texture to pass through to
/// the output attachment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PassthroughShaderInput {
    /// Bind group bindings of the input color texture and its sampler.
    pub input_texture_and_sampler_bindings: (u32, u32),
}

/// Generator for a passthrough shader.
#[derive(Clone, Debug)]
pub struct PassthroughShaderGenerator<'a> {
    input: &'a PassthroughShaderInput,
}

impl<'a> PassthroughShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: ShaderTricks = ShaderTricks::NO_VERTEX_PROJECTION;

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a PassthroughShaderInput) -> Self {
        Self { input }
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
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

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let (input_texture_binding, input_sampler_binding) =
            self.input.input_texture_and_sampler_bindings;

        let input_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "inputColor",
            *bind_group_idx,
            input_texture_binding,
            Some(input_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let input_color_expr = input_texture.generate_sampling_expr(
            fragment_function,
            screen_space_texture_coord_expr,
            None,
            None,
            None,
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            input_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
