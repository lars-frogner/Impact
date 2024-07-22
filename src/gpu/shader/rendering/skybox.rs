//! Generation of shaders for rendering skyboxes.

use super::{
    super::{
        append_to_arena, emit_in_func, include_expr_in_func, include_named_expr_in_func,
        insert_in_arena, new_name, InputStruct, OutputStructBuilder, PushConstantExpressions,
        SampledTexture, TextureType, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE, VECTOR_3_TYPE,
        VECTOR_4_SIZE, VECTOR_4_TYPE,
    },
    MeshVertexInputExpressions, RenderShaderTricks,
};
use crate::gpu::push_constant::PushConstantVariant;
use naga::{
    AddressSpace, BinaryOperator, Expression, Function, GlobalVariable, Module, ResourceBinding,
    SampleLevel, StructMember, Type, TypeInner,
};

/// Input description specifying the texture bindings required for generating a
/// shader for rendering a skybox.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkyboxShaderInput {
    /// Bind group binding of the uniform holding the skybox properties.
    pub parameters_uniform_binding: u32,
    /// Bind group bindings of the skybox cubemap texture and its sampler.
    pub skybox_cubemap_texture_and_sampler_bindings: (u32, u32),
}

/// Generator for a skybox shader.
#[derive(Clone, Debug)]
pub(super) struct SkyboxShaderGenerator<'a> {
    input: &'a SkyboxShaderInput,
}

/// Indices of the fields holding the skybox properties in the vertex shader
/// output struct.
#[derive(Clone, Debug)]
pub(super) struct SkyboxVertexOutputFieldIndices {
    model_space_position: usize,
}

impl<'a> SkyboxShaderGenerator<'a> {
    /// The [`ShaderTricks`] employed by the material.
    pub const TRICKS: RenderShaderTricks = RenderShaderTricks::FOLLOW_CAMERA
        .union(RenderShaderTricks::DRAW_AT_MAX_DEPTH)
        .union(RenderShaderTricks::NO_JITTER);

    /// Creates a new shader generator using the given input description.
    pub fn new(input: &'a SkyboxShaderInput) -> Self {
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
    /// The parameter uniform, skybox cubemap texture and sampler are declared
    /// as global variables, and a sampling expression is generated in the main
    /// fragment shader function. The sampled color is scaled by the maximum
    /// possible luminance (from the uniform) and the exposure (from a push
    /// constant) and returned from the function in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        push_constant_fragment_expressions: &PushConstantExpressions,
        fragment_input_struct: &InputStruct,
        material_input_field_indices: &SkyboxVertexOutputFieldIndices,
    ) {
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);

        let parameters_struct_size = F32_WIDTH + 12;

        let parameters_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("SkyboxParameters"),
                inner: TypeInner::Struct {
                    members: vec![StructMember {
                        name: new_name("maxLuminance"),
                        ty: f32_type,
                        binding: None,
                        offset: 0,
                    }],
                    span: parameters_struct_size,
                },
            },
        );

        let parameters_struct_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("skyboxParameters"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: self.input.parameters_uniform_binding,
                }),
                ty: parameters_struct_type,
                init: None,
            },
        );

        *bind_group_idx += 1;

        let parameters_struct_ptr_expr = include_expr_in_func(
            fragment_function,
            Expression::GlobalVariable(parameters_struct_var),
        );

        let max_luminance_expr = emit_in_func(fragment_function, |function| {
            let max_luminance_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: parameters_struct_ptr_expr,
                    index: 0,
                },
            );
            include_named_expr_in_func(
                function,
                "maxLuminance",
                Expression::Load {
                    pointer: max_luminance_ptr_expr,
                },
            )
        });

        let (skybox_cubemap_texture_binding, skybox_cubemap_sampler_binding) =
            self.input.skybox_cubemap_texture_and_sampler_bindings;

        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let skybox_cubemap_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::ImageCubemap,
            "skybox",
            *bind_group_idx,
            skybox_cubemap_texture_binding,
            Some(skybox_cubemap_sampler_binding),
            None,
        );

        *bind_group_idx += 1;

        let color_sampling_expr = skybox_cubemap_texture.generate_sampling_expr(
            fragment_function,
            fragment_input_struct.get_field_expr(material_input_field_indices.model_space_position),
            SampleLevel::Auto,
            None,
            None,
            None,
        );

        let pre_exposed_luminance_color_expr = emit_in_func(fragment_function, |function| {
            let scaling_factor_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: max_luminance_expr,
                    right: push_constant_fragment_expressions
                        .get(PushConstantVariant::Exposure)
                        .expect("Missing exposure push constant for skybox"),
                },
            );
            include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: scaling_factor_expr,
                    right: color_sampling_expr,
                },
            )
        });

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "luminance",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            pre_exposed_luminance_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}
