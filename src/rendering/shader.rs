//! Graphics shaders.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    vec,
};

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::{fs, path::Path};
    }
}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
}

#[derive(Clone, Debug)]
pub struct ShaderBuilder;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraShaderInput {
    pub view_proj_matrix_binding: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshShaderInput {
    pub position_location: u32,
    pub color_location: Option<u32>,
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
    FixedMaterial(FixedTextureShaderInput),
    BlinnPhongMaterial(BlinnPhongTextureShaderInput),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedTextureShaderInput {
    pub color_texture_and_sampler_bindings: (u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlinnPhongTextureShaderInput {
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformShaderInput {}

#[derive(Clone, Debug)]
struct VertexShaderBuilder {
    vertex_buffer_bindings: Vec<VertexBufferBinding>,
    uniform_bindings: Vec<UniformBinding>,
    variable_definitions: Vec<VariableDefinition>,
}

#[derive(Clone, Debug)]
struct FragmentShaderBuilder {
    texture_2d_bindings: Vec<Texture2DBinding>,
    uniform_bindings: Vec<UniformBinding>,
    variable_definitions: Vec<VariableDefinition>,
}

#[derive(Clone, Debug)]
struct VertexBufferBinding {
    struct_name: &'static str,
    input_arg_ident: &'static str,
    struct_fields: HashMap<&'static str, VertexBufferBindingField>,
}

#[derive(Clone, Debug)]
struct VertexBufferBindingField {
    location: u32,
    ident: &'static str,
    ty: &'static str,
}

#[derive(Clone, Debug)]
struct VertexPropertyRequirements {
    required_field_idents: HashSet<&'static str>,
}

#[derive(Clone, Debug)]
struct UniformBinding {
    declaration: String,
    variables: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct Texture2DBinding {
    ident: &'static str,
    group: u32,
    texture_binding: u32,
    sampler_binding: u32,
    texture_ident: String,
    sampler_ident: String,
}

#[derive(Clone, Debug)]
struct VariableDefinition {
    ident: &'static str,
    expr: Cow<'static, str>,
}

#[derive(Clone, Debug)]
struct Outputs {
    struct_name: &'static str,
    struct_ident: &'static str,
    builtin_fields: HashMap<&'static str, BuiltinOutputField>,
    fields: HashMap<&'static str, OutputField>,
}

#[derive(Clone, Debug)]
struct BuiltinOutputField {
    builtin_flag: &'static str,
    ident: &'static str,
    ty: &'static str,
    expr: Cow<'static, str>,
}

#[derive(Clone, Debug)]
struct OutputField {
    ident: &'static str,
    ty: &'static str,
    expr: Cow<'static, str>,
}

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
    ) -> Result<String> {
        // TODO: Materials must define requirements on the vertex
        // properties of the mesh, and these should be checked by
        // the `MeshShaderInput`.

        let camera_shader_input = camera_shader_input
            .ok_or_else(|| anyhow!("Tried to build shader with no camera input"))?;

        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let mut model_instance_transform_shader_input = None;
        let mut fixed_color_feature_shader_input = None;
        let mut blinn_phong_feature_shader_input = None;

        for &instance_feature_shader_input in instance_feature_shader_inputs {
            match instance_feature_shader_input {
                InstanceFeatureShaderInput::ModelInstanceTransform(shader_input) => {
                    let old = model_instance_transform_shader_input.replace(shader_input);
                    assert!(old.is_none());
                }
                InstanceFeatureShaderInput::FixedColorMaterial(shader_input) => {
                    let old = fixed_color_feature_shader_input.replace(shader_input);
                    assert!(old.is_none());
                }
                InstanceFeatureShaderInput::BlinnPhongMaterial(shader_input) => {
                    let old = blinn_phong_feature_shader_input.replace(shader_input);
                    assert!(old.is_none());
                }
                #[cfg(test)]
                InstanceFeatureShaderInput::None => {}
            }
        }

        let model_instance_transform_shader_input = model_instance_transform_shader_input
            .ok_or_else(|| {
                anyhow!("Tried to build shader with no model instance transform input")
            })?;

        let mut vertex_shader_builder = VertexShaderBuilder::new();
        let mut vertex_shader_outputs = Outputs::new_vertex_outputs();
        let mut fragment_shader_builder = FragmentShaderBuilder::new();
        let mut fragment_shader_outputs = Outputs::new_fragment_outputs();

        let camera_uniform_binding = camera_shader_input.generate_code();

        let (model_transform_binding, model_matrix_definition) =
            model_instance_transform_shader_input.generate_code();

        let (mut vertex_property_binding, position_expr) = mesh_shader_input.generate_code();

        match (
            fixed_color_feature_shader_input,
            blinn_phong_feature_shader_input,
            material_texture_shader_input,
        ) {
            (Some(feature_shader_input), None, Some(MaterialTextureShaderInput::None)) => {
                vertex_property_binding
                    .conform_to_requirements(VertexPropertyRequirements::with_position())?;

                let fixed_color_binding = feature_shader_input.generate_code();

                vertex_shader_outputs.add_fields(fixed_color_binding.all_fields_as_outputs());
                vertex_shader_builder.add_vertex_buffer_binding(fixed_color_binding);

                let color_expr = vertex_shader_outputs
                    .get_field_access_expression(FixedColorFeatureShaderInput::COLOR_IDENT)
                    .unwrap();
                fragment_shader_outputs.add_color(Cow::Owned(color_expr));
            }
            (None, None, Some(MaterialTextureShaderInput::FixedMaterial(texture_shader_input))) => {
                let requirements = VertexPropertyRequirements::with_position().and_texture_coords();

                vertex_property_binding.conform_to_requirements(requirements.clone())?;

                vertex_shader_outputs.add_fields(
                    vertex_property_binding
                        .required_fields_as_outputs(requirements.without_position()),
                );

                let texture_binding = texture_shader_input.generate_code();

                let texture_coord_expr = vertex_shader_outputs
                    .get_field_access_expression(MeshShaderInput::TEXTURE_COORD_IDENT)
                    .unwrap();
                let color_expr = texture_binding.sampling_expr(texture_coord_expr);
                fragment_shader_outputs.add_color(Cow::Owned(color_expr));

                fragment_shader_builder.add_texture_2d_binding(texture_binding);
            }
            (None, Some(feature_shader_input), Some(texture_shader_input)) => {
                let material_property_binding = feature_shader_input.generate_code();

                match texture_shader_input {
                    MaterialTextureShaderInput::None => {
                        let requirements =
                            VertexPropertyRequirements::with_position().and_normal_vector();

                        vertex_property_binding.conform_to_requirements(requirements.clone())?;

                        vertex_shader_outputs.add_fields(
                            vertex_property_binding
                                .required_fields_as_outputs(requirements.without_position()),
                        );
                    }
                    MaterialTextureShaderInput::BlinnPhongMaterial(texture_shader_input) => {
                        let requirements = VertexPropertyRequirements::with_position()
                            .and_normal_vector()
                            .and_texture_coords();

                        vertex_property_binding.conform_to_requirements(requirements.clone())?;

                        vertex_shader_outputs.add_fields(
                            vertex_property_binding
                                .required_fields_as_outputs(requirements.without_position()),
                        );

                        let texture_bindings = texture_shader_input.generate_code();

                        let texture_coord_expr = vertex_shader_outputs
                            .get_field_access_expression(MeshShaderInput::TEXTURE_COORD_IDENT)
                            .unwrap();

                        for binding in &texture_bindings {
                            fragment_shader_builder.add_variable_definition(
                                binding.sampling_assignment_to_same_name(&texture_coord_expr),
                            );
                        }

                        fragment_shader_builder.add_texture_2d_bindings(texture_bindings);
                    }
                    _ => {
                        return Err(anyhow!(
                            "Tried to use Blinn-Phong material with texture from another material"
                        ));
                    }
                }

                vertex_shader_outputs.add_fields(material_property_binding.all_fields_as_outputs());

                fragment_shader_builder.add_variable_definitions(
                    vertex_shader_outputs.local_variable_definitions_for_all_fields(),
                );

                let color_expr = format!(
                    "{ambient}",
                    ambient = BlinnPhongFeatureShaderInput::AMBIENT_COLOR_IDENT
                );

                fragment_shader_outputs.add_color(Cow::Owned(color_expr));
            }
            (None, None, None) => {
                let requirements = VertexPropertyRequirements::with_position().and_color();

                vertex_property_binding.conform_to_requirements(requirements.clone())?;

                vertex_shader_outputs.add_fields(
                    vertex_property_binding
                        .required_fields_as_outputs(requirements.without_position()),
                );

                let color_expr = vertex_shader_outputs
                    .get_field_access_expression(MeshShaderInput::COLOR_IDENT)
                    .unwrap();
                fragment_shader_outputs.add_color(Cow::Owned(color_expr));
            }
            _ => {
                return Err(anyhow!("Tried to build shader with invalid material"));
            }
        }

        let clip_position_expr = format!(
            "{view_projection_matrix} * {model_matrix} * {position}",
            view_projection_matrix = camera_uniform_binding.variables[0],
            model_matrix = model_matrix_definition.ident,
            position = position_expr
        );

        vertex_shader_outputs.add_clip_position(Cow::Owned(clip_position_expr));

        vertex_shader_builder.add_uniform_binding(camera_uniform_binding);
        vertex_shader_builder.add_vertex_buffer_binding(model_transform_binding);
        vertex_shader_builder.add_vertex_buffer_binding(vertex_property_binding);
        vertex_shader_builder.add_variable_definition(model_matrix_definition);

        let vertex_shader_source = vertex_shader_builder.build(&vertex_shader_outputs);
        let fragment_shader_source =
            fragment_shader_builder.build(&vertex_shader_outputs, &fragment_shader_outputs);

        Ok(format!(
            "{}\n\n{}",
            vertex_shader_source, fragment_shader_source
        ))
    }
}

impl VertexShaderBuilder {
    fn new() -> Self {
        Self {
            vertex_buffer_bindings: Vec::new(),
            uniform_bindings: Vec::new(),
            variable_definitions: Vec::new(),
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

    fn build(self, outputs: &Outputs) -> String {
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

        format!(
            "\
            {vertex_struct_definitions}\n\
            \n\
            {uniform_declarations}\n\
            \n\
            {output_struct_declaration}\n\
            \n\
            [[stage(vertex)]]\n\
            fn vs_main({input_declarations}) -> {output_struct_name} {{\n\
                {variable_definitions}\n\n\
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
        )
    }
}

impl FragmentShaderBuilder {
    fn new() -> Self {
        Self {
            texture_2d_bindings: Vec::new(),
            uniform_bindings: Vec::new(),
            variable_definitions: Vec::new(),
        }
    }

    fn add_texture_2d_binding(&mut self, binding: Texture2DBinding) {
        self.texture_2d_bindings.push(binding);
    }

    fn add_texture_2d_bindings(&mut self, bindings: impl IntoIterator<Item = Texture2DBinding>) {
        self.texture_2d_bindings.extend(bindings);
    }

    fn add_uniform_binding(&mut self, binding: UniformBinding) {
        self.uniform_bindings.push(binding);
    }

    fn add_variable_definition(&mut self, definition: VariableDefinition) {
        self.variable_definitions.push(definition);
    }

    fn add_variable_definitions(
        &mut self,
        definitions: impl IntoIterator<Item = VariableDefinition>,
    ) {
        self.variable_definitions.extend(definitions);
    }

    fn build(&self, inputs: &Outputs, outputs: &Outputs) -> String {
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

        format!(
            "\
                {texture_2d_definitions}\n\
                \n\
                {uniform_declarations}\n\
                \n\
                {output_struct_declaration}\n\
                \n\
                [[stage(fragment)]]\n\
                fn fs_main({input_declaration}) -> {output_struct_name} {{\n\
                    {variable_definitions}\n\n\
                    {output_struct_assignments_and_return}\n\
                }}\
            ",
            texture_2d_definitions = texture_2d_definitions.join("\n\n"),
            uniform_declarations = uniform_declarations.join("\n\n"),
            output_struct_declaration = outputs.struct_declaration(),
            input_declaration = inputs.input_declaration(),
            output_struct_name = outputs.struct_name,
            variable_definitions = variable_definitions.join("\n"),
            output_struct_assignments_and_return = outputs.struct_assignments_and_return()
        )
    }
}

impl VertexBufferBinding {
    fn new(struct_name: &'static str, input_arg_ident: &'static str) -> Self {
        Self {
            struct_name,
            input_arg_ident,
            struct_fields: HashMap::new(),
        }
    }

    fn conform_to_requirements(
        &mut self,
        mut requirements: VertexPropertyRequirements,
    ) -> Result<()> {
        self.struct_fields
            .retain(|&field, _| requirements.mark_as_met(field));

        if let Some(unmet_requirement) = requirements.remaining().next() {
            Err(anyhow!(
                "Vertex property requirement `{}` not met",
                unmet_requirement
            ))
        } else {
            Ok(())
        }
    }

    fn get_field_as_output(&self, field_ident: &'static str) -> Option<OutputField> {
        self.struct_fields
            .get(field_ident)
            .map(|field| field.to_output_field(self.input_arg_ident))
    }

    fn all_fields_as_outputs(&self) -> impl Iterator<Item = OutputField> + '_ {
        self.struct_fields
            .values()
            .map(|field| field.to_output_field(self.input_arg_ident))
    }

    fn required_fields_as_outputs(
        &self,
        requirements: VertexPropertyRequirements,
    ) -> impl Iterator<Item = OutputField> + '_ {
        self.all_fields_as_outputs()
            .filter(move |field| requirements.requires(field.ident))
    }

    fn struct_definition(&self) -> String {
        let mut struct_fields: Vec<_> = self.struct_fields.values().collect();
        struct_fields.sort_by_key(|field| field.location);
        let struct_fields: Vec<_> = struct_fields
            .into_iter()
            .map(VertexBufferBindingField::declaration)
            .collect();

        format!(
            "\
            struct {} {{\n\
                {}\n\
            }};",
            self.struct_name,
            struct_fields.join("\n")
        )
    }

    fn argument_declaration(&self) -> String {
        format!("{}: {}", self.input_arg_ident, self.struct_name)
    }

    fn add_field(&mut self, field: VertexBufferBindingField) {
        let old = self.struct_fields.insert(field.ident, field);
        assert!(old.is_none());
    }

    fn add_fields(&mut self, fields: impl IntoIterator<Item = VertexBufferBindingField>) {
        self.struct_fields
            .extend(fields.into_iter().map(|field| (field.ident, field)));
    }
}

impl VertexBufferBindingField {
    fn declaration(&self) -> String {
        format!(
            "    [[location({})]] {}: {};",
            self.location, self.ident, self.ty
        )
    }

    fn access_expression<S: Display>(&self, struct_ident: S) -> String {
        format!("{}.{}", struct_ident, self.ident)
    }

    fn to_output_field<S: Display>(&self, struct_ident: S) -> OutputField {
        OutputField {
            ident: self.ident,
            ty: self.ty,
            expr: Cow::Owned(self.access_expression(struct_ident)),
        }
    }
}

impl VertexPropertyRequirements {
    fn with(ident: &'static str) -> Self {
        Self {
            required_field_idents: [ident].into_iter().collect(),
        }
    }

    fn and(mut self, ident: &'static str) -> Self {
        self.required_field_idents.insert(ident);
        self
    }

    fn without(mut self, ident: &'static str) -> Self {
        self.required_field_idents.remove(ident);
        self
    }

    fn requires(&self, ident: &'static str) -> bool {
        self.required_field_idents.contains(ident)
    }

    fn remaining(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.required_field_idents.iter().copied()
    }

    fn mark_as_met(&mut self, ident: &'static str) -> bool {
        self.required_field_idents.remove(ident)
    }

    fn with_position() -> Self {
        Self::with(MeshShaderInput::POSITION_IDENT)
    }

    fn without_position(self) -> Self {
        self.without(MeshShaderInput::POSITION_IDENT)
    }

    fn and_color(self) -> Self {
        self.and(MeshShaderInput::COLOR_IDENT)
    }

    fn and_normal_vector(self) -> Self {
        self.and(MeshShaderInput::NORMAL_VECTOR_IDENT)
    }

    fn and_texture_coords(self) -> Self {
        self.and(MeshShaderInput::TEXTURE_COORD_IDENT)
    }
}

impl Texture2DBinding {
    fn new(ident: &'static str, group: u32, texture_binding: u32, sampler_binding: u32) -> Self {
        let texture_ident = format!("{}_texture", ident);
        let sampler_ident = format!("{}_sampler", ident);
        Self {
            ident,
            group,
            texture_binding,
            sampler_binding,
            texture_ident,
            sampler_ident,
        }
    }

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

    fn sampling_assignment_to_same_name<S: Display>(
        &self,
        texture_coord_expr: S,
    ) -> VariableDefinition {
        self.sampling_assignment(self.ident, texture_coord_expr)
    }
}

impl VariableDefinition {
    fn statement(&self) -> String {
        format!("    let {} = {};", self.ident, &self.expr)
    }
}

impl Outputs {
    const CLIP_POSITION_IDENT: &'static str = "clip_position";
    const COLOR_IDENT: &'static str = "color";

    fn new_vertex_outputs() -> Self {
        Self {
            struct_name: "VertexOutputs",
            struct_ident: "vertex_outputs",
            builtin_fields: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    fn new_fragment_outputs() -> Self {
        Self {
            struct_name: "FragmentOutputs",
            struct_ident: "fragment_outputs",
            builtin_fields: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    fn get_field_access_expression(&self, field_ident: &'static str) -> Option<String> {
        self.fields
            .get(field_ident)
            .map(|field| field.access_expression(self.struct_ident))
            .or_else(|| {
                self.builtin_fields
                    .get(field_ident)
                    .map(|field| field.access_expression(self.struct_ident))
            })
    }

    fn struct_declaration(&self) -> String {
        let mut builtin_output_field_declarations: Vec<_> = self.builtin_fields.values().collect();
        builtin_output_field_declarations.sort_by_key(|field| field.ident);
        let builtin_output_field_declarations: Vec<_> = builtin_output_field_declarations
            .into_iter()
            .map(BuiltinOutputField::declaration)
            .collect();

        let mut output_field_declarations: Vec<_> = self.fields.values().collect();
        output_field_declarations.sort_by_key(|field| field.ident);
        let output_field_declarations: Vec<_> = output_field_declarations
            .into_iter()
            .enumerate()
            .map(|(location, field)| field.declaration(location as u32))
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
            .values()
            .map(|field| field.assignment_to_field(self.struct_ident))
            .collect();

        let output_assignments: Vec<_> = self
            .fields
            .values()
            .map(|field| field.assignment_to_field(self.struct_ident))
            .collect();

        format!(
            "    \
            var {output_ident}: {struct_name};\n\n\
            {builtin_output_assignments}\n\
            {output_assignments}\n\n    \
            return {output_ident};\
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

    fn local_variable_definitions_for_all_fields(
        &self,
    ) -> impl Iterator<Item = VariableDefinition> + '_ {
        self.builtin_fields
            .values()
            .map(|field| field.local_variable_definition(self.struct_ident))
            .chain(
                self.fields
                    .values()
                    .map(|field| field.local_variable_definition(self.struct_ident)),
            )
    }

    fn add_clip_position(&mut self, clip_position_expr: Cow<'static, str>) {
        self.add_builtin_field(BuiltinOutputField {
            builtin_flag: "position",
            ident: Self::CLIP_POSITION_IDENT,
            ty: "vec4<f32>",
            expr: clip_position_expr,
        });
    }

    fn add_color(&mut self, color_expr: Cow<'static, str>) {
        self.add_field(OutputField {
            ident: Self::COLOR_IDENT,
            ty: "vec4<f32>",
            expr: color_expr,
        });
    }

    fn add_builtin_field(&mut self, field: BuiltinOutputField) {
        let old = self.builtin_fields.insert(field.ident, field);
        assert!(old.is_none());
    }

    fn add_field(&mut self, field: OutputField) {
        let old = self.fields.insert(field.ident, field);
        assert!(old.is_none());
    }

    fn add_fields(&mut self, fields: impl IntoIterator<Item = OutputField>) {
        self.fields
            .extend(fields.into_iter().map(|field| (field.ident, field)));
    }
}

impl BuiltinOutputField {
    fn declaration(&self) -> String {
        format!(
            "    [[builtin({})]] {}: {};",
            self.builtin_flag, self.ident, self.ty
        )
    }

    fn assignment_to_field<S: Display>(&self, output_struct_ident: S) -> String {
        format!(
            "    {}.{} = {};",
            output_struct_ident, self.ident, &self.expr
        )
    }

    fn access_expression<S: Display>(&self, output_struct_ident: S) -> String {
        format!("{}.{}", output_struct_ident, self.ident)
    }

    fn local_variable_definition<S: Display>(&self, output_struct_ident: S) -> VariableDefinition {
        VariableDefinition {
            ident: self.ident,
            expr: Cow::Owned(self.access_expression(output_struct_ident)),
        }
    }
}

impl OutputField {
    fn declaration(&self, location: u32) -> String {
        format!(
            "    [[location({})]] {}: {};",
            location, self.ident, self.ty
        )
    }

    fn assignment_to_field<S: Display>(&self, output_struct_ident: S) -> String {
        format!(
            "    {}.{} = {};",
            output_struct_ident, self.ident, &self.expr
        )
    }

    fn access_expression<S: Display>(&self, output_struct_ident: S) -> String {
        format!("{}.{}", output_struct_ident, self.ident)
    }

    fn local_variable_definition<S: Display>(&self, output_struct_ident: S) -> VariableDefinition {
        VariableDefinition {
            ident: self.ident,
            expr: Cow::Owned(self.access_expression(output_struct_ident)),
        }
    }
}

impl CameraShaderInput {
    fn generate_code(&self) -> UniformBinding {
        UniformBinding {
            declaration: format!(
                "\
                struct ViewProjectionTransform {{\n    \
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

        let mut transform_binding = VertexBufferBinding::new("ModelTransform", "model_transform");

        transform_binding.add_field(VertexBufferBindingField {
            location: loc_0,
            ident: "matrix_0",
            ty: "vec4<f32>",
        });
        transform_binding.add_field(VertexBufferBindingField {
            location: loc_1,
            ident: "matrix_1",
            ty: "vec4<f32>",
        });
        transform_binding.add_field(VertexBufferBindingField {
            location: loc_2,
            ident: "matrix_2",
            ty: "vec4<f32>",
        });
        transform_binding.add_field(VertexBufferBindingField {
            location: loc_3,
            ident: "matrix_3",
            ty: "vec4<f32>",
        });

        let matrix_definition = VariableDefinition {
            ident: "model_matrix",
            expr: Cow::Borrowed(
                "\
            mat4x4<f32>(\n        \
                model_transform.matrix_0,\n        \
                model_transform.matrix_1,\n        \
                model_transform.matrix_2,\n        \
                model_transform.matrix_3,\n    \
            )",
            ),
        };

        (transform_binding, matrix_definition)
    }
}

impl MeshShaderInput {
    const POSITION_IDENT: &str = "position";
    const COLOR_IDENT: &str = "color";
    const NORMAL_VECTOR_IDENT: &str = "normal_vector";
    const TEXTURE_COORD_IDENT: &str = "texture_coords";

    fn generate_code(&self) -> (VertexBufferBinding, &'static str) {
        let mut property_binding = VertexBufferBinding::new("VertexAttributes", "vertex");

        property_binding.add_field(VertexBufferBindingField {
            location: self.position_location,
            ident: Self::POSITION_IDENT,
            ty: "vec3<f32>",
        });

        if let Some(location) = self.color_location {
            let ident = Self::COLOR_IDENT;
            let ty = "vec4<f32>";
            property_binding.add_field(VertexBufferBindingField {
                location,
                ident,
                ty,
            });
        };

        if let Some(location) = self.normal_vector_location {
            let ident = Self::NORMAL_VECTOR_IDENT;
            let ty = "vec3<f32>";
            property_binding.add_field(VertexBufferBindingField {
                location,
                ident,
                ty,
            });
        };

        if let Some(location) = self.texture_coord_location {
            let ident = Self::TEXTURE_COORD_IDENT;
            let ty = "vec2<f32>";
            property_binding.add_field(VertexBufferBindingField {
                location,
                ident,
                ty,
            });
        };

        let position_expr = "vec4<f32>(vertex.position, 1.0)";

        (property_binding, position_expr)
    }
}

impl FixedColorFeatureShaderInput {
    const COLOR_IDENT: &str = "color";

    fn generate_code(&self) -> VertexBufferBinding {
        let mut color_binding =
            VertexBufferBinding::new("MaterialProperties", "material_properties");
        color_binding.add_field(VertexBufferBindingField {
            location: self.color_location,
            ident: Self::COLOR_IDENT,
            ty: "vec4<f32>",
        });
        color_binding
    }
}

impl FixedTextureShaderInput {
    fn generate_code(&self) -> Texture2DBinding {
        let (color_texture_binding, color_sampler_binding) =
            self.color_texture_and_sampler_bindings;

        Texture2DBinding::new(
            FixedColorFeatureShaderInput::COLOR_IDENT,
            1,
            color_texture_binding,
            color_sampler_binding,
        )
    }
}

impl BlinnPhongFeatureShaderInput {
    const AMBIENT_COLOR_IDENT: &str = "ambient_color";
    const DIFFUSE_COLOR_IDENT: &str = "diffuse_color";
    const SPECULAR_COLOR_IDENT: &str = "specular_color";
    const SHININESS_IDENT: &str = "shininess";
    const ALPHA_IDENT: &str = "alpha";

    fn generate_code(&self) -> VertexBufferBinding {
        let mut property_binding =
            VertexBufferBinding::new("MaterialProperties", "material_properties");

        property_binding.add_field(VertexBufferBindingField {
            location: self.ambient_color_location,
            ident: Self::AMBIENT_COLOR_IDENT,
            ty: "vec3<f32>",
        });

        if let Some(location) = self.diffuse_color_location {
            property_binding.add_field(VertexBufferBindingField {
                location,
                ident: Self::DIFFUSE_COLOR_IDENT,
                ty: "vec3<f32>",
            });
        }

        if let Some(location) = self.specular_color_location {
            property_binding.add_field(VertexBufferBindingField {
                location,
                ident: Self::SPECULAR_COLOR_IDENT,
                ty: "vec3<f32>",
            });
        }

        property_binding.add_field(VertexBufferBindingField {
            location: self.shininess_location,
            ident: Self::SHININESS_IDENT,
            ty: "f32",
        });

        property_binding.add_field(VertexBufferBindingField {
            location: self.alpha_location,
            ident: Self::ALPHA_IDENT,
            ty: "f32",
        });

        property_binding
    }
}

impl BlinnPhongTextureShaderInput {
    fn generate_code(&self) -> Vec<Texture2DBinding> {
        let mut bindings = Vec::with_capacity(2);

        let (diffuse_texture_binding, diffuse_sampler_binding) =
            self.diffuse_texture_and_sampler_bindings;
        bindings.push(Texture2DBinding::new(
            BlinnPhongFeatureShaderInput::DIFFUSE_COLOR_IDENT,
            1,
            diffuse_texture_binding,
            diffuse_sampler_binding,
        ));

        if let Some((specular_texture_binding, specular_sampler_binding)) =
            self.specular_texture_and_sampler_bindings
        {
            bindings.push(Texture2DBinding::new(
                BlinnPhongFeatureShaderInput::SPECULAR_COLOR_IDENT,
                1,
                specular_texture_binding,
                specular_sampler_binding,
            ));
        }

        bindings
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const CAMERA_INPUT: CameraShaderInput = CameraShaderInput {
        view_proj_matrix_binding: 0,
    };
    const MODEL_TRANSFORM_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::ModelInstanceTransform(ModelInstanceTransformShaderInput {
            model_matrix_locations: (0, 1, 2, 3),
        });
    const MINIMAL_MESH_INPUT: MeshShaderInput = MeshShaderInput {
        position_location: 0,
        color_location: None,
        normal_vector_location: None,
        texture_coord_location: None,
    };
    const FIXED_COLOR_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
            color_location: 4,
        });
    const FIXED_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::FixedMaterial(FixedTextureShaderInput {
            color_texture_and_sampler_bindings: (0, 1),
        });

    #[test]
    #[should_panic]
    fn building_shader_with_no_inputs_fails() {
        ShaderBuilder::build_shader_source(None, None, &[], None).unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_input_fails() {
        ShaderBuilder::build_shader_source(Some(&CAMERA_INPUT), None, &[], None).unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_and_mesh_input_fails() {
        ShaderBuilder::build_shader_source(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            &[],
            None,
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_without_material_and_no_color_in_mesh_fails() {
        ShaderBuilder::build_shader_source(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            &[&MODEL_TRANSFORM_INPUT],
            None,
        )
        .unwrap();
    }

    #[test]
    fn test() {
        println!(
            "{}",
            ShaderBuilder::build_shader_source(
                Some(&CAMERA_INPUT),
                Some(&MeshShaderInput {
                    position_location: 0,
                    color_location: None,
                    normal_vector_location: None,
                    texture_coord_location: Some(1),
                }),
                &[&MODEL_TRANSFORM_INPUT],
                Some(&FIXED_TEXTURE_INPUT),
            )
            .unwrap()
        );
    }
}
