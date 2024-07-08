//! Generation of shaders for materials with a fixed color
//! or texture.

use super::{
    super::{
        append_unity_component_to_vec3, insert_in_arena, new_name, InputStruct,
        OutputStructBuilder, SampledTexture, TextureType, VECTOR_3_SIZE, VECTOR_3_TYPE,
        VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    MeshVertexOutputFieldIndices,
};
use naga::{Function, Interpolation, Module, Sampling};

/// Input description specifying the vertex attribute location reqired for
/// generating a shader for a fixed color material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedColorFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing color.
    pub color_location: u32,
}

/// Input description specifying the texture bindings required for generating a
/// shader for a fixed color material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedTextureShaderInput {
    /// Bind group bindings of the color texture and its sampler.
    pub color_texture_and_sampler_bindings: (u32, u32),
}

/// Shader generator for a fixed color material.
#[derive(Clone, Debug)]
pub(super) struct FixedColorShaderGenerator<'a> {
    feature_input: &'a FixedColorFeatureShaderInput,
}

/// Shader generator for a fixed color material.
#[derive(Clone, Debug)]
pub(super) struct FixedTextureShaderGenerator<'a> {
    texture_input: &'a FixedTextureShaderInput,
}

/// Index of the field holding the fixed color in the
/// vertex shader output struct.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub(super) struct FixedColorVertexOutputFieldIdx(usize);

impl<'a> FixedColorShaderGenerator<'a> {
    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(feature_input: &'a FixedColorFeatureShaderInput) -> Self {
        Self { feature_input }
    }

    /// Generates the vertex shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The fixed color is added as an input argument to the main
    /// vertex shader function and assigned to the output struct
    /// returned from the function.
    ///
    /// # Returns
    /// The index of the color field in the output struct, required
    /// for accessing the color in [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> FixedColorVertexOutputFieldIdx {
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let vertex_color_arg_expr = super::generate_location_bound_input_argument(
            vertex_function,
            new_name("color"),
            vec3_type,
            self.feature_input.color_location,
        );

        // Since the color is the same for every vertex, we don't need
        // perspective interpolation
        let output_color_field_idx = vertex_output_struct_builder.add_field(
            "color",
            vec3_type,
            Some(Interpolation::Flat),
            Some(Sampling::Center),
            VECTOR_3_SIZE,
            vertex_color_arg_expr,
        );

        FixedColorVertexOutputFieldIdx(output_color_field_idx)
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The fixed color passed from the main vertex shader function
    /// is simply returned from the main fragment shader function in
    /// an output struct.
    pub fn generate_fragment_code(
        module: &mut Module,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        color_input_field_idx: &FixedColorVertexOutputFieldIdx,
    ) {
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let vertex_color_expr = fragment_input_struct.get_field_expr(color_input_field_idx.0);

        let output_rgba_color_expr =
            append_unity_component_to_vec3(&mut module.types, fragment_function, vertex_color_expr);

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

impl<'a> FixedTextureShaderGenerator<'a> {
    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(texture_input: &'a FixedTextureShaderInput) -> Self {
        Self { texture_input }
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The color texture and sampler are declared as global variables,
    /// and a sampling expression is generated in the main fragment
    /// shader function. The sampled color is returned from the function
    /// in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let (color_texture_binding, color_sampler_binding) =
            self.texture_input.color_texture_and_sampler_bindings;

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let color_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image2D,
            "color",
            bind_group,
            color_texture_binding,
            Some(color_sampler_binding),
            None,
        );

        let color_sampling_expr = color_texture.generate_sampling_expr(
            fragment_function,
            fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .texture_coords
                    .expect("No `texture_coords` passed to fixed texture fragment shader"),
            ),
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
