//! Generation of shaders for rendering skyboxes.

use super::{
    insert_in_arena, InputStruct, MeshVertexInputExpressions, OutputStructBuilder, SampledTexture,
    ShaderTricks, TextureType, VECTOR_3_SIZE, VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Function, Module};

/// Input description specifying the texture bindings required for generating a
/// shader for rendering a skybox.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkyboxTextureShaderInput {
    /// Bind group bindings of the skybox cubemap texture and its sampler.
    pub skybox_cubemap_texture_and_sampler_bindings: (u32, u32),
}

/// Generator for a skybox shader.
#[derive(Clone, Debug)]
pub struct SkyboxShaderGenerator<'a> {
    input: &'a SkyboxTextureShaderInput,
}

/// Indices of the fields holding the skybox properties in the vertex shader
/// output struct.
#[derive(Clone, Debug)]
pub struct SkyboxVertexOutputFieldIndices {
    model_space_position: usize,
}

impl<'a> SkyboxShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: ShaderTricks =
        ShaderTricks::FOLLOW_CAMERA.union(ShaderTricks::DRAW_AT_MAX_DEPTH);

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a SkyboxTextureShaderInput) -> Self {
        Self { input }
    }

    /// Generates the vertex shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The model space position passed to the main vertex shader function is
    /// assigned to the output struct returned from the function.
    ///
    /// # Returns
    /// The index of the model space position field in the output struct,
    /// required for accessing the position in [`generate_fragment_code`].
    #[allow(clippy::unused_self)]
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        mesh_vertex_input_expressions: &MeshVertexInputExpressions,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> SkyboxVertexOutputFieldIndices {
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let output_position_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "modelSpacePosition",
                vec3_type,
                VECTOR_3_SIZE,
                mesh_vertex_input_expressions.position,
            );

        SkyboxVertexOutputFieldIndices {
            model_space_position: output_position_field_idx,
        }
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The skybox cubemap texture and sampler are declared as global variables,
    /// and a sampling expression is generated in the main fragment shader
    /// function. The sampled color is returned from the function in an output
    /// struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        material_input_field_indices: &SkyboxVertexOutputFieldIndices,
    ) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let (skybox_cubemap_texture_binding, skybox_cubemap_sampler_binding) =
            self.input.skybox_cubemap_texture_and_sampler_bindings;

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let skybox_cubemap_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::ImageCubemap,
            "skybox",
            bind_group,
            skybox_cubemap_texture_binding,
            Some(skybox_cubemap_sampler_binding),
            None,
        );

        let color_sampling_expr = skybox_cubemap_texture.generate_sampling_expr(
            fragment_function,
            fragment_input_struct.get_field_expr(material_input_field_indices.model_space_position),
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
            color_sampling_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
