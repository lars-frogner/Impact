//! Generation and management of shaders.

mod rendering;
pub mod template;

pub use rendering::{
    AmbientLightShaderInput, BlinnPhongTextureShaderInput, BumpMappingTextureShaderInput,
    CameraShaderInput, DiffuseMicrofacetShadingModel, FixedColorFeatureShaderInput,
    FixedTextureShaderInput, InstanceFeatureShaderInput, LightMaterialFeatureShaderInput,
    LightShaderInput, MaterialShaderInput, MeshShaderInput, MicrofacetShadingModel,
    MicrofacetTextureShaderInput, ModelViewTransformShaderInput, NormalMappingShaderInput,
    OmnidirectionalLightShaderInput, ParallaxMappingShaderInput, PrepassTextureShaderInput,
    SingleModelViewTransformShaderInput, SkyboxShaderInput, SpecularMicrofacetShadingModel,
    UnidirectionalLightShaderInput,
};
use template::SpecificShaderTemplate;

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantGroupStage, PushConstantVariant},
        texture::attachment::RenderAttachmentQuantitySet,
        GraphicsDevice,
    },
    mesh::VertexAttributeSet,
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use naga::{
    AddressSpace, Arena, Binding, Block, BuiltIn, Bytes, Constant, Expression, Function,
    FunctionArgument, FunctionResult, GlobalVariable, Handle, ImageClass, ImageDimension,
    ImageQuery, Interpolation, Literal, LocalVariable, Module, Override, ResourceBinding,
    SampleLevel, Sampling, Scalar, ScalarKind, ShaderStage, Span, Statement, StructMember,
    SwitchCase, SwizzleComponent, Type, TypeInner, UniqueArena, VectorSize,
};
use rendering::RenderShaderGenerator;
use std::{
    borrow::Cow,
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    fs,
    hash::{Hash, Hasher},
    mem,
    path::Path,
};

/// Identifier for specific shaders.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ShaderID(u64);

#[derive(Debug)]
pub struct ShaderManager {
    /// Rendering shader programs.
    pub rendering_shaders: HashMap<ShaderID, Shader>,
    /// Compute shader programs.
    pub compute_shaders: HashMap<ShaderID, Shader>,
}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
    entry_point_names: EntryPointNames,
    source_code: Option<String>,
}

/// Names of the different shader entry point functions.
#[derive(Clone, Debug)]
pub struct EntryPointNames {
    /// Name of the vertex entry point function, or [`None`] if there is no
    /// vertex entry point.
    pub vertex: Option<Cow<'static, str>>,
    /// Name of the fragment entry point function, or [`None`] if there is no
    /// fragment entry point.
    pub fragment: Option<Cow<'static, str>>,
    /// Name of the compute entry point function, or [`None`] if there is no
    /// compute entry point.
    pub compute: Option<Cow<'static, str>>,
}

/// Represents a struct passed as an argument to a shader entry point. Holds the
/// handles for the expressions accessing each field of the struct.
#[derive(Clone, Debug)]
struct InputStruct {
    input_field_expressions: Vec<Handle<Expression>>,
}

/// Helper for constructing a struct [`Type`] for an argument to a shader entry
/// point and generating the code for accessing its fields.
#[derive(Clone, Debug)]
struct InputStructBuilder {
    builder: StructBuilder,
    input_arg_name: String,
}

/// Helper for constructing a struct [`Type`] for a shader entry point return
/// value and generating the code for assigning to its fields and returning its
/// value.
#[derive(Clone, Debug)]
struct OutputStructBuilder {
    builder: StructBuilder,
    input_expressions: Vec<Handle<Expression>>,
    location: u32,
}

/// Helper for constructing a struct [`Type`].
#[derive(Clone, Debug)]
struct StructBuilder {
    type_name: String,
    fields: Vec<StructMember>,
    offset: u32,
}

/// Helper for declaring global variables for a texture with an associated
/// sampler and generating a sampling expression.
#[derive(Clone, Debug)]
struct SampledTexture {
    texture_var: Handle<GlobalVariable>,
    sampler_var: Option<Handle<GlobalVariable>>,
    comparison_sampler_var: Option<Handle<GlobalVariable>>,
}

#[derive(Clone, Debug)]
struct PushConstantExpressions {
    push_constants: PushConstantGroup,
    stage: PushConstantGroupStage,
    expressions: Vec<Handle<Expression>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TextureType {
    Image2D,
    Image2DArray,
    ImageCubemap,
    DepthCubemap,
    DepthArray,
}

/// Helper for importing functions from one module to another.
///
/// This is an adaptation of the `DerivedModule` type in v0.5.0 of the
/// `naga_oil` library by robtfm: <https://github.com/robtfm/naga_oil>.
#[derive(Debug)]
struct ModuleImporter<'a, 'b> {
    imported_from_module: &'a Module,
    exported_to_module: &'b mut Module,
    type_map: HashMap<Handle<Type>, Handle<Type>>,
    const_map: HashMap<Handle<Constant>, Handle<Constant>>,
    global_map: HashMap<Handle<GlobalVariable>, Handle<GlobalVariable>>,
    const_expression_map: HashMap<Handle<Expression>, Handle<Expression>>,
    override_map: HashMap<Handle<Override>, Handle<Override>>,
    function_map: HashMap<String, Handle<Function>>,
}

/// A set of shader functions and types defined in source code that can be
/// imported into an existing [`Module`].
#[derive(Clone, Debug)]
struct SourceCode {
    module: Module,
    available_functions: HashMap<String, Handle<Function>>,
    available_named_types: HashMap<String, Handle<Type>>,
    used_functions: HashMap<String, Handle<Function>>,
    used_types: HashMap<String, Handle<Type>>,
}

const U32_WIDTH: u32 = mem::size_of::<u32>() as u32;

const U32_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Scalar(Scalar {
        kind: ScalarKind::Uint,
        width: U32_WIDTH as Bytes,
    }),
};

const F32_WIDTH: u32 = mem::size_of::<f32>() as u32;

const F32_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Scalar(Scalar {
        kind: ScalarKind::Float,
        width: F32_WIDTH as Bytes,
    }),
};

const VECTOR_2_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Bi,
        scalar: Scalar {
            kind: ScalarKind::Float,
            width: F32_WIDTH as Bytes,
        },
    },
};
const VECTOR_2_SIZE: u32 = 2 * F32_WIDTH;

const VECTOR_3_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Tri,
        scalar: Scalar {
            kind: ScalarKind::Float,
            width: F32_WIDTH as Bytes,
        },
    },
};
const VECTOR_3_SIZE: u32 = 3 * F32_WIDTH;

const VECTOR_4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Quad,
        scalar: Scalar {
            kind: ScalarKind::Float,
            width: F32_WIDTH as Bytes,
        },
    },
};
const VECTOR_4_SIZE: u32 = 4 * F32_WIDTH;

const MATRIX_4X4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Matrix {
        columns: VectorSize::Quad,
        rows: VectorSize::Quad,
        scalar: Scalar {
            kind: ScalarKind::Float,
            width: F32_WIDTH as Bytes,
        },
    },
};
const MATRIX_4X4_SIZE: u32 = 4 * 4 * F32_WIDTH;

const IMAGE_2D_TEXTURE_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::D2,
        arrayed: false,
        class: ImageClass::Sampled {
            kind: ScalarKind::Float,
            multi: false,
        },
    },
};

const IMAGE_2D_ARRAY_TEXTURE_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::D2,
        arrayed: true,
        class: ImageClass::Sampled {
            kind: ScalarKind::Float,
            multi: false,
        },
    },
};

const IMAGE_CUBEMAP_TEXTURE_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::Cube,
        arrayed: false,
        class: ImageClass::Sampled {
            kind: ScalarKind::Float,
            multi: false,
        },
    },
};

