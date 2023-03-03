//! Generation of shaders for materials with a global ambient color.

use super::{
    append_to_arena, define_constant_if_missing, emit_in_func, float32_constant,
    include_expr_in_func, include_named_expr_in_func, insert_in_arena, new_name,
    OutputStructBuilder, VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{AddressSpace, Expression, Function, GlobalVariable, Module, ResourceBinding};

/// Input description specifying the uniform binding reqired for generating a
/// shader for a
/// [`GlobalAmbientColorMaterial`](crate::scene::GlobalAmbientColorMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalAmbientColorShaderInput {
    /// Bind group binding of the uniform buffer holding the global ambient color.
    pub uniform_binding: u32,
}

/// Shader generator for a
/// [`GlobalAmbientColorMaterial`](crate::scene::GlobalAmbientColorMaterial).
#[derive(Clone, Debug)]
pub struct GlobalAmbientColorShaderGenerator<'a> {
    input: &'a GlobalAmbientColorShaderInput,
}

impl<'a> GlobalAmbientColorShaderGenerator<'a> {
    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(input: &'a GlobalAmbientColorShaderInput) -> Self {
        Self { input }
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The global ambient color is declared as a global uniform variable, and
    /// is returned from the function in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
    ) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let ambient_color_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("ambientColor"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: bind_group,
                    binding: self.input.uniform_binding,
                }),
                ty: vec3_type_handle,
                init: None,
            },
        );

        let ambient_color_ptr_expr_handle = include_expr_in_func(
            fragment_function,
            Expression::GlobalVariable(ambient_color_var_handle),
        );

        let unity_constant_expr = include_expr_in_func(
            fragment_function,
            Expression::Constant(define_constant_if_missing(
                &mut module.constants,
                float32_constant(1.0),
            )),
        );

        let ambient_rgba_color_expr_handle = emit_in_func(fragment_function, |function| {
            let ambient_color_expr_handle = include_named_expr_in_func(
                function,
                "ambientColor",
                Expression::Load {
                    pointer: ambient_color_ptr_expr_handle,
                },
            );

            include_expr_in_func(
                function,
                Expression::Compose {
                    ty: vec4_type_handle,
                    components: vec![ambient_color_expr_handle, unity_constant_expr],
                },
            )
        });

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            ambient_rgba_color_expr_handle,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
