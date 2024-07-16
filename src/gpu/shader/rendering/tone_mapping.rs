//! Generation of shaders for sampling an input texture and passing the sampled
//! color to an output attachment.

use super::{
    super::{
        insert_in_arena, InputStruct, OutputStructBuilder, SampledTexture, SourceCode, TextureType,
        VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    MeshVertexOutputFieldIndices, PushConstantExpressions, RenderShaderTricks,
};
use crate::gpu::{
    push_constant::PushConstantVariant,
    rendering::postprocessing::capturing::tone_mapping::ToneMapping,
};
use naga::{Function, Module, SampleLevel};

/// Input description specifying the bindings for the texture to pass through to
/// the output attachment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToneMappingShaderInput {
    /// The tone mapping method to use.
    pub mapping: ToneMapping,
    /// Bind group bindings of the input color texture and its sampler.
    pub input_texture_and_sampler_bindings: (u32, u32),
}

/// Generator for a tone mapping shader.
#[derive(Clone, Debug)]
pub(super) struct ToneMappingShaderGenerator<'a> {
    input: &'a ToneMappingShaderInput,
}

impl ToneMapping {
    fn function_name(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::ACES => Some("toneMapACES"),
            Self::KhronosPBRNeutral => Some("toneMapKhronosPBRNeutral"),
        }
    }
}

impl<'a> ToneMappingShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: RenderShaderTricks = RenderShaderTricks::NO_VERTEX_PROJECTION;

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a ToneMappingShaderInput) -> Self {
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
        push_constant_fragment_expressions: &PushConstantExpressions,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let inverse_window_dimensions_expr = push_constant_fragment_expressions
            .get(PushConstantVariant::InverseWindowDimensions)
            .expect("Missing inverse window dimensions push constant for tonemapping");

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
            SampleLevel::Zero,
            None,
            None,
            None,
        );

        let tone_mapped_color_expr = if let Some(function_name) = self.input.mapping.function_name()
        {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                function_name,
                vec![input_color_expr],
            )
        } else {
            input_color_expr
        };

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            tone_mapped_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