const SAMPLER_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Sampler { comparison: false },
};

const COMPARISON_SAMPLER_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Sampler { comparison: true },
};

const DEPTH_CUBEMAP_TEXTURE_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::Cube,
        arrayed: false,
        class: ImageClass::Depth { multi: false },
    },
};

const DEPTH_TEXTURE_ARRAY_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::D2,
        arrayed: true,
        class: ImageClass::Depth { multi: false },
    },
};

impl ShaderManager {
    /// Creates a new empty shader library.
    pub fn new() -> Self {
        Self {
            rendering_shaders: HashMap::new(),
            compute_shaders: HashMap::new(),
        }
    }

    /// Determines the shader ID for the given shader template and replacements,
    /// resolves the template and stores it as a rendering shader if it does not
    /// already exist and returns the shader ID.
    ///
    /// # Errors
    /// Returns an error if the shader template can not be resolved or compiled.
    pub fn get_or_create_rendering_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: SpecificShaderTemplate,
        replacements: &[(&str, String)],
    ) -> Result<ShaderID> {
        Self::get_or_create_shader_from_template(
            &mut self.rendering_shaders,
            graphics_device,
            template,
            replacements,
        )
    }

    /// Determines the shader ID for the given shader template and replacements,
    /// resolves the template and stores it as a compute shader if it does not
    /// already exist and returns the shader ID.
    ///
    /// # Errors
    /// Returns an error if the shader template can not be resolved or compiled.
    pub fn get_or_create_compute_shader_from_template(
        &mut self,
        graphics_device: &GraphicsDevice,
        template: SpecificShaderTemplate,
        replacements: &[(&str, String)],
    ) -> Result<ShaderID> {
        Self::get_or_create_shader_from_template(
            &mut self.compute_shaders,
            graphics_device,
            template,
            replacements,
        )
    }

    /// Obtains the appropriate rendering [`Shader`] for the given set of shader
    /// inputs.
    ///
    /// If a shader for the given inputs already exists, it is returned,
    /// otherwise a new shader is generated, compiled and cached.
    ///
    /// # Errors
    /// See [`ShaderGenerator::generate_rendering_shader_module`].
    pub fn obtain_rendering_shader(
        &mut self,
        graphics_device: &GraphicsDevice,
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constants: PushConstantGroup,
    ) -> Result<&Shader> {
        let shader_id = ShaderID::from_rendering_input(
            camera_shader_input,
            mesh_shader_input,
            light_shader_input,
            instance_feature_shader_inputs,
            material_shader_input,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            output_render_attachment_quantities,
            &push_constants,
        );

        match self.rendering_shaders.entry(shader_id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let module = RenderShaderGenerator::generate_shader_module(
                    camera_shader_input,
                    mesh_shader_input,
                    light_shader_input,
                    instance_feature_shader_inputs,
                    material_shader_input,
                    vertex_attribute_requirements,
                    input_render_attachment_quantities,
                    output_render_attachment_quantities,
                    push_constants,
                )?;
                Ok(entry.insert(Shader::from_naga_module(
                    graphics_device,
                    module,
                    format!("Generated rendering shader (hash {})", shader_id.0).as_str(),
                    None,
                )))
            }
        }
    }

    fn get_or_create_shader_from_template(
        shaders: &mut HashMap<ShaderID, Shader>,
        graphics_device: &GraphicsDevice,
        template: SpecificShaderTemplate,
        replacements: &[(&str, String)],
    ) -> Result<ShaderID> {
        let template_name = template.to_string();

        let shader_id =
            template::create_shader_id_for_template(&template_name, replacements.iter().cloned());

        match shaders.entry(shader_id) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                entry.insert(template.template().resolve_and_compile_as_wgsl(
                    graphics_device,
                    replacements.iter().cloned(),
                    &template_name,
                )?);
            }
        };

        Ok(shader_id)
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderID {
    /// Generates a [`ShaderID`] from the given string identifying the shader.
    pub fn from_identifier(identifier: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        identifier.hash(&mut hasher);
        Self(hasher.finish())
    }

    fn from_rendering_input(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constants: &PushConstantGroup,
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        "rendering".hash(&mut hasher);
        camera_shader_input.hash(&mut hasher);
        mesh_shader_input.hash(&mut hasher);
        light_shader_input.hash(&mut hasher);
        instance_feature_shader_inputs.hash(&mut hasher);
        material_shader_input.hash(&mut hasher);
        vertex_attribute_requirements.hash(&mut hasher);
        input_render_attachment_quantities.hash(&mut hasher);
        output_render_attachment_quantities.hash(&mut hasher);
        push_constants.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    pub fn from_wgsl_path(
        graphics_device: &GraphicsDevice,
        shader_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let shader_path = shader_path.as_ref();
        let label = shader_path.to_string_lossy();
        let source = fs::read_to_string(shader_path)?;
        Self::from_wgsl_source(graphics_device, source, label.as_ref())
    }

    /// Creates a new shader from the given source code.
    pub fn from_wgsl_source(
        graphics_device: &GraphicsDevice,
        source: String,
        label: &str,
    ) -> Result<Self> {
        let naga_module = naga::front::wgsl::parse_str(&source)?;
        Ok(Self::from_naga_module(
            graphics_device,
            naga_module,
            label,
            Some(source),
        ))
    }

    /// Creates a new shader from the given [`Module`].
    #[allow(unused_mut)]
    pub fn from_naga_module(
        graphics_device: &GraphicsDevice,
        naga_module: Module,
        label: &str,
        mut source_code: Option<String>,
    ) -> Self {
        #[cfg(debug_assertions)]
        if source_code.is_none() {
            source_code = Some(Self::generate_wgsl_from_naga_module(&naga_module));
        }

        let entry_point_names = EntryPointNames::from_naga_module(&naga_module);

        let module = graphics_device
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Naga(Cow::Owned(naga_module)),
                label: Some(label),
            });

        Self {
            module,
            entry_point_names,
            source_code,
        }
    }

    /// Returns a reference to the compiled shader module containing the vertex
    /// stage entry point if it exists.
    pub fn vertex_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns a reference to the compiled shader module containing the
    /// fragment stage entry point if it exists.
    pub fn fragment_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns a reference to the compiled shader module containing the compute
    /// stage entry point if it exists.
    pub fn compute_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns the name of the vertex entry point function, or [`None`] if
    /// there is no vertex entry point.
    pub fn vertex_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.vertex.as_deref()
    }

    /// Returns the name of the fragment entry point function, or [`None`] if
    /// there is no fragment entry point.
    pub fn fragment_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.fragment.as_deref()
    }

    /// Returns the name of the compute entry point function, or [`None`] if
    /// there is no compute entry point.
    pub fn compute_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names.compute.as_deref()
    }

    #[cfg(debug_assertions)]
    fn generate_wgsl_from_naga_module(module: &Module) -> String {
        Self::generate_wgsl_from_validated_naga_module(module, &Self::validate_naga_module(module))
    }

    #[cfg(debug_assertions)]
    fn validate_naga_module(module: &Module) -> naga::valid::ModuleInfo {
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(module)
            .expect("Shader validation failed")
    }

    #[cfg(debug_assertions)]
    fn generate_wgsl_from_validated_naga_module(
        module: &Module,
        module_info: &naga::valid::ModuleInfo,
    ) -> String {
        naga::back::wgsl::write_string(module, module_info, naga::back::wgsl::WriterFlags::all())
            .unwrap()
    }
}

impl std::fmt::Display for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self.source_code {
                Some(source) => source,
                None => "<Source code unavailable>",
            }
        )
    }
}

