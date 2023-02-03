//! Generation of shaders for materials with a fixed color
//! or texture.

use super::{
    append_to_arena, insert_in_arena, new_name, InputStruct, MeshVertexOutputFieldIndices,
    OutputStructBuilder, SampledTexture, VertexPropertyRequirements, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{
    Arena, Binding, Expression, Function, FunctionArgument, GlobalVariable, Interpolation,
    Sampling, Type, UniqueArena,
};

/// Input description specifying the vertex attribute location
/// reqired for generating a shader for a
/// [`FixedColorMaterial`](crate::scene::FixedColorMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedColorFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing color.
    pub color_location: u32,
}

/// Input description specifying the texture bindings required
/// for generating a shader for a
/// [`FixedTextureMaterial`](crate::scene::FixedTextureMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedTextureShaderInput {
    /// Bind group bindings of the color texture and its sampler.
    pub color_texture_and_sampler_bindings: (u32, u32),
}

/// Shader generator for a
/// [`FixedColorMaterial`](crate::scene::FixedColorMaterial).
#[derive(Clone, Debug)]
pub struct FixedColorShaderGenerator<'a> {
    feature_input: &'a FixedColorFeatureShaderInput,
}

/// Shader generator for a
/// [`FixedTextureMaterial`](crate::scene::FixedTextureMaterial).
#[derive(Clone, Debug)]
pub struct FixedTextureShaderGenerator<'a> {
    texture_input: &'a FixedTextureShaderInput,
}

/// Index of the field holding the fixed color in the
/// vertex shader output struct.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct FixedColorVertexOutputFieldIdx(usize);

impl<'a> FixedColorShaderGenerator<'a> {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    pub const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::empty()
    }

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
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> FixedColorVertexOutputFieldIdx {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let color_arg_idx = u32::try_from(vertex_function.arguments.len()).unwrap();

        vertex_function.arguments.push(FunctionArgument {
            name: new_name("color"),
            ty: vec4_type_handle,
            binding: Some(Binding::Location {
                location: self.feature_input.color_location,
                interpolation: None,
                sampling: None,
            }),
        });

        let vertex_color_arg_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::FunctionArgument(color_arg_idx),
        );

        // Since the color is the same for every vertex, we don't need
        // perspective interpolation
        let output_color_field_idx = vertex_output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            Some(Interpolation::Flat),
            Some(Sampling::Center),
            VECTOR_4_SIZE,
            vertex_color_arg_ptr_expr_handle,
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
        types: &mut UniqueArena<Type>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        color_input_field_idx: &FixedColorVertexOutputFieldIdx,
    ) {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            fragment_input_struct.get_field_expr_handle(color_input_field_idx.0),
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }
}

impl<'a> FixedTextureShaderGenerator<'a> {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    pub const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::TEXTURE_COORDS
    }

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
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let (color_texture_binding, color_sampler_binding) =
            self.texture_input.color_texture_and_sampler_bindings;

        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let color_texture = SampledTexture::declare(
            types,
            global_variables,
            "color",
            bind_group,
            color_texture_binding,
            color_sampler_binding,
        );

        let color_sampling_expr_handle = color_texture.generate_sampling_expr(
            fragment_function,
            fragment_input_struct.get_field_expr_handle(
                mesh_input_field_indices
                    .texture_coords
                    .expect("No `texture_coords` passed to fixed texture fragment shader"),
            ),
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            color_sampling_expr_handle,
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }
}
