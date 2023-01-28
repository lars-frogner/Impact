//! Generation of shaders for Blinn-Phong materials.

use super::{
    append_to_arena, emit, insert_in_arena, InputStruct, InputStructBuilder,
    MeshVertexOutputFieldIndices, OutputStructBuilder, SampledTexture, VertexPropertyRequirements,
    FLOAT32_WIDTH, FLOAT_TYPE, VECTOR_3_SIZE, VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Arena, Expression, Function, GlobalVariable, Handle, Type, UniqueArena};

/// Input description specifying the vertex attribute locations
/// of Blinn-Phong material properties, reqired for generating a
/// shader for a [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing ambient color.
    pub ambient_color_location: u32,
    /// Vertex attribute location for the instance feature
    /// representing diffuse color. If [`None`], diffuse
    /// color is obtained from a texture instead.
    pub diffuse_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing specular color. If [`None`], specular
    /// color is obtained from a texture instead.
    pub specular_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing shininess.
    pub shininess_location: u32,
    /// Vertex attribute location for the instance feature
    /// representing alpha.
    pub alpha_location: u32,
}

/// Input description specifying the bindings of textures
/// for Blinn-Phong properties, required for generating a
/// shader for a
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlinnPhongTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and
    /// its sampler.
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    /// Bind group bindings of the specular color texture and
    /// its sampler. If [`None`], specular color is an instance
    /// feature instead.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a
/// [`BlinnPhongMaterial`](crate::scene::BlinnPhongMaterial),
/// [`DiffuseTexturedBlinnPhongMaterial`](crate::scene::DiffuseTexturedBlinnPhongMaterial)
/// or a [`TexturedBlinnPhongMaterial`](crate::scene::TexturedBlinnPhongMaterial).
#[derive(Clone, Debug)]
pub struct BlinnPhongShaderGenerator<'a> {
    feature_input: &'a BlinnPhongFeatureShaderInput,
    texture_input: Option<&'a BlinnPhongTextureShaderInput>,
}

/// Indices of the fields holding the various Blinn-Phong
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct BlinnPhongVertexOutputFieldIndices {
    ambient_color: usize,
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    shininess: usize,
    alpha: usize,
}

impl<'a> BlinnPhongShaderGenerator<'a> {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    pub fn vertex_property_requirements(&self) -> VertexPropertyRequirements {
        if self.texture_input.is_some() {
            VertexPropertyRequirements::POSITION
                | VertexPropertyRequirements::NORMAL_VECTOR
                | VertexPropertyRequirements::TEXTURE_COORDS
        } else {
            VertexPropertyRequirements::POSITION | VertexPropertyRequirements::NORMAL_VECTOR
        }
    }

    /// Creates a new shader generator using the given input
    /// description.
    pub fn new(
        feature_input: &'a BlinnPhongFeatureShaderInput,
        texture_input: Option<&'a BlinnPhongTextureShaderInput>,
    ) -> Self {
        Self {
            feature_input,
            texture_input,
        }
    }