impl EntryPointNames {
    fn from_naga_module(module: &Module) -> Self {
        let mut entry_point_names = Self {
            vertex: None,
            fragment: None,
            compute: None,
        };
        for entry_point in &module.entry_points {
            match entry_point.stage {
                ShaderStage::Vertex => {
                    entry_point_names.vertex = Some(Cow::Owned(entry_point.name.clone()));
                }
                ShaderStage::Fragment => {
                    entry_point_names.fragment = Some(Cow::Owned(entry_point.name.clone()));
                }
                ShaderStage::Compute => {
                    entry_point_names.compute = Some(Cow::Owned(entry_point.name.clone()));
                }
            }
        }
        entry_point_names
    }
}

impl InputStruct {
    /// Returns the handle to the expression for the struct field with the given
    /// index.
    ///
    /// # Panics
    /// If the index is out of bounds.
    pub fn get_field_expr(&self, idx: usize) -> Handle<Expression> {
        self.input_field_expressions[idx]
    }
}

impl InputStructBuilder {
    /// Creates a builder for an input struct with the given type name and name
    /// to use when including the struct as an input argument.
    pub fn new<S: ToString, T: ToString>(type_name: S, input_arg_name: T) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_arg_name: input_arg_name.to_string(),
        }
    }

    pub fn n_fields(&self) -> usize {
        self.builder.n_fields()
    }

    /// Adds a new struct field.
    ///
    /// This method is intended for constructing an input struct to the vertex
    /// entry point. Thus, the field requires a location binding.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_field<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        location: u32,
        size: u32,
    ) -> usize {
        self.builder.add_field(
            name,
            type_handle,
            Some(Binding::Location {
                location,
                second_blend_source: false,
                interpolation: None,
                sampling: None,
            }),
            size,
        )
    }

    /// Generates code declaring the struct type and adds the struct as an input
    /// argument to the given [`Function`].
    ///
    /// # Returns
    /// An [`InputStruct`] holding the expression for accessing each field in
    /// the body of the function.
    pub fn generate_input_code(
        self,
        types: &mut UniqueArena<Type>,
        function: &mut Function,
    ) -> InputStruct {
        let n_fields = self.n_fields();

        let input_type = insert_in_arena(types, self.builder.into_type());

        let input_arg_ptr_expr =
            generate_input_argument(function, Some(self.input_arg_name), input_type, None);

        let input_field_expressions = emit_in_func(function, |function| {
            (0..n_fields)
                .map(|idx| {
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: input_arg_ptr_expr,
                            index: idx as u32,
                        },
                    )
                })
                .collect()
        });

        InputStruct {
            input_field_expressions,
        }
    }
}

impl OutputStructBuilder {
    /// Creates a builder for an output struct with the given type name.
    pub fn new<S: ToString>(type_name: S) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_expressions: Vec::new(),
            location: 0,
        }
    }

    /// Returns the expression for the field with the given index, or [`None`]
    /// if no field exists for the index.
    pub fn get_field_expr(&self, field_idx: usize) -> Option<Handle<Expression>> {
        self.input_expressions.get(field_idx).copied()
    }

    /// Adds a new struct field.
    ///
    /// The field is given an automatically incremented location binding.
    ///
    /// The given input expression handle specifies the expression whose value
    /// should be assigned to the field when [`generate_output_code`] is called.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_field<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        interpolation: Option<Interpolation>,
        sampling: Option<Sampling>,
        size: u32,
        input_expr: Handle<Expression>,
    ) -> usize {
        self.input_expressions.push(input_expr);

        let idx = self.builder.add_field(
            name,
            type_handle,
            Some(Binding::Location {
                location: self.location,
                second_blend_source: false,
                interpolation,
                sampling,
            }),
            size,
        );

        self.location += 1;

        idx
    }

    /// Adds a new struct field that will use perspective-correct interpolation
    /// and center sampling when passed to the fragment entry point.
    ///
    /// The field is given an automatically incremented location binding.
    ///
    /// The given input expression handle specifies the expression whose value
    /// should be assigned to the field when [`generate_output_code`] is called.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_field_with_perspective_interpolation<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        size: u32,
        input_expr: Handle<Expression>,
    ) -> usize {
        self.add_field(
            name,
            type_handle,
            Some(Interpolation::Perspective),
            Some(Sampling::Center),
            size,
            input_expr,
        )
    }

    /// Adds a new struct field with the built-in position binding rather than a
    /// location binding.
    ///
    /// The field is given an automatically incremented location binding.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_builtin_position_field<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        size: u32,
        input_expr: Handle<Expression>,
    ) -> usize {
        self.input_expressions.push(input_expr);

        self.builder.add_field(
            name,
            type_handle,
            Some(Binding::BuiltIn(BuiltIn::Position { invariant: false })),
            size,
        )
    }

    /// Adds a new struct field with the built-in fragment depth binding rather
    /// than a location binding.
    ///
    /// The field is given an automatically incremented location binding.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_builtin_fragment_depth_field<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        size: u32,
        input_expr: Handle<Expression>,
    ) -> usize {
        self.input_expressions.push(input_expr);

        self.builder.add_field(
            name,
            type_handle,
            Some(Binding::BuiltIn(BuiltIn::FragDepth)),
            size,
        )
    }

    /// Generates code declaring the struct type and adds the struct as the
    /// return type of the given [`Function`]. Also initializes the struct in
    /// the body of the function and generates statements assigning a value to
    /// each field using the expression provided when the field was added,
    /// followed by a return statement.
    pub fn generate_output_code(self, types: &mut UniqueArena<Type>, function: &mut Function) {
        let output_type = insert_in_arena(types, self.builder.into_type());

        function.result = Some(FunctionResult {
            ty: output_type,
            binding: None,
        });

        let output_ptr_expr = append_to_arena(
            &mut function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut function.local_variables,
                LocalVariable {
                    name: new_name("output"),
                    ty: output_type,
                    init: None,
                },
            )),
        );

        for (idx, input_expr) in self.input_expressions.into_iter().enumerate() {
            let output_struct_field_ptr = emit_in_func(function, |function| {
                include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: output_ptr_expr,
                        index: idx as u32,
                    },
                )
            });
            push_to_block(
                &mut function.body,
                Statement::Store {
                    pointer: output_struct_field_ptr,
                    value: input_expr,
                },
            );
        }

        let output_expr = emit_in_func(function, |function| {
            include_named_expr_in_func(
                function,
                "output",
                Expression::Load {
                    pointer: output_ptr_expr,
                },
            )
        });

        push_to_block(
            &mut function.body,
            Statement::Return {
                value: Some(output_expr),
            },
        );
    }

    /// Generates code declaring the struct type (only if not already declared)
    /// and adds the struct as an input argument to the given [`Function`].
    ///
    /// # Returns
    /// An [`InputStruct`] holding the expression for accessing each field in
    /// the body of the function.
    pub fn generate_input_code(
        self,
        types: &mut UniqueArena<Type>,
        function: &mut Function,
    ) -> InputStruct {
        InputStructBuilder {
            builder: self.builder,
            input_arg_name: "input".to_string(),
        }
        .generate_input_code(types, function)
    }
}

impl StructBuilder {
    /// Creates a new builder for a struct with the given type name.
    pub fn new<S: ToString>(type_name: S) -> Self {
        Self {
            type_name: type_name.to_string(),
            fields: Vec::new(),
            offset: 0,
        }
    }

    pub fn n_fields(&self) -> usize {
        self.fields.len()
    }

    /// Adds a new struct field.
    ///
    /// # Returns
    /// The index of the added field.
    pub fn add_field<S: ToString>(
        &mut self,
        name: S,
        type_handle: Handle<Type>,
        binding: Option<Binding>,
        size: u32,
    ) -> usize {
        let idx = self.fields.len();

        self.fields.push(StructMember {
            name: Some(name.to_string()),
            ty: type_handle,
            binding,
            offset: self.offset,
        });

        self.offset += size;

        idx
    }

