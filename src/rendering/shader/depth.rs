//! Generation of shaders for materials visualizing fragment depth.

use super::{
    define_constant_if_missing, emit_in_func, float32_constant, include_expr_in_func,
    insert_in_arena, InputStruct, LightVertexOutputFieldIndices, OutputStructBuilder,
    VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{BinaryOperator, Expression, Function, Module};

/// Shader generator for a
/// [`LightSpaceDepthMaterial`](crate::scene::LightSpaceDepthMaterial).
#[derive(Clone, Debug)]
pub struct LightSpaceDepthShaderGenerator;

impl LightSpaceDepthShaderGenerator {
    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The z-component of the interpolated light clip space position passed
    /// from the main vertex shader function is simply returned as a grayscale
    /// color from the main fragment shader function in an output struct.
    pub fn generate_fragment_code(
        module: &mut Module,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
    ) {
        let light_input_field_indices =
            light_input_field_indices.expect("Missing light for visualizing light space depth");

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        let light_clip_space_position_expr = match light_input_field_indices {
            LightVertexOutputFieldIndices::PointLight => unimplemented!(
                "Light clip space depth visualization is not supported for point lights"
            ),
            LightVertexOutputFieldIndices::DirectionalLight(light_input_field_indices) => {
                fragment_input_struct.get_field_expr(light_input_field_indices.light_clip_position)
            }
        };

        let unity_constant_expr = include_expr_in_func(
            fragment_function,
            Expression::Constant(define_constant_if_missing(
                &mut module.constants,
                float32_constant(1.0),
            )),
        );

        let half_constant_expr = include_expr_in_func(
            fragment_function,
            Expression::Constant(define_constant_if_missing(
                &mut module.constants,
                float32_constant(0.5),
            )),
        );

        let color_expr = emit_in_func(fragment_function, |function| {
            let depth_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_clip_space_position_expr,
                    index: 2,
                },
            );

            let depth_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Add,
                    left: depth_expr,
                    right: unity_constant_expr,
                },
            );

            let depth_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: depth_expr,
                    right: half_constant_expr,
                },
            );

            include_expr_in_func(
                function,
                Expression::Compose {
                    ty: vec4_type,
                    components: vec![depth_expr, depth_expr, depth_expr, unity_constant_expr],
                },
            )
        });

        output_struct_builder.add_field("color", vec4_type, None, None, VECTOR_4_SIZE, color_expr);

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
