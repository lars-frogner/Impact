//! Generation of shaders for material using vertex colors included in the mesh.

use super::{
    super::{
        append_unity_component_to_vec3, insert_in_arena, InputStruct, OutputStructBuilder,
        VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    MeshVertexOutputFieldIndices,
};
use naga::{Function, Module};

/// Shader generator for the case when vertex colors included in the mesh are
/// used to obtain the fragment color.
#[derive(Copy, Clone, Debug)]
pub(super) struct VertexColorShaderGenerator;

impl VertexColorShaderGenerator {
    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The interpolated vertex color passed from the main vertex shader
    /// function is simply returned from the main fragment shader function
    /// in an output struct.
    pub fn generate_fragment_code(
        module: &mut Module,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let vertex_color_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .color
                .expect("No `color` passed to vertex color fragment shader"),
        );

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