    /// Creates a struct [`Type`] from the builder.
    pub fn into_type(self) -> Type {
        Type {
            name: Some(self.type_name),
            inner: TypeInner::Struct {
                members: self.fields,
                span: self.offset,
            },
        }
    }
}

impl SampledTexture {
    /// Generates code declaring global variables for a texture and samplers
    /// with the given name root, group and bindings.
    ///
    /// # Returns
    /// A new [`SampledTexture`] with handles to the declared variables.
    pub fn declare(
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        texture_type: TextureType,
        name: &'static str,
        group: u32,
        texture_binding: u32,
        sampler_binding: Option<u32>,
        comparison_sampler_binding: Option<u32>,
    ) -> Self {
        let texture_type_const = match texture_type {
            TextureType::Image2D => IMAGE_2D_TEXTURE_TYPE,
            TextureType::Image2DArray => IMAGE_2D_ARRAY_TEXTURE_TYPE,
            TextureType::ImageCubemap => IMAGE_CUBEMAP_TEXTURE_TYPE,
            TextureType::DepthCubemap => DEPTH_CUBEMAP_TEXTURE_TYPE,
            TextureType::DepthArray => DEPTH_TEXTURE_ARRAY_TYPE,
        };

        let texture_type = insert_in_arena(types, texture_type_const);

        let texture_var = append_to_arena(
            global_variables,
            GlobalVariable {
                name: Some(format!("{}Texture", name)),
                space: AddressSpace::Handle,
                binding: Some(ResourceBinding {
                    group,
                    binding: texture_binding,
                }),
                ty: texture_type,
                init: None,
            },
        );

        let sampler_var = sampler_binding.map(|sampler_binding| {
            let sampler_type = insert_in_arena(types, SAMPLER_TYPE);

            append_to_arena(
                global_variables,
                GlobalVariable {
                    name: Some(format!("{}Sampler", name)),
                    space: AddressSpace::Handle,
                    binding: Some(ResourceBinding {
                        group,
                        binding: sampler_binding,
                    }),
                    ty: sampler_type,
                    init: None,
                },
            )
        });

        let comparison_sampler_var = comparison_sampler_binding.map(|comparison_sampler_binding| {
            let comparison_sampler_type = insert_in_arena(types, COMPARISON_SAMPLER_TYPE);

            append_to_arena(
                global_variables,
                GlobalVariable {
                    name: Some(format!("{}ComparisonSampler", name)),
                    space: AddressSpace::Handle,
                    binding: Some(ResourceBinding {
                        group,
                        binding: comparison_sampler_binding,
                    }),
                    ty: comparison_sampler_type,
                    init: None,
                },
            )
        });

        Self {
            texture_var,
            sampler_var,
            comparison_sampler_var,
        }
    }

    /// Generates and returns expressions for the texture and sampler
    /// (respectively) in the given function. `use_comparison_sampler` specifies
    /// whether the returned sampler should be a comparison sampler.
    ///
    /// # Panics
    /// If the requested sampler type is not available.
    pub fn generate_texture_and_sampler_expressions(
        &self,
        function: &mut Function,
        use_comparison_sampler: bool,
    ) -> (Handle<Expression>, Handle<Expression>) {
        let texture_var_expr =
            include_expr_in_func(function, Expression::GlobalVariable(self.texture_var));

        let sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(if use_comparison_sampler {
                self.comparison_sampler_var
                    .expect("Missing requested comparison sampler")
            } else {
                self.sampler_var.expect("Missing requested sampler")
            }),
        );

        (texture_var_expr, sampler_var_expr)
    }

    /// Generates and returns an expression sampling the texture at the given
    /// texture coordinates and level of detail. If sampling a depth texture, a
    /// reference depth must also be provided for the comparison sampling. If an
    /// array index is provided, it will be used to select the texture to sample
    /// from the texture array. If `gather` is not [`None`], the specified
    /// component of the texture will be sampled in the 2x2 grid of texels
    /// surrounding the texture coordinates, and the returned expression is a
    /// vec4 containing the samples.
    pub fn generate_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
        level: SampleLevel,
        array_index_expr: Option<Handle<Expression>>,
        depth_reference_expr: Option<Handle<Expression>>,
        gather: Option<SwizzleComponent>,
    ) -> Handle<Expression> {
        let (texture_var_expr, sampler_var_expr) =
            self.generate_texture_and_sampler_expressions(function, depth_reference_expr.is_some());

        let sampling_expr = emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::ImageSample {
                    image: texture_var_expr,
                    sampler: sampler_var_expr,
                    gather,
                    coordinate: texture_coord_expr,
                    array_index: array_index_expr,
                    offset: None,
                    level: if gather.is_none() {
                        level
                    } else {
                        SampleLevel::Zero
                    },
                    depth_ref: depth_reference_expr,
                },
            )
        });

        sampling_expr
    }

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression at the given level of
    /// detail and extracting the RGB values of the sampled RGBA color.
    pub fn generate_rgb_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
        level: SampleLevel,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, level, None, None, None);

        emit_in_func(function, |function| {
            include_expr_in_func(function, swizzle_xyz_expr(sampling_expr))
        })
    }

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression at the given level of
    /// detail, and extracting the RG values of the sampled RGBA color.
    pub fn generate_rg_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
        level: SampleLevel,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, level, None, None, None);

        emit_in_func(function, |function| {
            include_expr_in_func(function, swizzle_xy_expr(sampling_expr))
        })
    }

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression, and extracting the
    /// specified channel of the sampled RGBA color (channel index 0 is red, 3
    /// is alpha).
    pub fn generate_single_channel_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
        level: SampleLevel,
        channel_index: u32,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, level, None, None, None);

        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: sampling_expr,
                    index: channel_index,
                },
            )
        })
    }
}

