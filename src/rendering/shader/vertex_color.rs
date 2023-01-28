//! Generation of shaders for materials with a fixed color
//! or texture.

use super::{
    insert_in_arena, InputStruct, MeshVertexOutputFieldIndices, OutputStructBuilder,
    VertexPropertyRequirements, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Function, Type, UniqueArena};

/// Shader generator for the case when vertex colors
/// included in the mesh are used to obtain the fragment
/// color.
#[derive(Copy, Clone, Debug)]
pub struct VertexColorShaderGenerator;

impl VertexColorShaderGenerator {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    pub const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::COLOR
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The interpolated vertex color passed from the main vertex shader
    /// function is simply returned from the main fragment shader function
    /// in an output struct.
    pub fn generate_fragment_code(
        types: &mut UniqueArena<Type>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            fragment_input_struct.get_field_expr_handle(
                mesh_input_field_indices
                    .color
                    .expect("No `color` passed to vertex color fragment shader"),
            ),
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }
}