    /// Generates the vertex shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The struct of vertex buffered Blinn-Phong properties is added
    /// as an input argument to the main vertex shader function and its
    /// fields are assigned to the output struct returned from the function.
    ///
    /// # Returns
    /// The indices of the Blinn-Phong property fields in the output
    /// struct, required for accessing the properties in
    /// [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> BlinnPhongVertexOutputFieldIndices {
        let float_type_handle = insert_in_arena(types, FLOAT_TYPE);
        let vec3_type_handle = insert_in_arena(types, VECTOR_3_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("MaterialProperties", "material");

        let input_ambient_color_field_idx = input_struct_builder.add_field(
            "ambientColor",
            vec3_type_handle,
            self.feature_input.ambient_color_location,
            VECTOR_3_SIZE,
        );

        let input_diffuse_color_field_idx =
            self.feature_input.diffuse_color_location.map(|location| {
                input_struct_builder.add_field(
                    "diffuseColor",
                    vec3_type_handle,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_specular_color_field_idx =
            self.feature_input.specular_color_location.map(|location| {
                input_struct_builder.add_field(
                    "specularColor",
                    vec3_type_handle,
                    location,
                    VECTOR_3_SIZE,
                )
            });

        let input_shininess_field_idx = input_struct_builder.add_field(
            "shininess",
            float_type_handle,
            self.feature_input.shininess_location,
            FLOAT32_WIDTH,
        );

        let input_alpha_field_idx = input_struct_builder.add_field(
            "alpha",
            float_type_handle,
            self.feature_input.alpha_location,
            FLOAT32_WIDTH,
        );

        let input_struct = input_struct_builder.generate_input_code(types, vertex_function);

        let output_ambient_color_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "ambientColor",
                vec3_type_handle,
                VECTOR_3_SIZE,
                input_struct.get_field_expr_handle(input_ambient_color_field_idx),
            );

        let output_shininess_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "shininess",
                float_type_handle,
                FLOAT32_WIDTH,
                input_struct.get_field_expr_handle(input_shininess_field_idx),
            );

        let output_alpha_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "alpha",
                float_type_handle,
                FLOAT32_WIDTH,
                input_struct.get_field_expr_handle(input_alpha_field_idx),
            );

        let mut indices = BlinnPhongVertexOutputFieldIndices {
            ambient_color: output_ambient_color_field_idx,
            diffuse_color: None,
            specular_color: None,
            shininess: output_shininess_field_idx,
            alpha: output_alpha_field_idx,
        };

        if let Some(idx) = input_diffuse_color_field_idx {
            indices.diffuse_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "diffuseColor",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        if let Some(idx) = input_specular_color_field_idx {
            indices.specular_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "specularColor",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        indices
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The texture and sampler for any material properties sampled
    /// from textured are declared as global variables, and sampling
    /// expressions are generated in the main fragment shader function.
    /// These are used together with material properties passed from the
    /// main vertex shader to generate the Blinn-Phong shading equation,
    /// whose output color is returned from the main fragment shader
    /// function in an output struct.
    pub fn generate_fragment_code(
        &self,
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &BlinnPhongVertexOutputFieldIndices,
    ) {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let ambient_color_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.ambient_color);

        let shininess_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.shininess);

        let alpha_expr_handle =
            fragment_input_struct.get_field_expr_handle(material_input_field_indices.alpha);

        let (diffuse_color_expr_handle, specular_color_expr_handle) =
            if let Some(texture_input) = self.texture_input {
                let (diffuse_color_expr_handle, specular_color_expr_handle) =
                    Self::generate_texture_fragment_code(
                        texture_input,
                        types,
                        global_variables,
                        fragment_function,
                        fragment_input_struct,
                        mesh_input_field_indices,
                    );
                (
                    diffuse_color_expr_handle,
                    specular_color_expr_handle.unwrap_or_else(|| {
                        fragment_input_struct.get_field_expr_handle(
                            material_input_field_indices.specular_color.expect(
                                "Missing `specular_color` feature for Blinn-Phong material",
                            ),
                        )
                    }),
                )
            } else {
                (
                    fragment_input_struct.get_field_expr_handle(
                        material_input_field_indices
                            .diffuse_color
                            .expect("Missing `diffuse_color` feature for Blinn-Phong material"),
                    ),
                    fragment_input_struct.get_field_expr_handle(
                        material_input_field_indices
                            .specular_color
                            .expect("Missing `specular_color` feature for Blinn-Phong material"),
                    ),
                )
            };

        let color_expr_handle = ambient_color_expr_handle;

        let output_color_expr_handle = emit(
            &mut fragment_function.body,
            &mut fragment_function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Compose {
                        ty: vec4_type_handle,
                        components: vec![color_expr_handle, alpha_expr_handle],
                    },
                )
            },
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            output_color_expr_handle,
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }

    fn generate_texture_fragment_code(
        texture_input: &BlinnPhongTextureShaderInput,
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) -> (Handle<Expression>, Option<Handle<Expression>>) {
        let (diffuse_texture_binding, diffuse_sampler_binding) =
            texture_input.diffuse_texture_and_sampler_bindings;

        let diffuse_color_texture = SampledTexture::declare(
            types,
            global_variables,
            "diffuseColor",
            1,
            diffuse_texture_binding,
            diffuse_sampler_binding,
        );

        let texture_coord_expr_handle = fragment_input_struct.get_field_expr_handle(
            mesh_input_field_indices
                .texture_coords
                .expect("No `texture_coords` passed to fixed texture fragment shader"),
        );

        let diffuse_color_sampling_expr_handle = diffuse_color_texture
            .generate_sampling_expr(fragment_function, texture_coord_expr_handle);

        let specular_color_sampling_expr_handle = texture_input
            .specular_texture_and_sampler_bindings
            .map(|(specular_texture_binding, specular_sampler_binding)| {
                let specular_color_texture = SampledTexture::declare(
                    types,
                    global_variables,
                    "specularColor",
                    1,
                    specular_texture_binding,
                    specular_sampler_binding,
                );

                specular_color_texture
                    .generate_sampling_expr(fragment_function, texture_coord_expr_handle)
            });

        (
            diffuse_color_sampling_expr_handle,
            specular_color_sampling_expr_handle,
        )
    }
}