impl PushConstantExpressions {
    pub fn generate(
        module: &mut Module,
        function: &mut Function,
        push_constants: PushConstantGroup,
        stage: PushConstantGroupStage,
    ) -> Self {
        let mut builder = StructBuilder::new(format!("PushConstantsFor{:?}", stage));

        for push_constant in push_constants.iter_for_stage(stage) {
            match push_constant.variant() {
                PushConstantVariant::InverseWindowDimensions => {
                    let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
                    builder.add_field("inverseWindowDimensions", vec2_type, None, VECTOR_2_SIZE);
                }
                PushConstantVariant::PixelCount => {
                    let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
                    builder.add_field("pixelCount", f32_type, None, F32_WIDTH);
                }
                PushConstantVariant::LightIdx => {
                    let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
                    builder.add_field("activeLightIdx", u32_type, None, U32_WIDTH);
                }
                PushConstantVariant::CascadeIdx => {
                    let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
                    builder.add_field("activeCascadeIdx", u32_type, None, U32_WIDTH);
                }
                PushConstantVariant::Exposure => {
                    let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
                    builder.add_field("exposure", f32_type, None, F32_WIDTH);
                }
                PushConstantVariant::InverseExposure => {
                    let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
                    builder.add_field("inverseExposure", f32_type, None, F32_WIDTH);
                }
                PushConstantVariant::FrameCounter => {
                    let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
                    builder.add_field("frameCounter", u32_type, None, U32_WIDTH);
                }
            }
        }

        let n_push_constants = builder.n_fields() as u32;

        let expressions = if n_push_constants > 0 {
            let struct_type = insert_in_arena(&mut module.types, builder.into_type());

            let struct_var_ptr_expr = append_to_arena(
                &mut module.global_variables,
                GlobalVariable {
                    name: new_name(format!("pushConstantsFor{:?}", stage)),
                    space: AddressSpace::PushConstant,
                    binding: None,
                    ty: struct_type,
                    init: None,
                },
            );

            let struct_ptr_expr =
                include_expr_in_func(function, Expression::GlobalVariable(struct_var_ptr_expr));

            (0..n_push_constants)
                .map(|index| {
                    emit_in_func(function, |function| {
                        let pointer = include_expr_in_func(
                            function,
                            Expression::AccessIndex {
                                base: struct_ptr_expr,
                                index,
                            },
                        );
                        include_expr_in_func(function, Expression::Load { pointer })
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            push_constants,
            stage,
            expressions,
        }
    }

    pub fn get(&self, variant: PushConstantVariant) -> Option<Handle<Expression>> {
        self.push_constants
            .find_idx_for_stage(variant, self.stage)
            .map(|idx| self.expressions[idx])
    }
}

impl<'a, 'b> ModuleImporter<'a, 'b> {
    /// Creates a new importer for importing functions from
    /// `imported_from_module` to `exported_to_module`.
    pub fn new(imported_from_module: &'a Module, exported_to_module: &'b mut Module) -> Self {
        Self {
            imported_from_module,
            exported_to_module,
            type_map: HashMap::new(),
            const_map: HashMap::new(),
            global_map: HashMap::new(),
            const_expression_map: HashMap::new(),
            override_map: HashMap::new(),
            function_map: HashMap::new(),
        }
    }

    /// Imports the function with the given handle from the source to the
    /// destination module.
    ///
    /// # Errors
    /// Returns an error if no function with the given handle exists.
    pub fn import_function(&mut self, function: Handle<Function>) -> Result<Handle<Function>> {
        let func = self.imported_from_module.functions.try_get(function)?;
        let name = func.name.as_ref().unwrap().clone();

        let mapped_func = self.localize_function(func);

        let new_h = append_to_arena(&mut self.exported_to_module.functions, mapped_func);
        self.function_map.insert(name, new_h);

        Ok(new_h)
    }

    fn import_type(&mut self, h_type: Handle<Type>) -> Handle<Type> {
        self.type_map.get(&h_type).copied().unwrap_or_else(|| {
            let ty = self
                .imported_from_module
                .types
                .get_handle(h_type)
                .unwrap()
                .clone();

            let name = ty.name.clone();

            let new_type = Type {
                name,
                inner: match &ty.inner {
                    TypeInner::Scalar { .. }
                    | TypeInner::Vector { .. }
                    | TypeInner::Matrix { .. }
                    | TypeInner::ValuePointer { .. }
                    | TypeInner::Image { .. }
                    | TypeInner::Sampler { .. }
                    | TypeInner::Atomic { .. } => ty.clone().inner,

                    TypeInner::Pointer { base, space } => TypeInner::Pointer {
                        base: self.import_type(*base),
                        space: *space,
                    },
                    TypeInner::Struct { members, span } => {
                        let members = members
                            .iter()
                            .map(|m| StructMember {
                                name: m.name.clone(),
                                ty: self.import_type(m.ty),
                                binding: m.binding.clone(),
                                offset: m.offset,
                            })
                            .collect();
                        TypeInner::Struct {
                            members,
                            span: *span,
                        }
                    }
                    TypeInner::Array { base, size, stride } => TypeInner::Array {
                        base: self.import_type(*base),
                        size: *size,
                        stride: *stride,
                    },
                    TypeInner::BindingArray { base, size } => TypeInner::BindingArray {
                        base: self.import_type(*base),
                        size: *size,
                    },
                    TypeInner::AccelerationStructure { .. } | TypeInner::RayQuery { .. } => {
                        panic!("Unsupported type")
                    }
                },
            };
            let new_h = insert_in_arena(&mut self.exported_to_module.types, new_type);
            self.type_map.insert(h_type, new_h);
            new_h
        })
    }

    fn import_const(&mut self, h_const: Handle<Constant>) -> Handle<Constant> {
        self.const_map.get(&h_const).copied().unwrap_or_else(|| {
            let c = self
                .imported_from_module
                .constants
                .try_get(h_const)
                .unwrap()
                .clone();

            let new_const = Constant {
                name: c.name.clone(),
                ty: self.import_type(c.ty),
                init: self.import_const_expression(c.init),
            };

            let new_h =
                define_constant_if_missing(&mut self.exported_to_module.constants, new_const);
            self.const_map.insert(h_const, new_h);
            new_h
        })
    }

    fn import_override(&mut self, h_override: Handle<Override>) -> Handle<Override> {
        self.override_map
            .get(&h_override)
            .copied()
            .unwrap_or_else(|| {
                let o = self
                    .imported_from_module
                    .overrides
                    .try_get(h_override)
                    .unwrap()
                    .clone();

                let new_override = Override {
                    name: o.name.clone(),
                    id: o.id,
                    ty: self.import_type(o.ty),
                    init: o
                        .init
                        .map(|const_expr| self.import_const_expression(const_expr)),
                };

                let new_h = append_to_arena(&mut self.exported_to_module.overrides, new_override);
                self.override_map.insert(h_override, new_h);
                new_h
            })
    }

    fn import_global(&mut self, h_global: Handle<GlobalVariable>) -> Handle<GlobalVariable> {
        self.global_map.get(&h_global).copied().unwrap_or_else(|| {
            let gv = self
                .imported_from_module
                .global_variables
                .try_get(h_global)
                .unwrap()
                .clone();

            let new_global = GlobalVariable {
                name: gv.name.clone(),
                space: gv.space,
                binding: gv.binding.clone(),
                ty: self.import_type(gv.ty),
                init: gv
                    .init
                    .map(|const_expr| self.import_const_expression(const_expr)),
            };

            let new_h = append_to_arena(&mut self.exported_to_module.global_variables, new_global);
            self.global_map.insert(h_global, new_h);
            new_h
        })
    }

    fn import_const_expression(&mut self, h_const_expr: Handle<Expression>) -> Handle<Expression> {
        self.const_expression_map
            .get(&h_const_expr)
            .copied()
            .unwrap_or_else(|| {
                let const_expr = self
                    .imported_from_module
                    .global_expressions
                    .try_get(h_const_expr)
                    .unwrap()
                    .clone();

                let new_const_expr = match const_expr {
                    Expression::Constant(c) => Expression::Constant(self.import_const(c)),
                    Expression::Override(o) => Expression::Override(self.import_override(o)),
                    Expression::Compose { ty, components } => Expression::Compose {
                        ty: self.import_type(ty),
                        components: components
                            .iter()
                            .map(|const_expr| self.import_const_expression(*const_expr))
                            .collect(),
                    },
                    Expression::Access { base, index } => Expression::Access {
                        base: self.import_const_expression(base),
                        index: self.import_const_expression(index),
                    },
                    Expression::AccessIndex { base, index } => Expression::AccessIndex {
                        base: self.import_const_expression(base),
                        index,
                    },
                    Expression::Splat { size, value } => Expression::Splat {
                        size,
                        value: self.import_const_expression(value),
                    },
                    Expression::Swizzle {
                        size,
                        vector,
                        pattern,
                    } => Expression::Swizzle {
                        size,
                        vector: self.import_const_expression(vector),
                        pattern,
                    },
                    Expression::Unary { op, expr } => Expression::Unary {
                        op,
                        expr: self.import_const_expression(expr),
                    },
                    Expression::Binary { op, left, right } => Expression::Binary {
                        op,
                        left: self.import_const_expression(left),
                        right: self.import_const_expression(right),
                    },
                    Expression::Select {
                        condition,
                        accept,
                        reject,
                    } => Expression::Select {
                        condition: self.import_const_expression(condition),
                        accept: self.import_const_expression(accept),
                        reject: self.import_const_expression(reject),
                    },
                    Expression::Relational { fun, argument } => Expression::Relational {
                        fun,
                        argument: self.import_const_expression(argument),
                    },
                    Expression::Math {
                        fun,
                        arg,
                        arg1,
                        arg2,
                        arg3,
                    } => Expression::Math {
                        fun,
                        arg: self.import_const_expression(arg),
                        arg1: arg1.map(|arg1| self.import_const_expression(arg1)),
                        arg2: arg2.map(|arg2| self.import_const_expression(arg2)),
                        arg3: arg3.map(|arg3| self.import_const_expression(arg3)),
                    },
                    Expression::As {
                        expr,
                        kind,
                        convert,
                    } => Expression::As {
                        expr: self.import_const_expression(expr),
                        kind,
                        convert,
                    },
                    Expression::Literal(_) => const_expr,
                    Expression::ZeroValue(ty) => Expression::ZeroValue(self.import_type(ty)),
                    _ => panic!("Invalid variant for constant expression"),
                };

                let new_h = append_to_arena(
                    &mut self.exported_to_module.global_expressions,
                    new_const_expr,
                );
                self.const_expression_map.insert(h_const_expr, new_h);
                new_h
            })
    }

    fn import_block(
        &mut self,
        block: &Block,
        old_expressions: &Arena<Expression>,
        already_imported: &mut HashMap<Handle<Expression>, Handle<Expression>>,
        new_expressions: &mut Arena<Expression>,
    ) -> Block {
        macro_rules! map_expr {
            ($e:expr) => {
                self.import_expression(
                    *$e,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    false,
                )
            };
        }

        macro_rules! map_expr_opt {
            ($e:expr) => {
                $e.as_ref().map(|expr| map_expr!(expr))
            };
        }

        macro_rules! map_block {
            ($b:expr) => {
                self.import_block($b, old_expressions, already_imported, new_expressions)
            };
        }

        let statements = block
            .iter()
            .map(|stmt| {
                match stmt {
                    // Remap function calls
                    Statement::Call {
                        function,
                        arguments,
                        result,
                    } => Statement::Call {
                        function: self.map_function_handle(*function),
                        arguments: arguments.iter().map(|expr| map_expr!(expr)).collect(),
                        result: result.as_ref().map(|result| map_expr!(result)),
                    },

                    // Recursively
                    Statement::Block(b) => Statement::Block(map_block!(b)),
                    Statement::If {
                        condition,
                        accept,
                        reject,
                    } => Statement::If {
                        condition: map_expr!(condition),
                        accept: map_block!(accept),
                        reject: map_block!(reject),
                    },
                    Statement::Switch { selector, cases } => Statement::Switch {
                        selector: map_expr!(selector),
                        cases: cases
                            .iter()
                            .map(|case| SwitchCase {
                                value: case.value,
                                body: map_block!(&case.body),
                                fall_through: case.fall_through,
                            })
                            .collect(),
                    },
                    Statement::Loop {
                        body,
                        continuing,
                        break_if,
                    } => Statement::Loop {
                        body: map_block!(body),
                        continuing: map_block!(continuing),
                        break_if: map_expr_opt!(break_if),
                    },

                    // Map expressions
                    Statement::Emit(exprs) => {
                        // Iterate once to add expressions that should NOT be part of the emit statement
                        for expr in exprs.clone() {
                            self.import_expression(
                                expr,
                                old_expressions,
                                already_imported,
                                new_expressions,
                                true,
                            );
                        }
                        let old_length = new_expressions.len();
                        // Iterate again to add expressions that should be part of the emit statement
                        for expr in exprs.clone() {
                            map_expr!(&expr);
                        }

                        Statement::Emit(new_expressions.range_from(old_length))
                    }
                    Statement::Store { pointer, value } => Statement::Store {
                        pointer: map_expr!(pointer),
                        value: map_expr!(value),
                    },
                    Statement::ImageStore {
                        image,
                        coordinate,
                        array_index,
                        value,
                    } => Statement::ImageStore {
                        image: map_expr!(image),
                        coordinate: map_expr!(coordinate),
                        array_index: map_expr_opt!(array_index),
                        value: map_expr!(value),
                    },
                    Statement::Atomic {
                        pointer,
                        fun,
                        value,
                        result,
                    } => Statement::Atomic {
                        pointer: map_expr!(pointer),
                        fun: *fun,
                        value: map_expr!(value),
                        result: map_expr!(result),
                    },
                    Statement::Return { value } => Statement::Return {
                        value: map_expr_opt!(value),
                    },

                    // Else just copy
                    Statement::Break
                    | Statement::Continue
                    | Statement::Kill
                    | Statement::Barrier(_) => stmt.clone(),

                    Statement::RayQuery { .. }
                    | Statement::WorkGroupUniformLoad { .. }
                    | Statement::SubgroupBallot { .. }
                    | Statement::SubgroupCollectiveOperation { .. }
                    | Statement::SubgroupGather { .. } => {
                        panic!("Unsupported statement")
                    }
                }
            })
            .collect();

        Block::from_vec(statements)
    }

    fn import_expression(
        &mut self,
        h_expr: Handle<Expression>,
        old_expressions: &Arena<Expression>,
        already_imported: &mut HashMap<Handle<Expression>, Handle<Expression>>,
        new_expressions: &mut Arena<Expression>,
        non_emitting_only: bool, // Only brings items that should NOT be emitted into scope
    ) -> Handle<Expression> {
        if let Some(h_new) = already_imported.get(&h_expr) {
            return *h_new;
        }

        macro_rules! map_expr {
            ($e:expr) => {
                self.import_expression(
                    *$e,
                    old_expressions,
                    already_imported,
                    new_expressions,
                    non_emitting_only,
                )
            };
        }

        macro_rules! map_expr_opt {
            ($e:expr) => {
                $e.as_ref().map(|expr| {
                    self.import_expression(
                        *expr,
                        old_expressions,
                        already_imported,
                        new_expressions,
                        non_emitting_only,
                    )
                })
            };
        }

        let mut is_external = false;
        let expr = old_expressions.try_get(h_expr).unwrap();
        let expr = match expr {
            Expression::CallResult(f) => Expression::CallResult(self.map_function_handle(*f)),
            Expression::Constant(c) => {
                is_external = true;
                Expression::Constant(self.import_const(*c))
            }
            Expression::Override(o) => {
                is_external = true;
                Expression::Override(self.import_override(*o))
            }
            Expression::Compose { ty, components } => Expression::Compose {
                ty: self.import_type(*ty),
                components: components.iter().map(|expr| map_expr!(expr)).collect(),
            },
            Expression::GlobalVariable(gv) => {
                is_external = true;
                Expression::GlobalVariable(self.import_global(*gv))
            }
            Expression::ImageSample {
                image,
                sampler,
                gather,
                coordinate,
                array_index,
                offset,
                level,
                depth_ref,
            } => Expression::ImageSample {
                image: map_expr!(image),
                sampler: map_expr!(sampler),
                gather: *gather,
                coordinate: map_expr!(coordinate),
                array_index: map_expr_opt!(array_index),
                offset: offset.map(|const_expr| self.import_const_expression(const_expr)),
                level: match level {
                    SampleLevel::Auto | SampleLevel::Zero => *level,
                    SampleLevel::Exact(expr) => SampleLevel::Exact(map_expr!(expr)),
                    SampleLevel::Bias(expr) => SampleLevel::Bias(map_expr!(expr)),
                    SampleLevel::Gradient { x, y } => SampleLevel::Gradient {
                        x: map_expr!(x),
                        y: map_expr!(y),
                    },
                },
                depth_ref: map_expr_opt!(depth_ref),
            },
            Expression::Access { base, index } => Expression::Access {
                base: map_expr!(base),
                index: map_expr!(index),
            },
            Expression::AccessIndex { base, index } => Expression::AccessIndex {
                base: map_expr!(base),
                index: *index,
            },
            Expression::Splat { size, value } => Expression::Splat {
                size: *size,
                value: map_expr!(value),
            },
            Expression::Swizzle {
                size,
                vector,
                pattern,
            } => Expression::Swizzle {
                size: *size,
                vector: map_expr!(vector),
                pattern: *pattern,
            },
            Expression::Load { pointer } => Expression::Load {
                pointer: map_expr!(pointer),
            },
            Expression::ImageLoad {
                image,
                coordinate,
                array_index,
                sample,
                level,
            } => Expression::ImageLoad {
                image: map_expr!(image),
                coordinate: map_expr!(coordinate),
                array_index: map_expr_opt!(array_index),
                sample: map_expr_opt!(sample),
                level: map_expr_opt!(level),
            },
            Expression::ImageQuery { image, query } => Expression::ImageQuery {
                image: map_expr!(image),
                query: match query {
                    ImageQuery::Size { level } => ImageQuery::Size {
                        level: map_expr_opt!(level),
                    },
                    _ => *query,
                },
            },
            Expression::Unary { op, expr } => Expression::Unary {
                op: *op,
                expr: map_expr!(expr),
            },
            Expression::Binary { op, left, right } => Expression::Binary {
                op: *op,
                left: map_expr!(left),
                right: map_expr!(right),
            },
            Expression::Select {
                condition,
                accept,
                reject,
            } => Expression::Select {
                condition: map_expr!(condition),
                accept: map_expr!(accept),
                reject: map_expr!(reject),
            },
            Expression::Derivative { axis, expr, ctrl } => Expression::Derivative {
                axis: *axis,
                expr: map_expr!(expr),
                ctrl: *ctrl,
            },
            Expression::Relational { fun, argument } => Expression::Relational {
                fun: *fun,
                argument: map_expr!(argument),
            },
            Expression::Math {
                fun,
                arg,
                arg1,
                arg2,
                arg3,
            } => Expression::Math {
                fun: *fun,
                arg: map_expr!(arg),
                arg1: map_expr_opt!(arg1),
                arg2: map_expr_opt!(arg2),
                arg3: map_expr_opt!(arg3),
            },
            Expression::As {
                expr,
                kind,
                convert,
            } => Expression::As {
                expr: map_expr!(expr),
                kind: *kind,
                convert: *convert,
            },
            Expression::ArrayLength(expr) => Expression::ArrayLength(map_expr!(expr)),
            Expression::LocalVariable(_)
            | Expression::FunctionArgument(_)
            | Expression::Literal(_) => {
                is_external = true;
                expr.clone()
            }
            Expression::AtomicResult { .. } => expr.clone(),
            Expression::ZeroValue(ty) => Expression::ZeroValue(self.import_type(*ty)),
            Expression::RayQueryGetIntersection { .. }
            | Expression::RayQueryProceedResult
            | Expression::WorkGroupUniformLoadResult { .. }
            | Expression::SubgroupBallotResult
            | Expression::SubgroupOperationResult { .. } => {
                panic!("Unsupported expression")
            }
        };

        if !non_emitting_only || is_external {
            let h_new = append_to_arena(new_expressions, expr);

            already_imported.insert(h_expr, h_new);
            h_new
        } else {
            h_expr
        }
    }

    fn localize_function(&mut self, func: &Function) -> Function {
        let arguments = func
            .arguments
            .iter()
            .map(|arg| FunctionArgument {
                name: arg.name.clone(),
                ty: self.import_type(arg.ty),
                binding: arg.binding.clone(),
            })
            .collect();

        let result = func.result.as_ref().map(|r| FunctionResult {
            ty: self.import_type(r.ty),
            binding: r.binding.clone(),
        });

        let mut expressions = Arena::new();
        let mut expr_map = HashMap::new();

        let mut local_variables = Arena::new();
        for (h_l, l) in func.local_variables.iter() {
            let new_local = LocalVariable {
                name: l.name.clone(),
                ty: self.import_type(l.ty),
                init: l.init.map(|expr| {
                    self.import_expression(
                        expr,
                        &func.expressions,
                        &mut expr_map,
                        &mut expressions,
                        false,
                    )
                }),
            };
            let new_h = append_to_arena(&mut local_variables, new_local);
            assert_eq!(h_l, new_h);
        }

        let body = self.import_block(
            &func.body,
            &func.expressions,
            &mut expr_map,
            &mut expressions,
        );

        let named_expressions = func
            .named_expressions
            .iter()
            .filter_map(|(h_expr, name)| expr_map.get(h_expr).map(|new_h| (*new_h, name.clone())))
            .collect();

        Function {
            name: func.name.clone(),
            arguments,
            result,
            local_variables,
            expressions,
            named_expressions,
            body,
        }
    }

    fn map_function_handle(&mut self, h_func: Handle<Function>) -> Handle<Function> {
        let func = self.imported_from_module.functions.try_get(h_func).unwrap();
        let name = func.name.as_ref().unwrap();
        self.function_map
            .get(name)
            .copied()
            .unwrap_or_else(|| self.import_function(h_func).unwrap())
    }
}

impl SourceCode {
    /// Parses the given WGSL source code and returns a corresponding
    /// [`SourceCode`] object.
    ///
    /// # Errors
    /// Returns an error if the string contains invalid source code.
    pub fn from_wgsl_source(source: &str) -> Result<Self> {
        let module = naga::front::wgsl::parse_str(source)?;

        let available_functions = module
            .functions
            .iter()
            .map(|(function_handle, function)| {
                (function.name.as_ref().unwrap().to_string(), function_handle)
            })
            .collect();

        let available_named_types = module
            .types
            .iter()
            .filter_map(|(type_handle, ty)| {
                ty.name
                    .as_ref()
                    .map(|type_name| (type_name.to_string(), type_handle))
            })
            .collect();

        Ok(Self {
            module,
            available_functions,
            available_named_types,
            used_functions: HashMap::new(),
            used_types: HashMap::new(),
        })
    }

    /// Generates the code calling the function with the given name with the
    /// given argument expressions within the given parent function. The called
    /// function will be imported to the given module if it has not already been
    /// imported.
    ///
    /// # Returns
    /// The return value expression.
    ///
    /// # Panics
    /// If no function with the requested name exists in the source code.
    ///
    /// # Warning
    /// As the handles of previously imported functions are cached, calling this
    /// method on the same [`SourceCode`] instance with multiple [`Module`]s may
    /// cause incorrect functions to be called.
    pub fn generate_function_call(
        &mut self,
        module: &mut Module,
        parent_function: &mut Function,
        function_name: &str,
        arguments: Vec<Handle<Expression>>,
    ) -> Handle<Expression> {
        self.generate_function_call_in_block(
            module,
            &mut parent_function.body,
            &mut parent_function.expressions,
            function_name,
            arguments,
        )
    }

    /// Generates the code calling the function with the given name with the
    /// given argument expressions within the given statement block. The called
    /// function will be imported to the given module if it has not already been
    /// imported.
    ///
    /// # Returns
    /// The return value expression.
    ///
    /// # Panics
    /// If no function with the requested name exists in the source code.
    ///
    /// # Warning
    /// As the handles of previously imported functions are cached, calling this
    /// method on the same [`SourceCode`] instance with multiple [`Module`]s may
    /// cause incorrect functions to be called.
    pub fn generate_function_call_in_block(
        &mut self,
        module: &mut Module,
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        function_name: &str,
        arguments: Vec<Handle<Expression>>,
    ) -> Handle<Expression> {
        let imported_function_handle = *self
            .used_functions
            .entry(function_name.to_string())
            .or_insert_with(|| {
                let original_function_handle = *self
                    .available_functions
                    .get(function_name)
                    .unwrap_or_else(|| {
                        panic!(
                            "Requested missing function from shader library: {}",
                            function_name
                        )
                    });

                let mut importer = ModuleImporter::new(&self.module, module);
                importer.import_function(original_function_handle).unwrap()
            });

        let return_expr = append_to_arena(
            expressions,
            Expression::CallResult(imported_function_handle),
        );

        push_to_block(
            block,
            Statement::Call {
                function: imported_function_handle,
                arguments,
                result: Some(return_expr),
            },
        );

        return_expr
    }

    /// Returns the handle for the type with the given name. The requested type
    /// will be imported to the given module if it has not already been
    /// imported.
    ///
    /// # Panics
    /// If no type with the requested name exists in the source code.
    ///
    /// # Warning
    /// As the handles of previously imported types are cached, calling this
    /// method on the same [`SourceCode`] instance with multiple [`Module`]s may
    /// cause incorrect types to be returned.
    pub fn use_type(&mut self, module: &mut Module, type_name: &str) -> Handle<Type> {
        let imported_type_handle =
            *self
                .used_types
                .entry(type_name.to_string())
                .or_insert_with(|| {
                    let original_type_handle = *self
                        .available_named_types
                        .get(type_name)
                        .unwrap_or_else(|| {
                            panic!("Requested missing type from shader library: {}", type_name)
                        });

                    let mut importer = ModuleImporter::new(&self.module, module);
                    importer.import_type(original_type_handle)
                });

        imported_type_handle
    }
}

/// Adds an input argument with the given name, type and binding location to the
/// given function. The location binding is assumed to use no interpolation or
/// sampling.
///
/// # Returns
/// An expression in the function body referring to the input argument.
pub fn generate_location_bound_input_argument(
    function: &mut Function,
    input_arg_name: Option<String>,
    input_type: Handle<Type>,
    location: u32,
) -> Handle<Expression> {
    generate_input_argument(
        function,
        input_arg_name,
        input_type,
        Some(Binding::Location {
            location,
            second_blend_source: false,
            interpolation: None,
            sampling: None,
        }),
    )
}

/// Adds an input argument with the given name, type and binding to the given
/// function.
///
/// # Returns
/// An expression in the function body referring to the input argument.
pub fn generate_input_argument(
    function: &mut Function,
    input_arg_name: Option<String>,
    input_type: Handle<Type>,
    binding: Option<Binding>,
) -> Handle<Expression> {
    let input_arg_idx = u32::try_from(function.arguments.len()).unwrap();

    function.arguments.push(FunctionArgument {
        name: input_arg_name,
        ty: input_type,
        binding,
    });

    include_expr_in_func(function, Expression::FunctionArgument(input_arg_idx))
}

fn new_name<S: ToString>(name_str: S) -> Option<String> {
    Some(name_str.to_string())
}

fn swizzle_x_expr(expr: Handle<Expression>) -> Expression {
    Expression::AccessIndex {
        base: expr,
        index: 0,
    }
}

fn swizzle_y_expr(expr: Handle<Expression>) -> Expression {
    Expression::AccessIndex {
        base: expr,
        index: 1,
    }
}

fn swizzle_z_expr(expr: Handle<Expression>) -> Expression {
    Expression::AccessIndex {
        base: expr,
        index: 2,
    }
}

fn swizzle_w_expr(expr: Handle<Expression>) -> Expression {
    Expression::AccessIndex {
        base: expr,
        index: 3,
    }
}

fn swizzle_xy_expr(expr: Handle<Expression>) -> Expression {
    Expression::Swizzle {
        size: VectorSize::Bi,
        vector: expr,
        pattern: [
            SwizzleComponent::X,
            SwizzleComponent::Y,
            SwizzleComponent::X,
            SwizzleComponent::X,
        ],
    }
}

fn swizzle_xyz_expr(expr: Handle<Expression>) -> Expression {
    Expression::Swizzle {
        size: VectorSize::Tri,
        vector: expr,
        pattern: [
            SwizzleComponent::X,
            SwizzleComponent::Y,
            SwizzleComponent::Z,
            SwizzleComponent::X,
        ],
    }
}

/// Inserts the given value in the given [`UniqueArena`] if it is not already
/// present.
///
/// # Returns
/// A handle to the unique value.
fn insert_in_arena<T>(arena: &mut UniqueArena<T>, value: T) -> Handle<T>
where
    T: Eq + Hash,
{
    arena.insert(value, Span::UNDEFINED)
}

/// Appends the given value to the given [`Arena`].
///
/// # Returns
/// A handle to the appended value.
fn append_to_arena<T>(arena: &mut Arena<T>, value: T) -> Handle<T> {
    arena.append(value, Span::UNDEFINED)
}

/// Pushes the given [`Statement`] to the given [`Block`] of statements.
fn push_to_block(block: &mut Block, statement: Statement) {
    block.push(statement, Span::UNDEFINED);
}

/// Appends the given constant to the given constant [`Arena`] if it does not
/// already exist.
///
/// # Returns
/// A handle to the appended or existing constant.
fn define_constant_if_missing(
    constants: &mut Arena<Constant>,
    constant: Constant,
) -> Handle<Constant> {
    constants.fetch_or_append(constant, Span::UNDEFINED)
}

/// Executes the given closure that adds [`Expression`]s to the given
/// [`Function`] before pushing to the function body a [`Statement::Emit`]
/// emitting the range of added expressions.
///
/// # Returns
/// The value returned from the closure.
fn emit_in_func<T>(function: &mut Function, add_expressions: impl FnOnce(&mut Function) -> T) -> T {
    let start_length = function.expressions.len();
    let ret = add_expressions(function);
    let emit_statement = Statement::Emit(function.expressions.range_from(start_length));
    push_to_block(&mut function.body, emit_statement);
    ret
}

/// Includes the given expression in the the given function.
///
/// # Returns
/// A handle to the included expression.
fn include_expr_in_func(function: &mut Function, expr: Expression) -> Handle<Expression> {
    append_to_arena(&mut function.expressions, expr)
}

/// Includes the given expression as a named expression with the given name in
/// the given function.
///
/// # Returns
/// A handle to the included expression.
fn include_named_expr_in_func<S: ToString>(
    function: &mut Function,
    name: S,
    expr: Expression,
) -> Handle<Expression> {
    let handle = include_expr_in_func(function, expr);
    function.named_expressions.insert(handle, name.to_string());
    handle
}

/// Takes an expression for a `vec3<f32>` and generates an expression for a
/// `vec4<f32>` where the first three components are the same as in the `vec3`
/// and the last component is 1.0.
fn append_unity_component_to_vec3(
    types: &mut UniqueArena<Type>,
    function: &mut Function,
    vec3_expr: Handle<Expression>,
) -> Handle<Expression> {
    append_literal_to_vec3(types, function, vec3_expr, 1.0)
}

/// Takes an expression for a `vec3<f32>` and generates an expression for a
/// `vec4<f32>` where the first three components are the same as in the `vec3`
/// and the last component is the given float literal.
fn append_literal_to_vec3(
    types: &mut UniqueArena<Type>,
    function: &mut Function,
    vec3_expr: Handle<Expression>,
    literal: f32,
) -> Handle<Expression> {
    let vec4_type = insert_in_arena(types, VECTOR_4_TYPE);

    let literal_constant_expr =
        include_expr_in_func(function, Expression::Literal(Literal::F32(literal)));

    emit_in_func(function, |function| {
        include_expr_in_func(
            function,
            Expression::Compose {
                ty: vec4_type,
                components: vec![vec3_expr, literal_constant_expr],
            },
        )
    })
}
