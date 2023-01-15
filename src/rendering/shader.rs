//! Graphics shaders.

use crate::rendering::CoreRenderingSystem;
use std::{borrow::Cow, fmt::Display, vec};

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::{fs, path::Path};
        use anyhow::Result;
    }
}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
}

pub struct ShaderBuilder;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraShaderInput {
    pub view_proj_matrix_binding: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshShaderInput {
    pub position_location: u32,
    pub normal_vector_location: Option<u32>,
    pub texture_coord_location: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceFeatureShaderInput {
    ModelInstanceTransform(ModelInstanceTransformShaderInput),
    FixedColorMaterial(FixedColorFeatureShaderInput),
    BlinnPhongMaterial(BlinnPhongFeatureShaderInput),
    #[cfg(test)]
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInstanceTransformShaderInput {
    pub model_matrix_locations: (u32, u32, u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedColorFeatureShaderInput {
    pub color_location: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlinnPhongFeatureShaderInput {
    pub ambient_color_location: u32,
    pub diffuse_color_location: Option<u32>,
    pub specular_color_location: Option<u32>,
    pub shininess_location: u32,
    pub alpha_location: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialTextureShaderInput {
    BlinnPhongMaterial(BlinnPhongTextureShaderInput),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlinnPhongTextureShaderInput {
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformShaderInput {}

struct VertexShaderBuilder {
    vertex_buffer_bindings: Vec<VertexBufferBinding>,
    uniform_bindings: Vec<UniformBinding>,
    variable_definitions: Vec<VariableDefinition>,
    outputs: Option<Outputs>,
}

struct FragmentShaderBuilder {
    texture_2d_bindings: Vec<Texture2DBinding>,
    uniform_bindings: Vec<UniformBinding>,
    variable_definitions: Vec<VariableDefinition>,
    outputs: Option<Outputs>,
}

struct VertexBufferBinding {
    struct_name: &'static str,
    struct_fields: Vec<VertexBufferBindingField>,
    input_arg_ident: &'static str,
}

struct VertexBufferBindingField {
    location: u32,
    ident: &'static str,
    ty: &'static str,
}

struct UniformBinding {
    declaration: String,
    variables: Vec<&'static str>,
}

struct Texture2DBinding {
    group: u32,
    texture_binding: u32,
    sampler_binding: u32,
    texture_ident: &'static str,
    sampler_ident: &'static str,
}

struct VariableDefinition {
    ident: &'static str,
    expr: Cow<'static, str>,
}

struct Outputs {
    struct_name: &'static str,
    struct_ident: &'static str,
    builtin_fields: Vec<BuiltinOutputField>,
    fields: Vec<OutputField>,
}

struct BuiltinOutputField {
    builtin_flag: &'static str,
    ident: &'static str,
    ty: &'static str,
    expr: Cow<'static, str>,
}

struct OutputField {
    ident: &'static str,
    ty: &'static str,
    expr: Cow<'static, str>,
}

struct FragmentOutputs {}

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(
        core_system: &CoreRenderingSystem,
        shader_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let shader_path = shader_path.as_ref();
        let label = shader_path.to_string_lossy();
        let source = fs::read_to_string(shader_path)?;
        Ok(Self::from_source(core_system, &source, label.as_ref()))
    }

    /// Creates a new shader from the given source code.
    pub fn from_source(core_system: &CoreRenderingSystem, source: &str, label: &str) -> Self {
        let module = core_system
            .device()
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
                label: Some(label),
            });
        Self { module }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
}

impl ShaderBuilder {
    pub fn build_shader_source(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&MaterialTextureShaderInput>,
    ) {
    }
}

impl VertexShaderBuilder {
    fn new() -> Self {
        Self {
            vertex_buffer_bindings: Vec::new(),
            uniform_bindings: Vec::new(),
            variable_definitions: Vec::new(),
            outputs: None,
        }
    }

    fn add_vertex_buffer_binding(&mut self, binding: VertexBufferBinding) {
        self.vertex_buffer_bindings.push(binding);
    }

    fn add_uniform_binding(&mut self, binding: UniformBinding) {
        self.uniform_bindings.push(binding);
    }

    fn add_variable_definition(&mut self, definition: VariableDefinition) {
        self.variable_definitions.push(definition);
    }

    fn set_outputs(&mut self, outputs: Outputs) {
        self.outputs = Some(outputs);
    }

    fn build(&self) -> Option<String> {
        if let Some(outputs) = &self.outputs {
            let (vertex_struct_definitions, input_declarations): (Vec<_>, Vec<_>) = self
                .vertex_buffer_bindings
                .iter()
                .map(|binding| (binding.struct_definition(), binding.argument_declaration()))
                .unzip();

            let uniform_declarations: Vec<_> = self
                .uniform_bindings
                .iter()
                .map(|binding| binding.declaration.as_str())
                .collect();

            let variable_definitions: Vec<_> = self
                .variable_definitions
                .iter()
                .map(VariableDefinition::statement)
                .collect();

            Some(format!(
                "\
                {vertex_struct_definitions}\n\
                \n\
                {uniform_declarations}\n\
                \n\
                {output_struct_declaration}\n\
                \n\
                [[stage(vertex)]]\n\
                fn vs_main({input_declarations}) -> {output_struct_name} {{\n\
                    {variable_definitions}\n\
                    {output_struct_assignments_and_return}\n\
                }}\
            ",
                vertex_struct_definitions = vertex_struct_definitions.join("\n\n"),
                uniform_declarations = uniform_declarations.join("\n\n"),
                output_struct_declaration = outputs.struct_declaration(),
                input_declarations = input_declarations.join(", "),
                output_struct_name = outputs.struct_name,
                variable_definitions = variable_definitions.join("\n"),
                output_struct_assignments_and_return = outputs.struct_assignments_and_return()
            ))
        } else {
            None
        }
    }
}

impl FragmentShaderBuilder {
    fn new() -> Self {
        Self {
            texture_2d_bindings: Vec::new(),
            uniform_bindings: Vec::new(),
            variable_definitions: Vec::new(),
            outputs: None,
        }
    }

    fn add_texture_2d_binding(&mut self, binding: Texture2DBinding) {
        self.texture_2d_bindings.push(binding);
    }

    fn add_uniform_binding(&mut self, binding: UniformBinding) {
        self.uniform_bindings.push(binding);
    }

    fn add_variable_definition(&mut self, definition: VariableDefinition) {
        self.variable_definitions.push(definition);
    }

    fn set_outputs(&mut self, outputs: Outputs) {
        self.outputs = Some(outputs);
    }

    fn build(&self, outputs_from_vertex: Outputs) -> Option<String> {
        if let Some(outputs) = &self.outputs {
            let texture_2d_definitions: Vec<_> = self
                .texture_2d_bindings
                .iter()
                .map(Texture2DBinding::declarations)
                .collect();

            let uniform_declarations: Vec<_> = self
                .uniform_bindings
                .iter()
                .map(|binding| binding.declaration.as_str())
                .collect();

            let variable_definitions: Vec<_> = self
                .variable_definitions
                .iter()
                .map(VariableDefinition::statement)
                .collect();

            Some(format!(
                "\
                {texture_2d_definitions}\n\
                \n\
                {uniform_declarations}\n\
                \n\
                {output_struct_declaration}\n\
                \n\
                [[stage(fragment)]]\n\
                fn fs_main({input_declaration}) -> {output_struct_name} {{\n\
                    {variable_definitions}\n\
                    {output_struct_assignments_and_return}\n\
                }}\
            ",
                texture_2d_definitions = texture_2d_definitions.join("\n\n"),
                uniform_declarations = uniform_declarations.join("\n\n"),
                output_struct_declaration = outputs.struct_declaration(),
                input_declaration = outputs_from_vertex.input_declaration(),
                output_struct_name = outputs.struct_name,
                variable_definitions = variable_definitions.join("\n"),
                output_struct_assignments_and_return = outputs.struct_assignments_and_return()
            ))
        } else {
            None
        }
    }
}

impl VertexBufferBinding {
    fn struct_definition(&self) -> String {
        let struct_fields: Vec<_> = self
            .struct_fields
            .iter()
            .map(VertexBufferBindingField::declaration)
            .collect();

        format!(
            "\
            struct {} {{\n\
                {}
            }};",
            self.struct_name,
            struct_fields.join("\n")
        )
    }

    fn argument_declaration(&self) -> String {
        format!("{}: {}", self.input_arg_ident, self.struct_name)
    }
}

impl VertexBufferBindingField {
    fn declaration(&self) -> String {
        format!(
            "[[location({})]] {}: {};",
            self.location, self.ident, self.ty
        )
    }
}

impl Texture2DBinding {
    fn declarations(&self) -> String {
        format!(
            "\
            [[group({group}), binding({texture_binding})]]\n\
            var {texture_ident}: texture_2d<f32>;\n\
            [[group({group}), binding({sampler_binding})]]\n\
            var {sampler_ident}: sampler;\
        ",
            group = self.group,
            texture_binding = self.texture_binding,
            texture_ident = self.texture_ident,
            sampler_binding = self.sampler_binding,
            sampler_ident = self.sampler_ident
        )
    }

    fn sampling_expr<S: Display>(&self, texture_coord_expr: S) -> String {
        format!(
            "textureSample({}, {}, {})",
            self.texture_ident, self.sampler_ident, texture_coord_expr
        )
    }

    fn sampling_assignment<S: Display>(
        &self,
        variable_ident: &'static str,
        texture_coord_expr: S,
    ) -> VariableDefinition {
        VariableDefinition {
            ident: variable_ident,
            expr: Cow::Owned(self.sampling_expr(texture_coord_expr)),
        }
    }
}

impl VariableDefinition {
    fn statement(&self) -> String {
        format!("let {} = {};", self.ident, &self.expr)
    }
}

impl Outputs {
    fn struct_declaration(&self) -> String {
        let builtin_output_field_declarations: Vec<_> = self
            .builtin_fields
            .iter()
            .map(BuiltinOutputField::field_declaration)
            .collect();

        let output_field_declarations: Vec<_> = self
            .fields
            .iter()
            .enumerate()
            .map(|(location, field)| field.field_declaration(location as u32))
            .collect();

        format!(
            "\
            struct {} {{\n\
                {}\n\
                {}\n\
            }};\
            ",
            self.struct_name,
            builtin_output_field_declarations.join("\n"),
            output_field_declarations.join("\n")
        )
    }

    fn struct_assignments_and_return(&self) -> String {
        let builtin_output_assignments: Vec<_> = self
            .builtin_fields
            .iter()
            .map(|field| field.assignment_to_field(self.struct_ident))
            .collect();

        let output_assignments: Vec<_> = self
            .fields
            .iter()
            .map(|field| field.assignment_to_field(self.struct_ident))
            .collect();

        format!(
            "\
            var {output_ident}: {struct_name};\n\n\
            {builtin_output_assignments}\n\
            {output_assignments}\n\n\
            return {output_ident};\n\
        ",
            output_ident = self.struct_ident,
            struct_name = self.struct_name,
            builtin_output_assignments = builtin_output_assignments.join("\n"),
            output_assignments = output_assignments.join("\n")
        )
    }

    fn input_declaration(&self) -> String {
        format!("{}: {}", self.struct_ident, self.struct_name)
    }

    fn local_variable_definitions_for_all_fields(&self) -> Vec<VariableDefinition> {
        let mut variable_definitions =
            Vec::with_capacity(self.builtin_fields.len() + self.fields.len());

        for field in &self.builtin_fields {
            variable_definitions.push(field.local_variable_definition(self.struct_ident));
        }

        for field in &self.fields {
            variable_definitions.push(field.local_variable_definition(self.struct_ident));
        }

        variable_definitions
    }

    fn add_field(&mut self, field: OutputField) {
        self.fields.push(field);
    }
}

impl BuiltinOutputField {
    fn field_declaration(&self) -> String {
        format!(
            "[[builtin({})]] {}: {};",
            self.builtin_flag, self.ident, self.ty
        )
    }

    fn assignment_to_field<S: Display>(&self, output_struct_ident: S) -> String {
        format!("{}.{} = {};", output_struct_ident, self.ident, &self.expr)
    }

    fn local_variable_definition<S: Display>(&self, output_struct_ident: S) -> VariableDefinition {
        VariableDefinition {
            ident: self.ident,
            expr: Cow::Owned(format!("{}.{}", output_struct_ident, self.ident)),
        }
    }
}

impl OutputField {
    fn field_declaration(&self, location: u32) -> String {
        format!("[[location({})]] {}: {};", location, self.ident, self.ty)
    }

    fn assignment_to_field<S: Display>(&self, output_struct_ident: S) -> String {
        format!("{}.{} = {};", output_struct_ident, self.ident, &self.expr)
    }

    fn local_variable_definition<S: Display>(&self, output_struct_ident: S) -> VariableDefinition {
        VariableDefinition {
            ident: self.ident,
            expr: Cow::Owned(format!("{}.{}", output_struct_ident, self.ident)),
        }
    }
}

impl CameraShaderInput {
    fn generate_code(&self) -> UniformBinding {
        UniformBinding {
            declaration: format!(
                "\
                struct ViewProjectionTransform {{\n\
                    matrix: mat4x4<f32>;\n\
                }};\n\
                \n\
                [[group(0), binding({})]]\n\
                var<uniform> view_projection_transform: ViewProjectionTransform;\
            ",
                self.view_proj_matrix_binding
            ),
            variables: vec!["view_projection_transform.matrix"],
        }
    }
}

impl ModelInstanceTransformShaderInput {
    fn generate_code(&self) -> (VertexBufferBinding, VariableDefinition) {
        let (loc_0, loc_1, loc_2, loc_3) = self.model_matrix_locations;
        (
            VertexBufferBinding {
                struct_name: "ModelTransform",
                struct_fields: vec![
                    VertexBufferBindingField {
                        location: loc_0,
                        ident: "matrix_0",
                        ty: "vec4<f32>",
                    },
                    VertexBufferBindingField {
                        location: loc_1,
                        ident: "matrix_1",
                        ty: "vec4<f32>",
                    },
                    VertexBufferBindingField {
                        location: loc_2,
                        ident: "matrix_2",
                        ty: "vec4<f32>",
                    },
                    VertexBufferBindingField {
                        location: loc_3,
                        ident: "matrix_3",
                        ty: "vec4<f32>",
                    },
                ],
                input_arg_ident: "model_transform",
            },
            VariableDefinition {
                ident: "model_matrix",
                expr: Cow::Borrowed(
                    "\
                mat4x4<f32>(\n\
                    model_transform.matrix_0,\n\
                    model_transform.matrix_1,\n\
                    model_transform.matrix_2,\n\
                    model_transform.matrix_3,\n\
                )",
                ),
            },
        )
    }
}

impl MeshShaderInput {
    fn generate_code(&self) -> (VertexBufferBinding, Vec<OutputField>) {
        let mut struct_fields = Vec::with_capacity(3);
        let mut output_struct_fields = Vec::with_capacity(2);

        struct_fields.push(VertexBufferBindingField {
            location: self.position_location,
            ident: "position",
            ty: "vec3<f32>",
        });

        if let Some(location) = self.normal_vector_location {
            let ident = "normal_vector";
            let ty = "vec3<f32>";
            struct_fields.push(VertexBufferBindingField {
                location,
                ident,
                ty,
            });
            output_struct_fields.push(OutputField {
                ident,
                ty,
                expr: Cow::Borrowed("vertex.normal_vector"),
            });
        };

        if let Some(location) = self.texture_coord_location {
            let ident = "texture_coords";
            let ty = "vec2<f32>";
            struct_fields.push(VertexBufferBindingField {
                location,
                ident,
                ty,
            });
            output_struct_fields.push(OutputField {
                ident,
                ty,
                expr: Cow::Borrowed("vertex.texture_coords"),
            });
        };

        (
            VertexBufferBinding {
                struct_name: "VertexAttributes",
                struct_fields,
                input_arg_ident: "vertex",
            },
            output_struct_fields,
        )
    }
}

impl BlinnPhongTextureShaderInput {
    fn generate_code(&self) -> Vec<Texture2DBinding> {
        let mut bindings = Vec::with_capacity(2);

        let (diffuse_texture_binding, diffuse_sampler_binding) =
            self.diffuse_texture_and_sampler_bindings;
        bindings.push(Texture2DBinding {
            group: 1,
            texture_binding: diffuse_texture_binding,
            sampler_binding: diffuse_sampler_binding,
            texture_ident: "diffuse_texture",
            sampler_ident: "diffuse_sampler",
        });

        if let Some((specular_texture_binding, specular_sampler_binding)) =
            self.specular_texture_and_sampler_bindings
        {
            bindings.push(Texture2DBinding {
                group: 1,
                texture_binding: specular_texture_binding,
                sampler_binding: specular_sampler_binding,
                texture_ident: "specular_texture",
                sampler_ident: "specular_sampler",
            });
        }

        bindings
    }
}

impl BlinnPhongFeatureShaderInput {
    fn generate_code(&self) -> VertexBufferBinding {
        let mut struct_fields = Vec::with_capacity(5);

        struct_fields.push(VertexBufferBindingField {
            location: self.ambient_color_location,
            ident: "ambient_color",
            ty: "vec3<f32>",
        });

        if let Some(location) = self.diffuse_color_location {
            struct_fields.push(VertexBufferBindingField {
                location,
                ident: "diffuse_color",
                ty: "vec3<f32>",
            });
        }

        if let Some(location) = self.specular_color_location {
            struct_fields.push(VertexBufferBindingField {
                location,
                ident: "specular_color",
                ty: "vec3<f32>",
            });
        }

        struct_fields.push(VertexBufferBindingField {
            location: self.shininess_location,
            ident: "shininess",
            ty: "f32",
        });

        struct_fields.push(VertexBufferBindingField {
            location: self.alpha_location,
            ident: "alpha",
            ty: "f32",
        });

        VertexBufferBinding {
            struct_name: "MaterialProperties",
            struct_fields,
            input_arg_ident: "material_properties",
        }
    }
}

impl FixedColorFeatureShaderInput {
    fn generate_code(&self) -> VertexBufferBinding {
        VertexBufferBinding {
            struct_name: "MaterialProperties",
            struct_fields: vec![VertexBufferBindingField {
                location: self.color_location,
                ident: "color",
                ty: "vec4<f32>",
            }],
            input_arg_ident: "material_properties",
        }
    }
}
