//! Generation of graphics shaders.

mod ambient_color;
mod blinn_phong;
mod fixed;
mod vertex_color;

pub use ambient_color::GlobalAmbientColorShaderInput;
pub use blinn_phong::{BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput};
pub use fixed::{FixedColorFeatureShaderInput, FixedTextureShaderInput};

use crate::{
    geometry::{
        VertexAttribute, VertexAttributeSet, VertexColor, VertexNormalVector, VertexPosition,
        VertexTextureCoords, N_VERTEX_ATTRIBUTES,
    },
    rendering::{fre, CoreRenderingSystem},
    scene::MAX_SHADOW_MAP_CASCADES,
};
use ambient_color::GlobalAmbientColorShaderGenerator;
use anyhow::{anyhow, bail, Result};
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use fixed::{
    FixedColorShaderGenerator, FixedColorVertexOutputFieldIdx, FixedTextureShaderGenerator,
};
use naga::{
    AddressSpace, Arena, ArraySize, BinaryOperator, Binding, Block, BuiltIn, Bytes, Constant,
    ConstantInner, EntryPoint, Expression, Function, FunctionArgument, FunctionResult,
    GlobalVariable, Handle, ImageClass, ImageDimension, ImageQuery, Interpolation, LocalVariable,
    Module, ResourceBinding, SampleLevel, Sampling, ScalarKind, ScalarValue, ShaderStage, Span,
    Statement, StructMember, SwitchCase, SwizzleComponent, Type, TypeInner, UnaryOperator,
    UniqueArena, VectorSize,
};
use std::{borrow::Cow, collections::HashMap, hash::Hash, mem, vec};
use vertex_color::VertexColorShaderGenerator;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::{fs, path::Path};
    }
}

/// A graphics shader program.
#[derive(Debug)]
pub struct Shader {
    module: wgpu::ShaderModule,
    entry_point_names: EntryPointNames,
    #[cfg(debug_assertions)]
    source_code: String,
}

/// Names of the different shader entry point functions.
#[derive(Clone, Debug)]
pub struct EntryPointNames {
    /// Name of the vertex entry point function.
    pub vertex: Cow<'static, str>,
    /// Name of the fragment entry point function, or [`None`] if there is no
    /// fragment entry point.
    pub fragment: Option<Cow<'static, str>>,
}

/// Generator for shader programs.
#[derive(Clone, Debug)]
pub struct ShaderGenerator;

/// Input description specifying the uniform binding of the
/// projection matrix of the camera to use in the shader.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraShaderInput {
    /// Bind group binding of the uniform buffer holding the
    /// camera projection matrix.
    pub projection_matrix_binding: u32,
}

/// Input description specifying the locations of the available vertex
/// attributes of the mesh to use in the shader.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshShaderInput {
    pub locations: [Option<u32>; N_VERTEX_ATTRIBUTES],
}

/// Input description for any kind of per-instance feature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InstanceFeatureShaderInput {
    ModelViewTransform(ModelViewTransformShaderInput),
    FixedColorMaterial(FixedColorFeatureShaderInput),
    BlinnPhongMaterial(BlinnPhongFeatureShaderInput),
    /// For convenience in unit tests.
    #[cfg(test)]
    None,
}

/// Input description for any kind of material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaterialShaderInput {
    GlobalAmbientColor(GlobalAmbientColorShaderInput),
    VertexColor,
    Fixed(Option<FixedTextureShaderInput>),
    BlinnPhong(Option<BlinnPhongTextureShaderInput>),
}

/// Input description specifying the vertex attribute locations of the
/// components of the model view transform.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelViewTransformShaderInput {
    /// Vertex attribute location for the rotation quaternion.
    pub rotation_location: u32,
    /// Vertex attribute locations for the 4-element vector containing the
    /// translation vector and the scaling factor.
    pub translation_and_scaling_location: u32,
}

/// Shader input description for a specific light source type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LightShaderInput {
    PointLight(PointLightShaderInput),
    UnidirectionalLight(UnidirectionalLightShaderInput),
}

/// Input description for point light sources, specifying the bind group
/// binding and the total size of the point light uniform buffer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
    /// Bind group bindings of the shadow map texture, sampler and comparison
    /// sampler, respectively.
    pub shadow_map_texture_and_sampler_binding: (u32, u32, u32),
}

/// Input description for unidirectional light sources, specifying the bind
/// group binding and the total size of the unidirectional light uniform buffer
/// as well as shadow map bindings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnidirectionalLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
    /// Bind group bindings of the shadow map texture, sampler and comparison
    /// sampler, respectively.
    pub shadow_map_texture_and_sampler_bindings: (u32, u32, u32),
}

/// Shader generator for any kind of material.
#[derive(Clone, Debug)]
pub enum MaterialShaderGenerator<'a> {
    GlobalAmbientColor(GlobalAmbientColorShaderGenerator<'a>),
    VertexColor,
    FixedColor(FixedColorShaderGenerator<'a>),
    FixedTexture(FixedTextureShaderGenerator<'a>),
    BlinnPhong(BlinnPhongShaderGenerator<'a>),
}

/// Handles to expressions for accessing the rotational, translational and
/// scaling components of the model view transform variable.
#[derive(Clone, Debug)]
pub struct ModelViewTransformExpressions {
    pub rotation_quaternion: Handle<Expression>,
    pub translation_vector: Handle<Expression>,
    pub scaling_factor: Handle<Expression>,
}

/// Handle to expressions for a projection.
#[derive(Clone, Debug)]
pub enum ProjectionExpressions {
    Camera(CameraProjectionExpressions),
    PointLight(PointLightProjectionExpressions),
    UnidirectionalLight(UnidirectionalLightProjectionExpressions),
}

/// Handle to expression for the camera projection matrix.
#[derive(Clone, Debug)]
pub struct CameraProjectionExpressions {
    matrix: Handle<Expression>,
}

/// Marker type with method for projecting points onto a face of a shadow
/// cubemap.
#[derive(Clone, Debug)]
pub struct PointLightProjectionExpressions;

/// Handle to expressions for the orthographic transform components associated
/// with a unidirectional light.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightProjectionExpressions {
    pub translation: Handle<Expression>,
    pub scaling: Handle<Expression>,
}

/// Generator for shader code associated with a light source.
#[derive(Clone, Debug)]
pub enum LightShaderGenerator {
    PointLight(PointLightShaderGenerator),
    UnidirectionalLight(UnidirectionalLightShaderGenerator),
}

/// Generator for shader code associated with a point light source.
#[derive(Clone, Debug)]
pub enum PointLightShaderGenerator {
    ForShadowMapUpdate(PointLightShadowMapUpdateShaderGenerator),
    ForShading(PointLightShadingShaderGenerator),
}

/// Generator for shader code associated with a unidirectional light source.
#[derive(Clone, Debug)]
pub enum UnidirectionalLightShaderGenerator {
    ForShadowMapUpdate(UnidirectionalLightShadowMapUpdateShaderGenerator),
    ForShading(UnidirectionalLightShadingShaderGenerator),
}

/// Generator for shader code for updating the shadow cubemap of a point light.
#[derive(Clone, Debug)]
pub struct PointLightShadowMapUpdateShaderGenerator {
    pub near_distance: Handle<Expression>,
    pub inverse_distance_span: Handle<Expression>,
}

/// Generator for shader code for shading a fragment with the light from a point
/// light.
#[derive(Clone, Debug)]
pub struct PointLightShadingShaderGenerator {
    pub camera_to_light_space_rotation_quaternion: Handle<Expression>,
    pub camera_space_position: Handle<Expression>,
    pub radiance: Handle<Expression>,
    pub emission_radius: Handle<Expression>,
    pub near_distance: Handle<Expression>,
    pub inverse_distance_span: Handle<Expression>,
    pub shadow_map: SampledTexture,
}

/// Generator for shader code for updating the shadow map of a unidirectional
/// light source.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightShadowMapUpdateShaderGenerator {
    pub orthographic_projection: UnidirectionalLightProjectionExpressions,
}

/// Generator for shading a fragment with the light from a unidirectional light
/// source.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightShadingShaderGenerator {
    pub active_light_ptr_expr_in_vertex_function: Handle<Expression>,
    pub active_light_ptr_expr_in_fragment_function: Handle<Expression>,
    pub shadow_map: SampledTexture,
}

/// Indices of the fields holding the various mesh vertex attributes and related
/// quantities in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MeshVertexOutputFieldIndices {
    pub clip_position: usize,
    pub position: Option<usize>,
    pub color: Option<usize>,
    pub normal_vector: Option<usize>,
    pub texture_coords: Option<usize>,
}

/// Indices of the fields holding the various light related properties for the
/// relevant light type in the vertex shader output struct.
#[derive(Clone, Debug)]
pub enum LightVertexOutputFieldIndices {
    UnidirectionalLight(UnidirectionalLightVertexOutputFieldIndices),
}

/// Indices of the fields holding the various unidirectional light related
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightVertexOutputFieldIndices {
    pub light_space_position: usize,
    pub light_space_normal_vector: usize,
}

/// Indices of any fields holding the properties of a
/// specific material in the vertex shader output struct.
#[derive(Clone, Debug)]
pub enum MaterialVertexOutputFieldIndices {
    FixedColor(FixedColorVertexOutputFieldIdx),
    BlinnPhong(BlinnPhongVertexOutputFieldIndices),
    None,
}

/// Represents a struct passed as an argument to a shader
/// entry point. Holds the handles for the expressions
/// accessing each field of the struct.
#[derive(Clone, Debug)]
pub struct InputStruct {
    input_field_expressions: Vec<Handle<Expression>>,
}

/// Helper for constructing a struct [`Type`] for an
/// argument to a shader entry point and generating
/// the code for accessing its fields.
#[derive(Clone, Debug)]
pub struct InputStructBuilder {
    builder: StructBuilder,
    input_arg_name: String,
}

/// Helper for constructing a struct [`Type`] for a
/// shader entry point return value and generating
/// the code for assigning to its fields and returning
/// its value.
#[derive(Clone, Debug)]
pub struct OutputStructBuilder {
    builder: StructBuilder,
    input_expressions: Vec<Handle<Expression>>,
    location: u32,
}

/// Helper for constructing a struct [`Type`].
#[derive(Clone, Debug)]
pub struct StructBuilder {
    type_name: String,
    fields: Vec<StructMember>,
    offset: u32,
}

/// Helper for declaring global variables for a texture
/// with an associated sampler and generating a sampling
/// expression.
#[derive(Clone, Debug)]
pub struct SampledTexture {
    texture_var: Handle<GlobalVariable>,
    sampler_var: Option<Handle<GlobalVariable>>,
    comparison_sampler_var: Option<Handle<GlobalVariable>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureType {
    Image,
    DepthCubemap,
    DepthArray,
}

/// Helper for importing functions from one module to another.
///
/// This is an adaptation of the `DerivedModule` type in v0.5.0 of the
/// `naga_oil` library by robtfm: <https://github.com/robtfm/naga_oil>.
#[derive(Debug)]
pub struct ModuleImporter<'a, 'b> {
    imported_from_module: &'a Module,
    exported_to_module: &'b mut Module,
    type_map: HashMap<Handle<Type>, Handle<Type>>,
    const_map: HashMap<Handle<Constant>, Handle<Constant>>,
    global_map: HashMap<Handle<GlobalVariable>, Handle<GlobalVariable>>,
    function_map: HashMap<String, Handle<Function>>,
}

/// A set of shader functions and types defined in source code that can be
/// imported into an existing [`Module`].
#[derive(Debug)]
pub struct SourceCode {
    module: Module,
}

/// Handles to functions and named types imported into a [`Module`].
#[derive(Clone, Debug)]
pub struct SourceCodeHandles {
    /// Handles to imported functions, where the keys are the function names.
    pub functions: HashMap<String, Handle<Function>>,
    /// Handles to imported named types, where the keys are the type names.
    pub types: HashMap<String, Handle<Type>>,
}

const U32_WIDTH: u32 = mem::size_of::<u32>() as u32;

const U32_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Scalar {
        kind: ScalarKind::Uint,
        width: U32_WIDTH as Bytes,
    },
};

const F32_WIDTH: u32 = mem::size_of::<f32>() as u32;

const F32_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Scalar {
        kind: ScalarKind::Float,
        width: F32_WIDTH as Bytes,
    },
};

const VECTOR_2_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Bi,
        kind: ScalarKind::Float,
        width: F32_WIDTH as Bytes,
    },
};
const VECTOR_2_SIZE: u32 = 2 * F32_WIDTH;

const VECTOR_3_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Tri,
        kind: ScalarKind::Float,
        width: F32_WIDTH as Bytes,
    },
};
const VECTOR_3_SIZE: u32 = 3 * F32_WIDTH;

const VECTOR_4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Quad,
        kind: ScalarKind::Float,
        width: F32_WIDTH as Bytes,
    },
};
const VECTOR_4_SIZE: u32 = 4 * F32_WIDTH;

const MATRIX_4X4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Matrix {
        columns: VectorSize::Quad,
        rows: VectorSize::Quad,
        width: F32_WIDTH as Bytes,
    },
};

const IMAGE_TEXTURE_TYPE: Type = Type {
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

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_wgsl_path(
        core_system: &CoreRenderingSystem,
        shader_path: impl AsRef<Path>,
        entry_point_names: EntryPointNames,
    ) -> Result<Self> {
        let shader_path = shader_path.as_ref();
        let label = shader_path.to_string_lossy();
        let source = fs::read_to_string(shader_path)?;
        Ok(Self::from_wgsl_source(
            core_system,
            &source,
            entry_point_names,
            label.as_ref(),
        ))
    }

    /// Creates a new shader from the given source code.
    pub fn from_wgsl_source(
        core_system: &CoreRenderingSystem,
        source: &str,
        entry_point_names: EntryPointNames,
        label: &str,
    ) -> Self {
        let module = core_system
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
                label: Some(label),
            });
        Self {
            module,
            entry_point_names,
            #[cfg(debug_assertions)]
            source_code: source.to_string(),
        }
    }

    /// Creates a new shader from the given [`Module`].
    pub fn from_naga_module(
        core_system: &CoreRenderingSystem,
        naga_module: Module,
        entry_point_names: EntryPointNames,
        label: &str,
    ) -> Self {
        #[cfg(debug_assertions)]
        let source_code = Self::generate_wgsl_from_naga_module(&naga_module);

        let module = core_system
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                source: wgpu::ShaderSource::Naga(Cow::Owned(naga_module)),
                label: Some(label),
            });

        Self {
            module,
            entry_point_names,
            #[cfg(debug_assertions)]
            source_code,
        }
    }

    /// Returns a reference to the compiled shader module
    /// containing the vertex stage entry point.
    pub fn vertex_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns a reference to the compiled shader module
    /// containing the fragment stage entry point.
    pub fn fragment_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }

    /// Returns the name of the vertex entry point function.
    pub fn vertex_entry_point_name(&self) -> &str {
        &self.entry_point_names.vertex
    }

    /// Returns the name of the fragment entry point function, or [`None`] if
    /// there is no fragment entry point.
    pub fn fragment_entry_point_name(&self) -> Option<&str> {
        self.entry_point_names
            .fragment
            .as_ref()
            .map(|name| name.as_ref())
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

#[cfg(debug_assertions)]
impl std::fmt::Display for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.source_code)
    }
}

impl ShaderGenerator {
    /// Uses the given camera, mesh, light, model and material input
    /// descriptions to generate an appropriate shader [`Module`], containing a
    /// vertex and (optionally) a fragment entry point.
    ///
    /// # Returns
    /// The generated shader [`Module`] and its [`EntryPointNames`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - There is no mesh input (no shaders witout a mesh supported).
    /// - `instance_feature_shader_inputs` does not contain a
    ///   [`ModelInstanceTransformShaderInput`] (no shaders without a model view
    ///   transform supported).
    /// - `instance_feature_shader_inputs` and `material_shader_input` do not
    ///   provide a consistent and supported material description.
    /// - Not all vertex attributes required by the material are available in
    ///   the input mesh.
    pub fn generate_shader_module(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_shader_input: Option<&MaterialShaderInput>,
        mut vertex_attribute_requirements: VertexAttributeSet,
    ) -> Result<(Module, EntryPointNames)> {
        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let (model_view_transform_shader_input, material_shader_generator) =
            Self::interpret_inputs(instance_feature_shader_inputs, material_shader_input)?;

        let mut module = Module::default();
        let mut vertex_function = Function::default();
        let mut fragment_function = Function::default();

        // Caution: The order in which the shader generators use and increment
        // the bind group index must match the order in which the bind groups
        // are set in `RenderPassRecorder::record_render_pass`, that is:
        // 1. Camera.
        // 2. Lights.
        // 3. Shadow map textures.
        // 4. Fixed material resources.
        // 5. Material property textures.
        let mut bind_group_idx = 0;

        let camera_projection = camera_shader_input.map(|camera_shader_input| {
            Self::generate_vertex_code_for_projection_matrix(
                camera_shader_input,
                &mut module,
                &mut vertex_function,
                &mut bind_group_idx,
            )
        });

        let model_view_transform = Self::generate_vertex_code_for_model_view_transform(
            model_view_transform_shader_input,
            &mut module,
            &mut vertex_function,
        );

        let light_shader_generator = light_shader_input.map(|light_shader_input| {
            Self::create_light_shader_generator(
                light_shader_input,
                &mut module,
                &mut vertex_function,
                &mut fragment_function,
                &mut bind_group_idx,
                &mut vertex_attribute_requirements,
                material_shader_generator.is_some(),
            )
        });

        let projection = if let Some(camera_projection) = camera_projection {
            ProjectionExpressions::Camera(camera_projection)
        } else if material_shader_generator.is_some() {
            bail!("Tried to build shader with material but no camera");
        } else {
            light_shader_generator
                .as_ref()
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to build shader with no camera or light input (missing projection)"
                    )
                })?
                .get_projection_to_light_clip_space()
                .unwrap()
        };

        let (mesh_vertex_output_field_indices, mut vertex_output_struct_builder) =
            Self::generate_vertex_code_for_vertex_attributes(
                mesh_shader_input,
                vertex_attribute_requirements,
                &mut module,
                &mut vertex_function,
                &model_view_transform,
                projection,
            )?;

        let entry_point_names = if let Some(material_shader_generator) = material_shader_generator {
            let light_vertex_output_field_indices =
                light_shader_generator.as_ref().and_then(|light| {
                    light.generate_vertex_output_code_for_shading(
                        &mut module,
                        &mut vertex_function,
                        &mut vertex_output_struct_builder,
                        &mesh_vertex_output_field_indices,
                    )
                });

            let material_vertex_output_field_indices = material_shader_generator
                .generate_vertex_code(
                    &mut module,
                    &mut vertex_function,
                    &mut vertex_output_struct_builder,
                );

            vertex_output_struct_builder
                .clone()
                .generate_output_code(&mut module.types, &mut vertex_function);

            let fragment_input_struct = vertex_output_struct_builder
                .generate_input_code(&mut module.types, &mut fragment_function);

            material_shader_generator.generate_fragment_code(
                &mut module,
                &mut fragment_function,
                &mut bind_group_idx,
                &fragment_input_struct,
                &mesh_vertex_output_field_indices,
                light_vertex_output_field_indices.as_ref(),
                &material_vertex_output_field_indices,
                light_shader_generator.as_ref(),
            );

            EntryPointNames {
                vertex: Cow::Borrowed("mainVS"),
                fragment: Some(Cow::Borrowed("mainFS")),
            }
        } else {
            vertex_output_struct_builder
                .clone()
                .generate_output_code(&mut module.types, &mut vertex_function);

            let fragment_entry_point_name =
                if let Some(light_shader_generator) = light_shader_generator {
                    if light_shader_generator.has_fragment_output() {
                        let fragment_input_struct = vertex_output_struct_builder
                            .generate_input_code(&mut module.types, &mut fragment_function);

                        light_shader_generator.generate_fragment_output_code(
                            &mut module,
                            &mut fragment_function,
                            &fragment_input_struct,
                            &mesh_vertex_output_field_indices,
                        );

                        Some(Cow::Borrowed("mainFS"))
                    } else {
                        None
                    }
                } else {
                    None
                };

            EntryPointNames {
                vertex: Cow::Borrowed("mainVS"),
                fragment: fragment_entry_point_name,
            }
        };

        module.entry_points.push(EntryPoint {
            name: entry_point_names.vertex.to_string(),
            stage: ShaderStage::Vertex,
            early_depth_test: None,
            workgroup_size: [0, 0, 0],
            function: vertex_function,
        });

        if let Some(name) = entry_point_names.fragment.as_ref() {
            module.entry_points.push(EntryPoint {
                name: name.to_string(),
                stage: ShaderStage::Fragment,
                early_depth_test: None,
                workgroup_size: [0, 0, 0],
                function: fragment_function,
            });
        }

        Ok((module, entry_point_names))
    }

    /// Interprets the set of instance feature, material and and material
    /// property texture inputs to gather them into groups of inputs that belong
    /// together, most notably gathering the inputs representing the material
    /// into a [`MaterialShaderGenerator`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - `instance_feature_shader_inputs` does not contain a
    ///   [`ModelInstanceTransformShaderInput`].
    /// - `instance_feature_shader_inputs`, `material_shader_input` do not
    ///   provide a consistent and supported material description.
    ///
    /// # Panics
    /// If `instance_feature_shader_inputs` contain multiple inputs of the same
    /// type.
    fn interpret_inputs<'a>(
        instance_feature_shader_inputs: &'a [&'a InstanceFeatureShaderInput],
        material_shader_input: Option<&'a MaterialShaderInput>,
    ) -> Result<(
        &'a ModelViewTransformShaderInput,
        Option<MaterialShaderGenerator<'a>>,
    )> {
        let mut model_view_transform_shader_input = None;
        let mut fixed_color_feature_shader_input = None;
        let mut blinn_phong_feature_shader_input = None;

        for &instance_feature_shader_input in instance_feature_shader_inputs {
            match instance_feature_shader_input {
                InstanceFeatureShaderInput::ModelViewTransform(shader_input) => {
                    let old = model_view_transform_shader_input.replace(shader_input);
                    // There should not be multiple instance feature inputs of
                    // the same type
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

        let model_view_transform_shader_input =
            model_view_transform_shader_input.ok_or_else(|| {
                anyhow!("Tried to build shader with no instance model view transform input")
            })?;

        let material_shader_builder = match (
            fixed_color_feature_shader_input,
            blinn_phong_feature_shader_input,
            material_shader_input,
        ) {
            (None, None, None) => None,
            (None, None, Some(MaterialShaderInput::GlobalAmbientColor(input))) => {
                Some(MaterialShaderGenerator::GlobalAmbientColor(
                    GlobalAmbientColorShaderGenerator::new(input),
                ))
            }
            (Some(feature_input), None, Some(MaterialShaderInput::Fixed(None))) => Some(
                MaterialShaderGenerator::FixedColor(FixedColorShaderGenerator::new(feature_input)),
            ),
            (None, None, Some(MaterialShaderInput::Fixed(Some(texture_input)))) => {
                Some(MaterialShaderGenerator::FixedTexture(
                    FixedTextureShaderGenerator::new(texture_input),
                ))
            }
            (None, Some(feature_input), Some(MaterialShaderInput::BlinnPhong(texture_input))) => {
                Some(MaterialShaderGenerator::BlinnPhong(
                    BlinnPhongShaderGenerator::new(feature_input, texture_input.as_ref()),
                ))
            }
            (None, None, Some(MaterialShaderInput::VertexColor)) => {
                Some(MaterialShaderGenerator::VertexColor)
            }
            input => {
                return Err(anyhow!(
                    "Tried to build shader with invalid material: {:?}",
                    input
                ));
            }
        };

        Ok((model_view_transform_shader_input, material_shader_builder))
    }

    /// Generates the declaration of the model view transform type, adds it as
    /// an argument to the main vertex shader function and generates expressions
    /// for the rotational, translational and scaling components of the
    /// transformation in the body of the function.
    ///
    /// # Returns
    /// A [`ModelViewTransformExpressions`] with handles to expressions for the
    /// components of the transformation.
    fn generate_vertex_code_for_model_view_transform(
        model_view_transform_shader_input: &ModelViewTransformShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
    ) -> ModelViewTransformExpressions {
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let model_view_transform_type = Type {
            name: new_name("ModelViewTransform"),
            inner: TypeInner::Struct {
                members: vec![
                    StructMember {
                        name: new_name("rotationQuaternion"),
                        ty: vec4_type,
                        binding: Some(Binding::Location {
                            location: model_view_transform_shader_input.rotation_location,
                            interpolation: None,
                            sampling: None,
                        }),
                        offset: 0,
                    },
                    StructMember {
                        name: new_name("translationAndScaling"),
                        ty: vec4_type,
                        binding: Some(Binding::Location {
                            location: model_view_transform_shader_input
                                .translation_and_scaling_location,
                            interpolation: None,
                            sampling: None,
                        }),
                        offset: VECTOR_4_SIZE,
                    },
                ],
                span: 2 * VECTOR_4_SIZE,
            },
        };

        let model_view_transform_type =
            insert_in_arena(&mut module.types, model_view_transform_type);

        let model_view_transform_arg_ptr_expr = generate_input_argument(
            vertex_function,
            new_name("modelViewTransform"),
            model_view_transform_type,
            None,
        );

        let (rotation_quaternion_expr, translation_expr, scaling_expr) =
            emit_in_func(vertex_function, |function| {
                let rotation_quaternion_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: model_view_transform_arg_ptr_expr,
                        index: 0,
                    },
                );
                let translation_and_scaling_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: model_view_transform_arg_ptr_expr,
                        index: 1,
                    },
                );
                let translation_expr =
                    include_expr_in_func(function, swizzle_xyz_expr(translation_and_scaling_expr));
                let scaling_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: translation_and_scaling_expr,
                        index: 3,
                    },
                );
                (rotation_quaternion_expr, translation_expr, scaling_expr)
            });

        ModelViewTransformExpressions {
            rotation_quaternion: rotation_quaternion_expr,
            translation_vector: translation_expr,
            scaling_factor: scaling_expr,
        }
    }

    /// Generates the declaration of the global uniform variable for the camera
    /// projection matrix and returns a new [`CameraProjectionMatrix`]
    /// representing the matrix.
    fn generate_vertex_code_for_projection_matrix(
        camera_shader_input: &CameraShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        bind_group_idx: &mut u32,
    ) -> CameraProjectionExpressions {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let mat4x4_type = insert_in_arena(&mut module.types, MATRIX_4X4_TYPE);

        let projection_matrix_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("projectionMatrix"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: bind_group,
                    binding: camera_shader_input.projection_matrix_binding,
                }),
                ty: mat4x4_type,
                init: None,
            },
        );

        let projection_matrix_ptr_expr = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(projection_matrix_var),
        );

        let matrix_expr = emit_in_func(vertex_function, |function| {
            include_named_expr_in_func(
                function,
                "projectionMatrix",
                Expression::Load {
                    pointer: projection_matrix_ptr_expr,
                },
            )
        });

        CameraProjectionExpressions {
            matrix: matrix_expr,
        }
    }

    /// Generates the arguments for the required mesh vertex attributes in the
    /// main vertex shader function and begins generating the struct of output
    /// to pass from the vertex entry point to the fragment entry point.
    ///
    /// Only vertex attributes required by the material are included as input
    /// arguments.
    ///
    /// The output struct always includes the clip space position, and the
    /// expression computing this by transforming the vertex position with the
    /// model view and projection transformations is generated here. Other
    /// vertex attributes are included in the output struct as required by the
    /// material. If the vertex position or normal vector is required, this is
    /// transformed to camera space before assigned to the output struct.
    ///
    /// # Returns
    /// Because the output struct may have to include material properties, its
    /// code can not be fully generated at this point. Instead, the
    /// [`OutputStructBuilder`] is returned so that the material shader
    /// generator can complete it. The indices of the included vertex attribute
    /// fields are also returned for access in the fragment shader.
    ///
    /// # Errors
    /// Returns an error if not all vertex attributes required by the material
    /// are available in the input mesh.
    fn generate_vertex_code_for_vertex_attributes(
        mesh_shader_input: &MeshShaderInput,
        requirements: VertexAttributeSet,
        module: &mut Module,
        vertex_function: &mut Function,
        model_view_transform: &ModelViewTransformExpressions,
        projection: ProjectionExpressions,
    ) -> Result<(MeshVertexOutputFieldIndices, OutputStructBuilder)> {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }

            fn transformPosition(
                rotationQuaternion: vec4<f32>,
                translation: vec3<f32>,
                scaling: f32,
                position: vec3<f32>
            ) -> vec3<f32> {
                return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let input_model_position_expr =
            Self::add_vertex_attribute_input_argument::<VertexPosition<fre>>(
                vertex_function,
                mesh_shader_input,
                new_name("modelSpacePosition"),
                vec3_type,
            )?;

        let input_color_expr = if requirements.contains(VertexAttributeSet::COLOR) {
            Some(
                Self::add_vertex_attribute_input_argument::<VertexColor<fre>>(
                    vertex_function,
                    mesh_shader_input,
                    new_name("color"),
                    vec3_type,
                )?,
            )
        } else {
            None
        };

        let input_model_normal_vector_expr =
            if requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
                Some(Self::add_vertex_attribute_input_argument::<
                    VertexNormalVector<fre>,
                >(
                    vertex_function,
                    mesh_shader_input,
                    new_name("modelSpaceNormalVector"),
                    vec3_type,
                )?)
            } else {
                None
            };

        let input_texture_coord_expr = if requirements.contains(VertexAttributeSet::TEXTURE_COORDS)
        {
            Some(Self::add_vertex_attribute_input_argument::<
                VertexTextureCoords<fre>,
            >(
                vertex_function,
                mesh_shader_input,
                new_name("textureCoords"),
                vec2_type,
            )?)
        } else {
            None
        };

        let position_expr = SourceCode::generate_call_named(
            vertex_function,
            "cameraSpacePosition",
            source_code.functions["transformPosition"],
            vec![
                model_view_transform.rotation_quaternion,
                model_view_transform.translation_vector,
                model_view_transform.scaling_factor,
                input_model_position_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("VertexOutput");

        let clip_position_expr =
            projection.generate_clip_position_expr(module, vertex_function, position_expr);
        let output_clip_position_field_idx = output_struct_builder.add_builtin_position_field(
            "clipSpacePosition",
            vec4_type,
            VECTOR_4_SIZE,
            clip_position_expr,
        );

        let mut output_field_indices = MeshVertexOutputFieldIndices {
            clip_position: output_clip_position_field_idx,
            position: None,
            color: None,
            normal_vector: None,
            texture_coords: None,
        };

        if requirements.contains(VertexAttributeSet::POSITION) {
            output_field_indices.position = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "position",
                    vec3_type,
                    VECTOR_3_SIZE,
                    position_expr,
                ),
            );
        }

        if let Some(input_color_expr) = input_color_expr {
            output_field_indices.color = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "color",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_color_expr,
                ),
            );
        }

        if let Some(input_model_normal_vector_expr) = input_model_normal_vector_expr {
            let normal_vector_expr = SourceCode::generate_call_named(
                vertex_function,
                "cameraSpaceNormalVector",
                source_code.functions["rotateVectorWithQuaternion"],
                vec![
                    model_view_transform.rotation_quaternion,
                    input_model_normal_vector_expr,
                ],
            );

            output_field_indices.normal_vector = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "normalVector",
                    vec3_type,
                    VECTOR_3_SIZE,
                    normal_vector_expr,
                ),
            );
        }

        if let Some(input_texture_coord_expr) = input_texture_coord_expr {
            output_field_indices.texture_coords = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "textureCoords",
                    vec2_type,
                    VECTOR_2_SIZE,
                    input_texture_coord_expr,
                ),
            );
        }

        Ok((output_field_indices, output_struct_builder))
    }

    fn add_vertex_attribute_input_argument<V>(
        function: &mut Function,
        mesh_shader_input: &MeshShaderInput,
        arg_name: Option<String>,
        type_handle: Handle<Type>,
    ) -> Result<Handle<Expression>>
    where
        V: VertexAttribute,
    {
        if let Some(location) = mesh_shader_input.locations[V::GLOBAL_INDEX] {
            Ok(generate_location_bound_input_argument(
                function,
                arg_name,
                type_handle,
                location,
            ))
        } else {
            Err(anyhow!("Missing required vertex attribute: {}", V::NAME))
        }
    }

    /// Creates a generator of shader code for the light type in the given
    /// shader input.
    fn create_light_shader_generator(
        light_shader_input: &LightShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        vertex_attribute_requirements: &mut VertexAttributeSet,
        has_material: bool,
    ) -> LightShaderGenerator {
        match light_shader_input {
            LightShaderInput::PointLight(light_shader_input) => {
                Self::create_point_light_shader_generator(
                    light_shader_input,
                    module,
                    fragment_function,
                    bind_group_idx,
                    vertex_attribute_requirements,
                    has_material,
                )
            }
            LightShaderInput::UnidirectionalLight(light_shader_input) => {
                Self::create_unidirectional_light_shader_generator(
                    light_shader_input,
                    module,
                    vertex_function,
                    fragment_function,
                    bind_group_idx,
                    has_material,
                )
            }
        }
    }

    /// Creates a generator of shader code for point lights.
    ///
    /// This involves generating declarations for the point light uniform type,
    /// the type the point light uniform buffer will be mapped to, the global
    /// variable this is bound to, the global variables referring to the shadow
    /// map texture and sampler if required, and expressions for the fields of
    /// the light at the active index (which is set in a push constant).
    fn create_point_light_shader_generator(
        light_shader_input: &PointLightShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        vertex_attribute_requirements: &mut VertexAttributeSet,
        has_material: bool,
    ) -> LightShaderGenerator {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        // The struct is padded to 16 byte alignment as required for uniforms
        let single_light_struct_size = 4 * VECTOR_4_SIZE;

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let distance_mapping_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("DistanceMapping"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("nearDistance"),
                            ty: f32_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("inverseDistanceSpan"),
                            ty: f32_type,
                            binding: None,
                            offset: F32_WIDTH,
                        },
                    ],
                    span: VECTOR_4_SIZE,
                },
            },
        );

        let single_light_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("PointLight"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("cameraToLightRotationQuaternion"),
                            ty: vec4_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("cameraSpacePosition"),
                            ty: vec3_type,
                            binding: None,
                            offset: VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("radianceAndEmissionRadius"),
                            ty: vec4_type,
                            binding: None,
                            offset: 2 * VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("distanceMapping"),
                            ty: distance_mapping_struct_type,
                            binding: None,
                            offset: 3 * VECTOR_4_SIZE,
                        },
                    ],
                    span: single_light_struct_size,
                },
            },
        );

        let max_light_count_constant = define_constant_if_missing(
            &mut module.constants,
            u32_constant(light_shader_input.max_light_count),
        );

        let light_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type,
                    size: ArraySize::Constant(max_light_count_constant),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("PointLights"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("numLights"),
                            ty: u32_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("lights"),
                            ty: light_array_type,
                            binding: None,
                            offset: light_count_size,
                        },
                    ],
                    span: single_light_struct_size
                        .checked_mul(u32::try_from(light_shader_input.max_light_count).unwrap())
                        .unwrap()
                        .checked_add(light_count_size)
                        .unwrap(),
                },
            },
        );

        let lights_struct_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("pointLights"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: light_shader_input.uniform_binding,
                }),
                ty: lights_struct_type,
                init: None,
            },
        );

        *bind_group_idx += 1;

        let active_light_idx_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("activeLightIdx"),
                space: AddressSpace::PushConstant,
                binding: None,
                ty: u32_type,
                init: None,
            },
        );

        if has_material {
            // If we have a material, we will do shading that involves the
            // shadow cubemap
            let (
                shadow_map_texture_binding,
                shadow_map_sampler_binding,
                shadow_map_comparison_sampler_binding,
            ) = light_shader_input.shadow_map_texture_and_sampler_binding;

            let shadow_map = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::DepthCubemap,
                "shadowMap",
                *bind_group_idx,
                shadow_map_texture_binding,
                Some(shadow_map_sampler_binding),
                Some(shadow_map_comparison_sampler_binding),
            );

            *bind_group_idx += 1;

            LightShaderGenerator::new_for_point_light_shading(
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
                shadow_map,
            )
        } else {
            // For updating the shadow map, we need access to the unprojected
            // cubemap space position in the fragment shader
            *vertex_attribute_requirements |= VertexAttributeSet::POSITION;

            LightShaderGenerator::new_for_point_light_shadow_map_update(
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
            )
        }
    }

    /// Creates a generator of shader code for point lights.
    ///
    /// This involves generating declarations for the unidirectional light
    /// uniform type, the type the unidirectional light uniform buffer will be
    /// mapped to, the global variable this is bound to, the global variables
    /// referring to the shadow map texture and sampler if required, and
    /// expressions for the fields of the light at the active index (which is
    /// set in a push constant).
    fn create_unidirectional_light_shader_generator(
        light_shader_input: &UnidirectionalLightShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        has_material: bool,
    ) -> LightShaderGenerator {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        // The structs are padded to 16 byte alignment as required for uniforms
        let orthographic_transform_struct_size = 2 * VECTOR_4_SIZE;

        let single_light_struct_size = 3 * VECTOR_4_SIZE
            + MAX_SHADOW_MAP_CASCADES * orthographic_transform_struct_size
            + 4 * F32_WIDTH;

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let orthographic_transform_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("OrthographicTransform"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("translation"),
                            ty: vec3_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("scaling"),
                            ty: vec3_type,
                            binding: None,
                            offset: VECTOR_4_SIZE,
                        },
                    ],
                    span: orthographic_transform_struct_size,
                },
            },
        );

        let max_shadow_map_cascades_constant = define_constant_if_missing(
            &mut module.constants,
            u32_constant(MAX_SHADOW_MAP_CASCADES.into()),
        );

        let orthographic_transform_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: orthographic_transform_struct_type,
                    size: ArraySize::Constant(max_shadow_map_cascades_constant),
                    stride: orthographic_transform_struct_size,
                },
            },
        );

        let single_light_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("UnidirectionalLight"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("cameraToLightRotationQuaternion"),
                            ty: vec4_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("cameraSpaceDirection"),
                            ty: vec3_type,
                            binding: None,
                            offset: VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("radianceAndTanAngularRadius"),
                            ty: vec4_type,
                            binding: None,
                            offset: 2 * VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("orthographicTransforms"),
                            ty: orthographic_transform_array_type,
                            binding: None,
                            offset: 3 * VECTOR_4_SIZE,
                        },
                        // We interpret the array of partition depths as a vec4
                        // rather than an array to satisfy 16-byte padding
                        // requirements. The largest value for
                        // MAX_SHADOW_MAP_CASCADES that we support is thus 5. If
                        // MAX_SHADOW_MAP_CASCADES is smaller than that, the
                        // last element(s) in the vec4 will consist of padding.
                        StructMember {
                            name: new_name("partitionDepths"),
                            ty: vec4_type,
                            binding: None,
                            offset: 3 * VECTOR_4_SIZE
                                + MAX_SHADOW_MAP_CASCADES * orthographic_transform_struct_size,
                        },
                        // <-- The rest of the struct is for padding an not
                        // needed in the shader
                    ],
                    span: single_light_struct_size,
                },
            },
        );

        let max_light_count_constant = define_constant_if_missing(
            &mut module.constants,
            u32_constant(light_shader_input.max_light_count),
        );

        let lights_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type,
                    size: ArraySize::Constant(max_light_count_constant),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("UnidirectionalLights"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("numLights"),
                            ty: u32_type,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("lights"),
                            ty: lights_array_type,
                            binding: None,
                            offset: light_count_size,
                        },
                    ],
                    span: single_light_struct_size
                        .checked_mul(u32::try_from(light_shader_input.max_light_count).unwrap())
                        .unwrap()
                        .checked_add(light_count_size)
                        .unwrap(),
                },
            },
        );

        let lights_struct_var = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("unidirectionalLights"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: light_shader_input.uniform_binding,
                }),
                ty: lights_struct_type,
                init: None,
            },
        );

        *bind_group_idx += 1;

        if has_material {
            let active_light_idx_var = append_to_arena(
                &mut module.global_variables,
                GlobalVariable {
                    name: new_name("activeLightIdx"),
                    space: AddressSpace::PushConstant,
                    binding: None,
                    ty: u32_type,
                    init: None,
                },
            );

            // If we have a material, we will do shading that involves the
            // shadow map
            let (
                shadow_map_texture_binding,
                shadow_map_sampler_binding,
                shadow_map_comparison_sampler_binding,
            ) = light_shader_input.shadow_map_texture_and_sampler_bindings;

            let shadow_map = SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::DepthArray,
                "cascadedShadowMap",
                *bind_group_idx,
                shadow_map_texture_binding,
                Some(shadow_map_sampler_binding),
                Some(shadow_map_comparison_sampler_binding),
            );

            *bind_group_idx += 1;

            LightShaderGenerator::new_for_unidirectional_light_shading(
                vertex_function,
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
                shadow_map,
            )
        } else {
            // For updating the shadow map, we need the index of the cascade to
            // update, which is provided in the same push constant range as the
            // light index
            let active_light_and_cascade_idx_struct_type = insert_in_arena(
                &mut module.types,
                Type {
                    name: new_name("ActiveLightAndCascadeIdx"),
                    inner: TypeInner::Struct {
                        members: vec![
                            StructMember {
                                name: new_name("lightIdx"),
                                ty: u32_type,
                                binding: None,
                                offset: 0,
                            },
                            StructMember {
                                name: new_name("cascadeIdx"),
                                ty: u32_type,
                                binding: None,
                                offset: U32_WIDTH,
                            },
                        ],
                        span: 2 * U32_WIDTH,
                    },
                },
            );

            let active_light_and_cascade_idx_var = append_to_arena(
                &mut module.global_variables,
                GlobalVariable {
                    name: new_name("activeLightAndCascadeIdx"),
                    space: AddressSpace::PushConstant,
                    binding: None,
                    ty: active_light_and_cascade_idx_struct_type,
                    init: None,
                },
            );

            LightShaderGenerator::new_for_unidirectional_light_shadow_map_update(
                vertex_function,
                lights_struct_var,
                active_light_and_cascade_idx_var,
            )
        }
    }
}

impl MaterialShaderInput {
    /// Whether the material requires light sources.
    pub fn requires_lights(&self) -> bool {
        match self {
            Self::GlobalAmbientColor(_) | Self::VertexColor | Self::Fixed(_) => false,
            Self::BlinnPhong(_) => true,
        }
    }
}

impl<'a> MaterialShaderGenerator<'a> {
    /// Generates the vertex shader code specific to the relevant material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// Any per-instance material properties to return from the vertex entry
    /// point are included in an input argument and assigned to dedicated
    /// fields in the [`OutputStructBuilder`].
    ///
    /// # Returns
    /// Any indices of material property fields added to the output struct.
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> MaterialVertexOutputFieldIndices {
        match self {
            Self::FixedColor(builder) => MaterialVertexOutputFieldIndices::FixedColor(
                builder.generate_vertex_code(module, vertex_function, vertex_output_struct_builder),
            ),
            Self::BlinnPhong(builder) => MaterialVertexOutputFieldIndices::BlinnPhong(
                builder.generate_vertex_code(module, vertex_function, vertex_output_struct_builder),
            ),
            _ => MaterialVertexOutputFieldIndices::None,
        }
    }

    /// Generates the fragment shader code specific to the relevant
    /// material by adding code representation to the given [`naga`]
    /// objects.
    ///
    /// The generated code will involve accessing vertex and material
    /// properties in the input struct passed from the vertex entry point,
    /// declaring and sampling any required textures and creating and
    /// returning an output struct with the computed fragment color.
    ///
    /// # Panics
    /// If `material_input_field_indices` does not represent the same
    /// material as this enum.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &MaterialVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        match (self, material_input_field_indices) {
            (Self::GlobalAmbientColor(generator), MaterialVertexOutputFieldIndices::None) => {
                generator.generate_fragment_code(module, fragment_function, bind_group_idx);
            }
            (Self::VertexColor, MaterialVertexOutputFieldIndices::None) => {
                VertexColorShaderGenerator::generate_fragment_code(
                    module,
                    fragment_function,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (
                Self::FixedColor(_),
                MaterialVertexOutputFieldIndices::FixedColor(color_input_field_idx),
            ) => FixedColorShaderGenerator::generate_fragment_code(
                module,
                fragment_function,
                fragment_input_struct,
                color_input_field_idx,
            ),
            (Self::FixedTexture(generator), MaterialVertexOutputFieldIndices::None) => generator
                .generate_fragment_code(
                    module,
                    fragment_function,
                    bind_group_idx,
                    fragment_input_struct,
                    mesh_input_field_indices,
                ),
            (
                Self::BlinnPhong(generator),
                MaterialVertexOutputFieldIndices::BlinnPhong(material_input_field_indices),
            ) => generator.generate_fragment_code(
                module,
                fragment_function,
                bind_group_idx,
                fragment_input_struct,
                mesh_input_field_indices,
                light_input_field_indices,
                material_input_field_indices,
                light_shader_generator,
            ),
            _ => panic!("Mismatched material shader builder and output field indices type"),
        }
    }
}

impl ProjectionExpressions {
    /// Generates an expression for the given position (as a vec3) projected
    /// with the projection in the vertex entry point function. The projected
    /// position will be a vec4.
    pub fn generate_clip_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        match self {
            Self::Camera(camera_projection_matrix) => camera_projection_matrix
                .generate_clip_position_expr(module, vertex_function, position_expr),
            Self::PointLight(point_light_cubemap_projection) => point_light_cubemap_projection
                .generate_clip_position_expr(module, vertex_function, position_expr),
            Self::UnidirectionalLight(unidirectional_light_orthographic_projection) => {
                unidirectional_light_orthographic_projection.generate_clip_position_expr(
                    module,
                    vertex_function,
                    position_expr,
                )
            }
        }
    }
}

impl CameraProjectionExpressions {
    /// Generates an expression for the given position (as a vec3) projected
    /// with the projection matrix in the vertex entry point function. The
    /// projected position will be a vec4.
    pub fn generate_clip_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let homogeneous_position_expr = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            vertex_function,
            position_expr,
        );

        emit_in_func(vertex_function, |function| {
            include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: self.matrix,
                    right: homogeneous_position_expr,
                },
            )
        })
    }
}

impl PointLightProjectionExpressions {
    #[allow(clippy::unused_self)]
    pub fn generate_clip_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn applyCubemapFaceProjection(
                position: vec3<f32>,
            ) -> vec4<f32> {
                // It is important not to perform perspective division manually
                // here, because the homogeneous vector should be interpolated
                // first.

                return vec4<f32>(
                    position.xy,
                    // This component does not matter, as we compute the proper
                    // depth in the fragment shader
                    position.z,
                    position.z,
                );
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        SourceCode::generate_call_named(
            vertex_function,
            "lightClipSpacePosition",
            source_code.functions["applyCubemapFaceProjection"],
            vec![position_expr],
        )
    }
}

impl UnidirectionalLightProjectionExpressions {
    /// Generates an expression for the given position (as a vec3) projected
    /// with the orthographic projection in the vertex entry point function. The
    /// projected position will be a vec4 with w = 1.0;
    pub fn generate_clip_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn applyOrthographicProjectionToPosition(
                orthographicTranslation: vec3<f32>,
                orthographicScaling: vec3<f32>,
                position: vec3<f32>
            ) -> vec3<f32> {
                return (position + orthographicTranslation) * orthographicScaling;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let light_clip_space_position_expr = SourceCode::generate_call_named(
            vertex_function,
            "lightClipSpacePosition",
            source_code.functions["applyOrthographicProjectionToPosition"],
            vec![self.translation, self.scaling, position_expr],
        );

        append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            vertex_function,
            light_clip_space_position_expr,
        )
    }
}

impl LightShaderGenerator {
    pub fn new_for_point_light_shadow_map_update(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
    ) -> Self {
        Self::PointLight(PointLightShaderGenerator::ForShadowMapUpdate(
            PointLightShadowMapUpdateShaderGenerator::new(
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
            ),
        ))
    }

    pub fn new_for_point_light_shading(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
        shadow_map: SampledTexture,
    ) -> Self {
        Self::PointLight(PointLightShaderGenerator::ForShading(
            PointLightShadingShaderGenerator::new(
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
                shadow_map,
            ),
        ))
    }

    pub fn new_for_unidirectional_light_shadow_map_update(
        vertex_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_and_cascade_idx_var: Handle<GlobalVariable>,
    ) -> Self {
        Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShadowMapUpdate(
            UnidirectionalLightShadowMapUpdateShaderGenerator::new(
                vertex_function,
                lights_struct_var,
                active_light_and_cascade_idx_var,
            ),
        ))
    }

    pub fn new_for_unidirectional_light_shading(
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
        shadow_map: SampledTexture,
    ) -> Self {
        Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShading(
            UnidirectionalLightShadingShaderGenerator::new(
                vertex_function,
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
                shadow_map,
            ),
        ))
    }

    pub fn get_projection_to_light_clip_space(&self) -> Option<ProjectionExpressions> {
        match self {
            Self::PointLight(_) => Some(ProjectionExpressions::PointLight(
                PointLightProjectionExpressions,
            )),
            Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShadowMapUpdate(
                shader_generator,
            )) => Some(shader_generator.get_projection_to_light_clip_space()),
            Self::UnidirectionalLight(_) => None,
        }
    }

    pub fn generate_vertex_output_code_for_shading(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        output_struct_builder: &mut OutputStructBuilder,
        mesh_output_field_indices: &MeshVertexOutputFieldIndices,
    ) -> Option<LightVertexOutputFieldIndices> {
        match self {
            Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShading(
                shader_generator,
            )) => Some(LightVertexOutputFieldIndices::UnidirectionalLight(
                shader_generator.generate_vertex_output_code_for_shading(
                    module,
                    vertex_function,
                    output_struct_builder,
                    mesh_output_field_indices,
                ),
            )),
            _ => None,
        }
    }

    pub fn has_fragment_output(&self) -> bool {
        matches!(
            self,
            Self::PointLight(PointLightShaderGenerator::ForShadowMapUpdate(_))
        )
    }

    pub fn generate_fragment_output_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        if let Self::PointLight(PointLightShaderGenerator::ForShadowMapUpdate(
            shadow_map_update_shader_generator,
        )) = self
        {
            shadow_map_update_shader_generator.generate_fragment_output_code(
                module,
                fragment_function,
                fragment_input_struct,
                mesh_input_field_indices,
            );
        }
    }

    fn generate_active_light_ptr_expr(
        function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
    ) -> Handle<Expression> {
        let lights_struct_ptr_expr =
            include_expr_in_func(function, Expression::GlobalVariable(lights_struct_var));

        let active_light_idx_expr =
            Self::generate_active_light_idx_expr(function, active_light_idx_var);

        Self::generate_single_light_ptr_expr(
            function,
            lights_struct_ptr_expr,
            active_light_idx_expr,
        )
    }

    fn generate_active_light_idx_expr(
        function: &mut Function,
        active_light_idx_var: Handle<GlobalVariable>,
    ) -> Handle<Expression> {
        let active_light_idx_ptr_expr =
            include_expr_in_func(function, Expression::GlobalVariable(active_light_idx_var));

        let active_light_idx_expr = emit_in_func(function, |function| {
            include_named_expr_in_func(
                function,
                "activeLightIdx",
                Expression::Load {
                    pointer: active_light_idx_ptr_expr,
                },
            )
        });

        active_light_idx_expr
    }

    fn generate_single_light_ptr_expr(
        function: &mut Function,
        lights_struct_ptr_expr: Handle<Expression>,
        light_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let lights_field_ptr =
            Self::generate_field_access_ptr_expr(function, lights_struct_ptr_expr, 1);

        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::Access {
                    base: lights_field_ptr,
                    index: light_idx_expr,
                },
            )
        })
    }

    fn generate_named_field_access_expr(
        function: &mut Function,
        name: impl ToString,
        struct_ptr_expr: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        let field_ptr = Self::generate_field_access_ptr_expr(function, struct_ptr_expr, field_idx);
        emit_in_func(function, |function| {
            include_named_expr_in_func(function, name, Expression::Load { pointer: field_ptr })
        })
    }

    fn generate_field_access_ptr_expr(
        function: &mut Function,
        struct_ptr_expr: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: struct_ptr_expr,
                    index: field_idx,
                },
            )
        })
    }
}

impl PointLightShadowMapUpdateShaderGenerator {
    pub fn new(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
    ) -> Self {
        let active_light_ptr_expr = LightShaderGenerator::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var,
            active_light_idx_var,
        );

        let distance_mapping = LightShaderGenerator::generate_field_access_ptr_expr(
            fragment_function,
            active_light_ptr_expr,
            3,
        );

        let near_distance = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightNearDistance",
            distance_mapping,
            0,
        );

        let inverse_distance_span = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightInverseDistanceSpan",
            distance_mapping,
            1,
        );

        Self {
            near_distance,
            inverse_distance_span,
        }
    }

    pub fn generate_fragment_output_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn computeShadowMapFragmentDepth(
                nearDistance: f32,
                inverseDistanceSpan: f32,
                cubemapSpaceFragmentPosition: vec3<f32>,
            ) -> f32 {
                // Compute distance between fragment and light and scale to [0, 1] range
                return (length(cubemapSpaceFragmentPosition) - nearDistance) * inverseDistanceSpan;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for point light shadow map update"),
        );

        let depth = SourceCode::generate_call_named(
            fragment_function,
            "fragmentDepth",
            source_code.functions["computeShadowMapFragmentDepth"],
            vec![
                self.near_distance,
                self.inverse_distance_span,
                position_expr,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_builtin_fragment_depth_field(
            "fragmentDepth",
            f32_type,
            F32_WIDTH,
            depth,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }
}

impl PointLightShadingShaderGenerator {
    pub fn new(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
        shadow_map: SampledTexture,
    ) -> Self {
        let active_light_ptr_expr = LightShaderGenerator::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var,
            active_light_idx_var,
        );

        let camera_to_light_space_rotation_quaternion =
            LightShaderGenerator::generate_named_field_access_expr(
                fragment_function,
                "cameraToLightSpaceRotationQuaternion",
                active_light_ptr_expr,
                0,
            );

        let camera_space_position = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "cameraSpaceLightPosition",
            active_light_ptr_expr,
            1,
        );

        let radiance_and_emission_radius = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightRadianceAndEmissionRadius",
            active_light_ptr_expr,
            2,
        );

        let (radiance, emission_radius) = emit_in_func(fragment_function, |function| {
            (
                include_expr_in_func(function, swizzle_xyz_expr(radiance_and_emission_radius)),
                include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: radiance_and_emission_radius,
                        index: 3,
                    },
                ),
            )
        });

        let distance_mapping = LightShaderGenerator::generate_field_access_ptr_expr(
            fragment_function,
            active_light_ptr_expr,
            3,
        );

        let near_distance = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightNearDistance",
            distance_mapping,
            0,
        );

        let inverse_distance_span = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightInverseDistanceSpan",
            distance_mapping,
            1,
        );

        Self {
            camera_to_light_space_rotation_quaternion,
            camera_space_position,
            radiance,
            emission_radius,
            near_distance,
            inverse_distance_span,
            shadow_map,
        }
    }

    pub fn generate_fragment_shading_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        camera_clip_position_expr: Handle<Expression>,
        position_expr: Handle<Expression>,
        normal_vector_expr: Handle<Expression>,
    ) -> (Handle<Expression>, Handle<Expression>) {
        let source_code = SourceCode::from_wgsl_source(
            "\
            struct LightQuantities {
                lightDirection: vec3<f32>,
                lightDirectionDotNormalVector: f32,
                attenuatedLightRadiance: vec3<f32>,
                lightSpaceFragmentDisplacement: vec3<f32>,
                normalizedDistance: f32,
            }

            fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }

            fn computeLightQuantities(
                lightPosition: vec3<f32>,
                lightRadiance: vec3<f32>,
                cameraToLightSpaceRotationQuaternion: vec4<f32>,
                nearDistance: f32,
                inverseDistanceSpan: f32,
                fragmentPosition: vec3<f32>,
                fragmentNormal: vec3<f32>,
            ) -> LightQuantities {
                var output: LightQuantities;

                let lightDisplacement = lightPosition - fragmentPosition;
                let inverseSquaredDistance = 1.0 / dot(lightDisplacement, lightDisplacement);
                output.lightDirection = lightDisplacement * sqrt(inverseSquaredDistance);
                output.lightDirectionDotNormalVector = dot(output.lightDirection, fragmentNormal);
                
                output.attenuatedLightRadiance = lightRadiance * inverseSquaredDistance;

                // Add an offset to the fragment position along the fragment
                // normal to avoid shadow acne. The offset increases as the
                // light becomes less perpendicular to the surface.
                let offsetFragmentDisplacement = -lightDisplacement + fragmentNormal * clamp(1.0 - output.lightDirectionDotNormalVector, 2e-2, 1.0) * 5e-3 / inverseDistanceSpan;

                output.lightSpaceFragmentDisplacement = rotateVectorWithQuaternion(cameraToLightSpaceRotationQuaternion, offsetFragmentDisplacement);
                output.normalizedDistance = (length(output.lightSpaceFragmentDisplacement) - nearDistance) * inverseDistanceSpan;

                return output;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let light_quantities = SourceCode::generate_call_named(
            fragment_function,
            "lightQuantities",
            source_code.functions["computeLightQuantities"],
            vec![
                self.camera_space_position,
                self.radiance,
                self.camera_to_light_space_rotation_quaternion,
                self.near_distance,
                self.inverse_distance_span,
                position_expr,
                normal_vector_expr,
            ],
        );

        let (light_space_fragment_displacement_expr, depth_reference_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 3,
                        },
                    ),
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 4,
                        },
                    ),
                )
            });

        let light_access_factor_expr = self
            .shadow_map
            .generate_light_access_factor_expr_for_shadow_cubemap(
                module,
                fragment_function,
                self.emission_radius,
                camera_clip_position_expr,
                light_space_fragment_displacement_expr,
                depth_reference_expr,
            );

        emit_in_func(fragment_function, |function| {
            let light_direction_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_quantities,
                    index: 0,
                },
            );

            let attenuated_radiance_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_quantities,
                    index: 2,
                },
            );

            let shadow_masked_attenuated_radiance_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: light_access_factor_expr,
                    right: attenuated_radiance_expr,
                },
            );

            (light_direction_expr, shadow_masked_attenuated_radiance_expr)
        })
    }
}

impl UnidirectionalLightShaderGenerator {
    fn generate_single_orthographic_transform_ptr_expr(
        function: &mut Function,
        active_light_ptr_expr: Handle<Expression>,
        cascade_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let orthographic_transforms_field_ptr =
            LightShaderGenerator::generate_field_access_ptr_expr(
                function,
                active_light_ptr_expr,
                3,
            );

        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::Access {
                    base: orthographic_transforms_field_ptr,
                    index: cascade_idx_expr,
                },
            )
        })
    }
}

impl UnidirectionalLightShadowMapUpdateShaderGenerator {
    pub fn new(
        vertex_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_and_cascade_idx_var: Handle<GlobalVariable>,
    ) -> Self {
        let active_light_and_cascade_idx_ptr_expr = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(active_light_and_cascade_idx_var),
        );

        let active_light_idx_expr = LightShaderGenerator::generate_named_field_access_expr(
            vertex_function,
            "lightIdx",
            active_light_and_cascade_idx_ptr_expr,
            0,
        );

        let active_cascade_idx_expr = LightShaderGenerator::generate_named_field_access_expr(
            vertex_function,
            "cascadeIdx",
            active_light_and_cascade_idx_ptr_expr,
            1,
        );

        let lights_struct_ptr_expr = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(lights_struct_var),
        );

        let active_light_ptr_expr = LightShaderGenerator::generate_single_light_ptr_expr(
            vertex_function,
            lights_struct_ptr_expr,
            active_light_idx_expr,
        );

        let orthographic_transform_ptr_expr =
            UnidirectionalLightShaderGenerator::generate_single_orthographic_transform_ptr_expr(
                vertex_function,
                active_light_ptr_expr,
                active_cascade_idx_expr,
            );

        let orthographic_translation = LightShaderGenerator::generate_named_field_access_expr(
            vertex_function,
            "lightOrthographicTranslation",
            orthographic_transform_ptr_expr,
            0,
        );

        let orthographic_scaling = LightShaderGenerator::generate_named_field_access_expr(
            vertex_function,
            "lightOrthographicScaling",
            orthographic_transform_ptr_expr,
            1,
        );

        let orthographic_projection = UnidirectionalLightProjectionExpressions {
            translation: orthographic_translation,
            scaling: orthographic_scaling,
        };

        Self {
            orthographic_projection,
        }
    }

    pub fn get_projection_to_light_clip_space(&self) -> ProjectionExpressions {
        ProjectionExpressions::UnidirectionalLight(self.orthographic_projection.clone())
    }
}

impl UnidirectionalLightShadingShaderGenerator {
    pub fn new(
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        active_light_idx_var: Handle<GlobalVariable>,
        shadow_map: SampledTexture,
    ) -> Self {
        let active_light_ptr_expr_in_vertex_function =
            LightShaderGenerator::generate_active_light_ptr_expr(
                vertex_function,
                lights_struct_var,
                active_light_idx_var,
            );

        let active_light_ptr_expr_in_fragment_function =
            LightShaderGenerator::generate_active_light_ptr_expr(
                fragment_function,
                lights_struct_var,
                active_light_idx_var,
            );

        Self {
            active_light_ptr_expr_in_vertex_function,
            active_light_ptr_expr_in_fragment_function,
            shadow_map,
        }
    }

    pub fn generate_vertex_output_code_for_shading(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        output_struct_builder: &mut OutputStructBuilder,
        mesh_output_field_indices: &MeshVertexOutputFieldIndices,
    ) -> UnidirectionalLightVertexOutputFieldIndices {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let camera_space_position_expr = output_struct_builder
            .get_field_expr(
                mesh_output_field_indices
                    .position
                    .expect("Missing position for shading with unidirectional light"),
            )
            .unwrap();

        let camera_space_normal_vector_expr = output_struct_builder
            .get_field_expr(
                mesh_output_field_indices
                    .normal_vector
                    .expect("Missing normal vector for shading with unidirectional light"),
            )
            .unwrap();

        let camera_to_light_space_rotation_quaternion_expr =
            LightShaderGenerator::generate_named_field_access_expr(
                vertex_function,
                "cameraToLightSpaceRotationQuaternion",
                self.active_light_ptr_expr_in_vertex_function,
                0,
            );

        let light_space_position_expr = SourceCode::generate_call_named(
            vertex_function,
            "lightSpacePosition",
            source_code.functions["rotateVectorWithQuaternion"],
            vec![
                camera_to_light_space_rotation_quaternion_expr,
                camera_space_position_expr,
            ],
        );

        let light_space_normal_vector_expr = SourceCode::generate_call_named(
            vertex_function,
            "lightSpaceNormalVector",
            source_code.functions["rotateVectorWithQuaternion"],
            vec![
                camera_to_light_space_rotation_quaternion_expr,
                camera_space_normal_vector_expr,
            ],
        );

        UnidirectionalLightVertexOutputFieldIndices {
            light_space_position: output_struct_builder.add_field_with_perspective_interpolation(
                "lightSpacePosition",
                vec3_type,
                VECTOR_3_SIZE,
                light_space_position_expr,
            ),
            light_space_normal_vector: output_struct_builder
                .add_field_with_perspective_interpolation(
                    "lightSpaceNormalVector",
                    vec3_type,
                    VECTOR_3_SIZE,
                    light_space_normal_vector_expr,
                ),
        }
    }

    pub fn generate_fragment_shading_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        camera_clip_position_expr: Handle<Expression>,
        light_space_position_expr: Handle<Expression>,
        light_space_normal_vector_expr: Handle<Expression>,
    ) -> (Handle<Expression>, Handle<Expression>) {
        let determine_cascade_idx_body = match MAX_SHADOW_MAP_CASCADES {
            1 => "cascadeIdx = 0;",
            2 => {
                "if depth < partitionDepths.x {
                     cascadeIdx = 0;
                 } else {
                     cascadeIdx = 1;
                 }"
            }
            3 => {
                "if depth < partitionDepths.x {
                     cascadeIdx = 0;
                 } else if depth < partitionDepths.y {
                     cascadeIdx = 1;
                 } else {
                     cascadeIdx = 2;
                 }"
            }
            4 => {
                "if depth < partitionDepths.x {
                     cascadeIdx = 0;
                 } else if depth < partitionDepths.y {
                     cascadeIdx = 1;
                 } else if depth < partitionDepths.z {
                     cascadeIdx = 2;
                 } else {
                     cascadeIdx = 3;
                 }"
            }
            5 => {
                "if depth < partitionDepths.x {
                     cascadeIdx = 0;
                 } else if depth < partitionDepths.y {
                     cascadeIdx = 1;
                 } else if depth < partitionDepths.z {
                     cascadeIdx = 2;
                 } else if depth < partitionDepths.w {
                     cascadeIdx = 3;
                 } else {
                     cascadeIdx = 4;
                 }"
            }
            _ => panic!("MAX_SHADOW_MAP_CASCADES outside of supported range [1, 5]"),
        };

        let source_code = SourceCode::from_wgsl_source(
            &format!("\
            fn determineCascadeIdx(partitionDepths: vec4<f32>, cameraClipSpacePosition: vec4<f32>) -> i32 {{
                var cascadeIdx: i32;
                let depth = cameraClipSpacePosition.z;
                {}
                return cascadeIdx;
            }}

            fn applyNormalBias(
                lightSpacePosition: vec3<f32>,
                lightSpaceNormalVector: vec3<f32>
            ) -> vec3<f32> {{
                let lightDirectionDotNormalVector = -lightSpaceNormalVector.z;
                return lightSpacePosition + lightSpaceNormalVector * clamp(1.0 - lightDirectionDotNormalVector, 0.0, 1.0) * 1e-1;
            }}

            fn applyOrthographicProjectionToPosition(
                orthographicTranslation: vec3<f32>,
                orthographicScaling: vec3<f32>,
                position: vec3<f32>
            ) -> vec3<f32> {{
                return (position + orthographicTranslation) * orthographicScaling;
            }}

            fn computeLightClipSpacePosition(
                orthographicTranslation: vec3<f32>,
                orthographicScaling: vec3<f32>,
                lightSpacePosition: vec3<f32>,
                lightSpaceNormalVector: vec3<f32>,
            ) -> vec3<f32> {{
                let biasedLightSpacePosition = applyNormalBias(lightSpacePosition, lightSpaceNormalVector);
                return applyOrthographicProjectionToPosition(orthographicTranslation, orthographicScaling, biasedLightSpacePosition);
            }}
        ", determine_cascade_idx_body),
        )
        .unwrap()
        .import_to_module(module);

        let camera_space_direction_expr = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "cameraSpaceLightDirection",
            self.active_light_ptr_expr_in_fragment_function,
            1,
        );

        let radiance_and_tan_angular_radius_expr =
            LightShaderGenerator::generate_named_field_access_expr(
                fragment_function,
                "lightRadianceAndTanAngularRadius",
                self.active_light_ptr_expr_in_fragment_function,
                2,
            );

        let (radiance_expr, tan_angular_radius_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        swizzle_xyz_expr(radiance_and_tan_angular_radius_expr),
                    ),
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: radiance_and_tan_angular_radius_expr,
                            index: 3,
                        },
                    ),
                )
            });

        let partition_depths_expr = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "partitionDepths",
            self.active_light_ptr_expr_in_fragment_function,
            4,
        );

        let cascade_idx_expr = SourceCode::generate_call_named(
            fragment_function,
            "cascadeIdx",
            source_code.functions["determineCascadeIdx"],
            vec![partition_depths_expr, camera_clip_position_expr],
        );

        let orthographic_transform_ptr_expr =
            UnidirectionalLightShaderGenerator::generate_single_orthographic_transform_ptr_expr(
                fragment_function,
                self.active_light_ptr_expr_in_fragment_function,
                cascade_idx_expr,
            );

        let orthographic_translation_expr = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightOrthographicTranslation",
            orthographic_transform_ptr_expr,
            0,
        );

        let orthographic_scaling_expr = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightOrthographicScaling",
            orthographic_transform_ptr_expr,
            1,
        );

        let (world_to_light_clip_space_xy_scale_expr, world_to_light_clip_space_z_scale_expr) =
            emit_in_func(fragment_function, |function| {
                let world_to_light_clip_space_xy_scale_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: orthographic_scaling_expr,
                        index: 0,
                    },
                );

                let orthographic_scale_z_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: orthographic_scaling_expr,
                        index: 2,
                    },
                );

                let world_to_light_clip_space_z_scale_expr = include_expr_in_func(
                    function,
                    Expression::Unary {
                        op: UnaryOperator::Negate,
                        expr: orthographic_scale_z_expr,
                    },
                );

                (
                    world_to_light_clip_space_xy_scale_expr,
                    world_to_light_clip_space_z_scale_expr,
                )
            });

        let light_clip_position_expr = SourceCode::generate_call_named(
            fragment_function,
            "lightClipSpacePosition",
            source_code.functions["computeLightClipSpacePosition"],
            vec![
                orthographic_translation_expr,
                orthographic_scaling_expr,
                light_space_position_expr,
                light_space_normal_vector_expr,
            ],
        );

        let light_access_factor_expr = self
            .shadow_map
            .generate_light_access_factor_expr_for_cascaded_shadow_map(
                module,
                fragment_function,
                tan_angular_radius_expr,
                world_to_light_clip_space_xy_scale_expr,
                world_to_light_clip_space_z_scale_expr,
                camera_clip_position_expr,
                light_clip_position_expr,
                cascade_idx_expr,
            );

        let (light_direction_expr, attenuated_radiance_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        Expression::Unary {
                            op: UnaryOperator::Negate,
                            expr: camera_space_direction_expr,
                        },
                    ),
                    include_expr_in_func(
                        function,
                        Expression::Binary {
                            op: BinaryOperator::Multiply,
                            left: light_access_factor_expr,
                            right: radiance_expr,
                        },
                    ),
                )
            });

        (light_direction_expr, attenuated_radiance_expr)
    }
}

impl InputStruct {
    /// Returns the handle to the expression for the struct
    /// field with the given index.
    ///
    /// # Panics
    /// If the index is out of bounds.
    pub fn get_field_expr(&self, idx: usize) -> Handle<Expression> {
        self.input_field_expressions[idx]
    }
}

impl InputStructBuilder {
    /// Creates a builder for an input struct with the given
    /// type name and name to use when including the struct
    /// as an input argument.
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
    /// This method is intended for constructing an input struct
    /// to the vertex entry point. Thus, the field requires a
    /// location binding.
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
                interpolation: None,
                sampling: None,
            }),
            size,
        )
    }

    /// Generates code declaring the struct type and adds the
    /// struct as an input argument to the given [`Function`].
    ///
    /// # Returns
    /// An [`InputStruct`] holding the expression for accessing
    /// each field in the body of the function.
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
                .into_iter()
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
    /// Creates a builder for an output struct with the given
    /// type name.
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
    /// The field is given an automatically incremented location
    /// binding.
    ///
    /// The given input expression handle specifies the expression
    /// whose value should be assigned to the field when
    /// [`generate_output_code`] is called.
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
                interpolation,
                sampling,
            }),
            size,
        );

        self.location += 1;

        idx
    }

    /// Adds a new struct field that will use perspective-correct
    /// interpolation and center sampling when passed to the fragment
    /// entry point.
    ///
    /// The field is given an automatically incremented location
    /// binding.
    ///
    /// The given input expression handle specifies the expression
    /// whose value should be assigned to the field when
    /// [`generate_output_code`] is called.
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

    /// Adds a new struct field with the built-in position binding
    /// rather than a location binding.
    ///
    /// The field is given an automatically incremented location
    /// binding.
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

    /// Generates code declaring the struct type and adds the
    /// struct as the return type of the given [`Function`].
    /// Also initializes the struct in the body of the function
    /// and generates statements assigning a value to each field
    /// using the expression provided when the field was added,
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

    /// Generates code declaring the struct type (only if not
    /// already declared) and adds the struct as an input argument
    /// to the given [`Function`].
    ///
    /// # Returns
    /// An [`InputStruct`] holding the expression for accessing
    /// each field in the body of the function.
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
    /// Creates a new builder for a struct with the given type
    /// name.
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
            TextureType::Image => IMAGE_TEXTURE_TYPE,
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

    /// Generates and returns an expression sampling the texture at the given
    /// texture coordinates. If sampling a depth texture, a reference depth must
    /// also be provided for the comparison sampling. If an array index is
    /// provided, it will be used to select the texture to sample from the
    /// texture array. If `gather` is not [`None`], the specified component of
    /// the texture will be sampled in the 2x2 grid of texels surrounding the
    /// texture coordinates, and the returned expression is a vec4 containing
    /// the samples.
    pub fn generate_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
        array_index_expr: Option<Handle<Expression>>,
        depth_reference_expr: Option<Handle<Expression>>,
        gather: Option<SwizzleComponent>,
    ) -> Handle<Expression> {
        let texture_var_expr =
            include_expr_in_func(function, Expression::GlobalVariable(self.texture_var));

        let sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(if depth_reference_expr.is_some() {
                self.comparison_sampler_var
                    .expect("Missing comparison sampler for sampling with depth reference")
            } else {
                self.sampler_var
                    .expect("Missing sampler for sampling without depth reference")
            }),
        );

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
                        SampleLevel::Auto
                    } else {
                        SampleLevel::Zero
                    },
                    depth_ref: depth_reference_expr,
                },
            )
        });

        sampling_expr
    }

    /// Generates and returns an expression sampling the texture at
    /// the texture coordinates specified by the given expression,
    /// and extracting the RGB values of the sampled RGBA color.
    pub fn generate_rgb_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, None, None, None);

        emit_in_func(function, |function| {
            include_expr_in_func(function, swizzle_xyz_expr(sampling_expr))
        })
    }

    /// Generates and returns an expression for the fraction of light reaching
    /// the fragment based on sampling of the specified shadow map cascade
    /// around the texture coordinates converted from the x- and y-component of
    /// the given light clip space position, using the z-component as the
    /// reference depth.
    pub fn generate_light_access_factor_expr_for_shadow_cubemap(
        &self,
        module: &mut Module,
        function: &mut Function,
        emission_radius_expr: Handle<Expression>,
        camera_clip_position_expr: Handle<Expression>,
        light_space_fragment_displacement_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        self.generate_pcss_light_access_factor_expr_for_shadow_cubemap(
            module,
            function,
            emission_radius_expr,
            camera_clip_position_expr,
            light_space_fragment_displacement_expr,
            depth_reference_expr,
        )
    }

    /// Generates and returns an expression for the fraction of light reaching
    /// the fragment based on sampling of the specified shadow map cascade
    /// around the texture coordinates converted from the x- and y-component of
    /// the given light clip space position, using the z-component as the
    /// reference depth.
    pub fn generate_light_access_factor_expr_for_cascaded_shadow_map(
        &self,
        module: &mut Module,
        function: &mut Function,
        tan_angular_radius_expr: Handle<Expression>,
        world_to_light_clip_space_xy_scale_expr: Handle<Expression>,
        world_to_light_clip_space_z_scale_expr: Handle<Expression>,
        camera_clip_position_expr: Handle<Expression>,
        light_clip_position_expr: Handle<Expression>,
        cascade_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);

        let unity_constant_expr = include_expr_in_func(
            function,
            Expression::Constant(define_constant_if_missing(
                &mut module.constants,
                float32_constant(1.0),
            )),
        );

        let half_constant_expr = include_expr_in_func(
            function,
            Expression::Constant(define_constant_if_missing(
                &mut module.constants,
                float32_constant(0.5),
            )),
        );

        let (texture_coord_expr, depth_reference_expr) = emit_in_func(function, |function| {
            // Map x [-1, 1] to u [0, 1]

            let light_clip_position_x_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_clip_position_expr,
                    index: 0,
                },
            );

            let offset_light_clip_position_x_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Add,
                    left: light_clip_position_x_expr,
                    right: unity_constant_expr,
                },
            );

            let u_texture_coord_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: offset_light_clip_position_x_expr,
                    right: half_constant_expr,
                },
            );

            // Map y [-1, 1] to v [1, 0]

            let light_clip_position_y_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_clip_position_expr,
                    index: 1,
                },
            );

            let negated_light_clip_position_y_expr = include_expr_in_func(
                function,
                Expression::Unary {
                    op: UnaryOperator::Negate,
                    expr: light_clip_position_y_expr,
                },
            );

            let offset_negated_light_clip_position_y_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Add,
                    left: negated_light_clip_position_y_expr,
                    right: unity_constant_expr,
                },
            );

            let v_texture_coord_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: offset_negated_light_clip_position_y_expr,
                    right: half_constant_expr,
                },
            );

            let texture_coords_expr = include_expr_in_func(
                function,
                Expression::Compose {
                    ty: vec2_type,
                    components: vec![u_texture_coord_expr, v_texture_coord_expr],
                },
            );

            let depth_reference_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_clip_position_expr,
                    index: 2,
                },
            );

            (texture_coords_expr, depth_reference_expr)
        });

        self.generate_pcss_light_access_factor_expr_for_cascaded_shadow_map(
            module,
            function,
            tan_angular_radius_expr,
            world_to_light_clip_space_xy_scale_expr,
            world_to_light_clip_space_z_scale_expr,
            camera_clip_position_expr,
            texture_coord_expr,
            depth_reference_expr,
            cascade_idx_expr,
        )
    }

    fn generate_pcss_light_access_factor_expr_for_shadow_cubemap(
        &self,
        module: &mut Module,
        function: &mut Function,
        emission_radius_expr: Handle<Expression>,
        camera_clip_position_expr: Handle<Expression>,
        light_space_fragment_displacement_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let source_code = SourceCode::from_wgsl_source(
            "\
            // Returns a random number between 0 and 1 based on the pixel coordinates
            fn generateInterleavedGradientNoiseFactor(cameraClipSpacePosition: vec4<f32>) -> f32 {
                let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
                return fract(magic.z * fract(dot(magic.xy, cameraClipSpacePosition.xy)));
            }

            fn generateVogelDiskSampleCoords(baseAngle: f32, inverseSqrtSampleCount: f32, sampleIdx: u32) -> vec2<f32> {
                let goldenAngle: f32 = 2.4;
                let radius = sqrt(f32(sampleIdx) + 0.5) * inverseSqrtSampleCount;
                let angle = baseAngle + goldenAngle * f32(sampleIdx);
                return vec2<f32>(radius * cos(angle), radius * sin(angle));
            }

            fn computeVogelDiskBaseAngle(cameraClipSpacePosition: vec4<f32>) -> f32 {
                // Multiply with 2 * pi to get random angle
                return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraClipSpacePosition);
            }

            fn findPerpendicularVector(vector: vec3<f32>) -> vec3<f32> {
                let shifted_signs = sign(vector) + 0.5;
                let sign_xz = sign(shifted_signs.x * shifted_signs.z);
                let sign_yz = sign(shifted_signs.y * shifted_signs.z);
                return vec3<f32>(sign_xz * vector.z, sign_yz * vector.z, -sign_xz * vector.x - sign_yz * vector.y);
            }

            fn generateSampleDisplacement(
                displacement: vec3<f32>,
                displacementNormalDirection: vec3<f32>,
                displacementBinormalDirection: vec3<f32>,
                sampleOnPerpendicularDisk: vec2<f32>,
            ) -> vec3<f32> {
                return displacement + sampleOnPerpendicularDisk.x * displacementNormalDirection + sampleOnPerpendicularDisk.y * displacementBinormalDirection;
            }

            fn computeShadowPenumbraExtent(
                shadowMapTexture: texture_depth_cube,
                pointSampler: sampler,
                emissionRadius: f32,
                vogelDiskBaseAngle: f32,
                displacement: vec3<f32>,
                displacementNormalDirection: vec3<f32>,
                displacementBinormalDirection: vec3<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let sampleDiskRadius: f32 = 0.4;
                let sampleCount: u32 = 8u;

                let inverseSqrtSampleCount = inverseSqrt(f32(sampleCount));

                var averageOccludingDepth: f32 = 0.0;
                var occludingDepthCount: f32 = 0.0;

                for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
                    let sampleOnPerpendicularDisk = sampleDiskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
                    let sampleDisplacement = generateSampleDisplacement(displacement, displacementNormalDirection, displacementBinormalDirection, sampleOnPerpendicularDisk);

                    let sampledDepth = textureSample(shadowMapTexture, pointSampler, sampleDisplacement);

                    if (sampledDepth < referenceDepth) {
                        averageOccludingDepth += sampledDepth;
                        occludingDepthCount += 1.0;
                    }
                }

                let minPenumbraExtent = 0.01;

                if (occludingDepthCount > 0.0) {
                    averageOccludingDepth /= occludingDepthCount;
                    return max(minPenumbraExtent, emissionRadius * (referenceDepth - averageOccludingDepth) / averageOccludingDepth);
                } else {
                    return -1.0;
                }
            }

            fn computeVogelDiskComparisonSampleAverage(
                shadowMapTexture: texture_depth_cube,
                comparisonSampler: sampler_comparison,
                vogelDiskBaseAngle: f32,
                sampleDiskRadius: f32,
                displacement: vec3<f32>,
                displacementNormalDirection: vec3<f32>,
                displacementBinormalDirection: vec3<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let sample_density = 800.0;

                let sampleCount = u32(clamp(sampleDiskRadius * sample_density, 3.0, 64.0));

                let invSampleCount = 1.0 / f32(sampleCount);
                let inverseSqrtSampleCount = sqrt(invSampleCount);

                var sampleAverage: f32 = 0.0;

                for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
                    let sampleOnPerpendicularDisk = sampleDiskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
                    let sampleDisplacement = generateSampleDisplacement(displacement, displacementNormalDirection, displacementBinormalDirection, sampleOnPerpendicularDisk);

                    sampleAverage += textureSampleCompare(shadowMapTexture, comparisonSampler, sampleDisplacement, referenceDepth);
                }

                sampleAverage *= invSampleCount;

                return sampleAverage;
            }

            fn computePCSSLightAccessFactor(
                shadowMapTexture: texture_depth_cube,
                pointSampler: sampler,
                comparisonSampler: sampler_comparison,
                emissionRadius: f32,
                cameraClipSpacePosition: vec4<f32>,
                lightSpaceFragmentDisplacement: vec3<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let vogelDiskBaseAngle = computeVogelDiskBaseAngle(cameraClipSpacePosition);

                let displacementNormalDirection = normalize(findPerpendicularVector(lightSpaceFragmentDisplacement));
                let displacementBinormalDirection = normalize(cross(lightSpaceFragmentDisplacement, displacementNormalDirection));
                
                let shadowPenumbraExtent = computeShadowPenumbraExtent(
                    shadowMapTexture,
                    pointSampler,
                    emissionRadius,
                    vogelDiskBaseAngle,
                    lightSpaceFragmentDisplacement,
                    displacementNormalDirection,
                    displacementBinormalDirection,
                    referenceDepth,
                );

                if (shadowPenumbraExtent < 0.0) {
                    return 1.0;
                }

                return computeVogelDiskComparisonSampleAverage(
                    shadowMapTexture,
                    comparisonSampler,
                    vogelDiskBaseAngle,
                    shadowPenumbraExtent,
                    lightSpaceFragmentDisplacement,
                    displacementNormalDirection,
                    displacementBinormalDirection,
                    referenceDepth,
                );
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let texture_var_expr =
            include_expr_in_func(function, Expression::GlobalVariable(self.texture_var));

        let sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(
                self.sampler_var
                    .expect("Missing sampler for PCSS shadow mapping"),
            ),
        );

        let comparison_sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(
                self.comparison_sampler_var
                    .expect("Missing comparison sampler for PCSS shadow mapping"),
            ),
        );

        SourceCode::generate_call_named(
            function,
            "lightAccessFactor",
            source_code.functions["computePCSSLightAccessFactor"],
            vec![
                texture_var_expr,
                sampler_var_expr,
                comparison_sampler_var_expr,
                emission_radius_expr,
                camera_clip_position_expr,
                light_space_fragment_displacement_expr,
                depth_reference_expr,
            ],
        )
    }

    fn generate_pcss_light_access_factor_expr_for_cascaded_shadow_map(
        &self,
        module: &mut Module,
        function: &mut Function,
        tan_angular_radius_expr: Handle<Expression>,
        world_to_light_clip_space_xy_scale_expr: Handle<Expression>,
        world_to_light_clip_space_z_scale_expr: Handle<Expression>,
        camera_clip_position_expr: Handle<Expression>,
        texture_coord_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
        array_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let source_code = SourceCode::from_wgsl_source(
            "\
            // Returns a random number between 0 and 1 based on the pixel coordinates
            fn generateInterleavedGradientNoiseFactor(cameraClipSpacePosition: vec4<f32>) -> f32 {
                let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
                return fract(magic.z * fract(dot(magic.xy, cameraClipSpacePosition.xy)));
            }

            fn generateVogelDiskSampleCoords(baseAngle: f32, inverseSqrtSampleCount: f32, sampleIdx: u32) -> vec2<f32> {
                let goldenAngle: f32 = 2.4;
                let radius = sqrt(f32(sampleIdx) + 0.5) * inverseSqrtSampleCount;
                let angle = baseAngle + goldenAngle * f32(sampleIdx);
                return vec2<f32>(radius * cos(angle), radius * sin(angle));
            }

            fn computeVogelDiskBaseAngle(cameraClipSpacePosition: vec4<f32>) -> f32 {
                // Multiply with 2 * pi to get random angle
                return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraClipSpacePosition);
            }

            fn computeShadowPenumbraExtent(
                shadowMapTexture: texture_depth_2d_array,
                pointSampler: sampler,
                array_index: i32,
                tanAngularRadius: f32,
                vogelDiskBaseAngle: f32,
                worldSpaceToLightClipSpaceXYScale: f32,
                worldSpaceToLightClipSpaceZScale: f32,
                centerTextureCoords: vec2<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let diskRadius: f32 = 0.4 * worldSpaceToLightClipSpaceXYScale;
                let sampleCount: u32 = 8u;

                let inverseSqrtSampleCount = inverseSqrt(f32(sampleCount));

                var averageOccludingDepth: f32 = 0.0;
                var occludingDepthCount: f32 = 0.0;

                for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
                    let sampleTextureCoords = centerTextureCoords + diskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
                    let sampledDepth = textureSample(shadowMapTexture, pointSampler, sampleTextureCoords, array_index);

                    if (sampledDepth < referenceDepth) {
                        averageOccludingDepth += sampledDepth;
                        occludingDepthCount += 1.0;
                    }
                }

                let minPenumbraExtent = 0.01;

                if (occludingDepthCount > 0.0) {
                    averageOccludingDepth /= occludingDepthCount;
                    return max(minPenumbraExtent, tanAngularRadius * (referenceDepth - averageOccludingDepth) / worldSpaceToLightClipSpaceZScale);
                } else {
                    return -1.0;
                }
            }

            fn computeVogelDiskComparisonSampleAverage(
                shadowMapTexture: texture_depth_2d_array,
                comparisonSampler: sampler_comparison,
                array_index: i32,
                vogelDiskBaseAngle: f32,
                worldSpaceToLightClipSpaceXYScale: f32,
                worldSpaceDiskRadius: f32,
                centerTextureCoords: vec2<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let sample_density = 800.0;

                let sampleCount = u32(clamp(worldSpaceDiskRadius * sample_density, 3.0, 64.0));

                let diskRadius = worldSpaceDiskRadius * worldSpaceToLightClipSpaceXYScale;

                let invSampleCount = 1.0 / f32(sampleCount);
                let inverseSqrtSampleCount = sqrt(invSampleCount);

                var sampleAverage: f32 = 0.0;

                for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
                    let sampleTextureCoords = centerTextureCoords + diskRadius * generateVogelDiskSampleCoords(vogelDiskBaseAngle, inverseSqrtSampleCount, sampleIdx);
                    sampleAverage += textureSampleCompare(shadowMapTexture, comparisonSampler, sampleTextureCoords, array_index, referenceDepth);
                }

                sampleAverage *= invSampleCount;

                return sampleAverage;
            }

            fn computePCSSLightAccessFactor(
                shadowMapTexture: texture_depth_2d_array,
                pointSampler: sampler,
                comparisonSampler: sampler_comparison,
                array_index: i32,
                tanAngularRadius: f32,
                worldSpaceToLightClipSpaceXYScale: f32,
                worldSpaceToLightClipSpaceZScale: f32,
                cameraClipSpacePosition: vec4<f32>,
                centerTextureCoords: vec2<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let vogelDiskBaseAngle = computeVogelDiskBaseAngle(cameraClipSpacePosition);
                
                let shadowPenumbraExtent = computeShadowPenumbraExtent(
                    shadowMapTexture,
                    pointSampler,
                    array_index,
                    tanAngularRadius,
                    vogelDiskBaseAngle,
                    worldSpaceToLightClipSpaceXYScale,
                    worldSpaceToLightClipSpaceZScale,
                    centerTextureCoords,
                    referenceDepth,
                );

                if (shadowPenumbraExtent < 0.0) {
                    return 1.0;
                }

                return computeVogelDiskComparisonSampleAverage(
                    shadowMapTexture,
                    comparisonSampler,
                    array_index,
                    vogelDiskBaseAngle,
                    worldSpaceToLightClipSpaceXYScale,
                    shadowPenumbraExtent,
                    centerTextureCoords,
                    referenceDepth,
                );
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let texture_var_expr =
            include_expr_in_func(function, Expression::GlobalVariable(self.texture_var));

        let sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(
                self.sampler_var
                    .expect("Missing sampler for PCSS shadow mapping"),
            ),
        );

        let comparison_sampler_var_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(
                self.comparison_sampler_var
                    .expect("Missing comparison sampler for PCSS shadow mapping"),
            ),
        );

        SourceCode::generate_call_named(
            function,
            "lightAccessFactor",
            source_code.functions["computePCSSLightAccessFactor"],
            vec![
                texture_var_expr,
                sampler_var_expr,
                comparison_sampler_var_expr,
                array_idx_expr,
                tan_angular_radius_expr,
                world_to_light_clip_space_xy_scale_expr,
                world_to_light_clip_space_z_scale_expr,
                camera_clip_position_expr,
                texture_coord_expr,
                depth_reference_expr,
            ],
        )
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
                    TypeInner::Array { base, size, stride } => {
                        let size = match size {
                            ArraySize::Constant(c) => ArraySize::Constant(self.import_const(*c)),
                            ArraySize::Dynamic => ArraySize::Dynamic,
                        };
                        TypeInner::Array {
                            base: self.import_type(*base),
                            size,
                            stride: *stride,
                        }
                    }
                    TypeInner::BindingArray { base, size } => {
                        let size = match size {
                            ArraySize::Constant(c) => ArraySize::Constant(self.import_const(*c)),
                            ArraySize::Dynamic => ArraySize::Dynamic,
                        };
                        TypeInner::BindingArray {
                            base: self.import_type(*base),
                            size,
                        }
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
                specialization: c.specialization,
                inner: match &c.inner {
                    ConstantInner::Scalar { .. } => c.inner.clone(),
                    ConstantInner::Composite { ty, components } => {
                        let components = components.iter().map(|c| self.import_const(*c)).collect();
                        ConstantInner::Composite {
                            ty: self.import_type(*ty),
                            components,
                        }
                    }
                },
            };

            let new_h =
                define_constant_if_missing(&mut self.exported_to_module.constants, new_const);
            self.const_map.insert(h_const, new_h);
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
                init: gv.init.map(|c| self.import_const(c)),
            };

            let new_h = append_to_arena(&mut self.exported_to_module.global_variables, new_global);
            self.global_map.insert(h_global, new_h);
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
                                value: case.value.clone(),
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
                offset: offset.map(|c| self.import_const(c)),
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
            Expression::Derivative { axis, expr } => Expression::Derivative {
                axis: *axis,
                expr: map_expr!(expr),
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

            Expression::LocalVariable(_) | Expression::FunctionArgument(_) => {
                is_external = true;
                expr.clone()
            }

            Expression::AtomicResult { .. } => expr.clone(),
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

        let mut local_variables = Arena::new();
        for (h_l, l) in func.local_variables.iter() {
            let new_local = LocalVariable {
                name: l.name.clone(),
                ty: self.import_type(l.ty),
                init: l.init.map(|c| self.import_const(c)),
            };
            let new_h = append_to_arena(&mut local_variables, new_local);
            assert_eq!(h_l, new_h);
        }

        let mut expressions = Arena::new();
        let mut expr_map = HashMap::new();

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
        Ok(Self { module })
    }

    /// Imports the functions and named types into the given module.
    ///
    /// # Returns
    /// The handles to the imported functions and types.
    pub fn import_to_module(&self, module: &mut Module) -> SourceCodeHandles {
        let mut importer = ModuleImporter::new(&self.module, module);

        let mut function_handles = HashMap::with_capacity(self.module.functions.len());
        for (function, func) in self.module.functions.iter() {
            function_handles.insert(
                func.name.as_ref().unwrap().to_string(),
                importer.import_function(function).unwrap(),
            );
        }

        let mut type_handles = HashMap::with_capacity(self.module.types.len());
        for (type_handle, ty) in self.module.types.iter() {
            if let Some(name) = ty.name.as_ref() {
                type_handles.insert(name.to_string(), importer.import_type(type_handle));
            }
        }

        SourceCodeHandles {
            functions: function_handles,
            types: type_handles,
        }
    }

    /// Generates the code calling a function with the given handle with the
    /// given argument expressions, assigning the given name to the return
    /// expression.
    ///
    /// # Returns
    /// The return value expression.
    pub fn generate_call_named(
        function: &mut Function,
        name: impl ToString,
        function_handle: Handle<Function>,
        arguments: Vec<Handle<Expression>>,
    ) -> Handle<Expression> {
        let return_expr =
            include_named_expr_in_func(function, name, Expression::CallResult(function_handle));

        push_to_block(
            &mut function.body,
            Statement::Call {
                function: function_handle,
                arguments,
                result: Some(return_expr),
            },
        );

        return_expr
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

fn u32_constant(value: u64) -> Constant {
    Constant {
        name: None,
        specialization: None,
        inner: ConstantInner::Scalar {
            width: U32_WIDTH as Bytes,
            value: ScalarValue::Uint(value),
        },
    }
}

fn float32_constant(value: f64) -> Constant {
    Constant {
        name: None,
        specialization: None,
        inner: ConstantInner::Scalar {
            width: F32_WIDTH as Bytes,
            value: ScalarValue::Float(value),
        },
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

/// Inserts the given value in the given [`UniqueArena`]
/// if it is not already present.
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

/// Pushes the given [`Statement`] to the given [`Block`]
/// of statements.
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
    constants: &mut Arena<Constant>,
    function: &mut Function,
    vec3_expr: Handle<Expression>,
) -> Handle<Expression> {
    let vec4_type = insert_in_arena(types, VECTOR_4_TYPE);

    let unity_constant_expr = include_expr_in_func(
        function,
        Expression::Constant(define_constant_if_missing(constants, float32_constant(1.0))),
    );

    emit_in_func(function, |function| {
        include_expr_in_func(
            function,
            Expression::Compose {
                ty: vec4_type,
                components: vec![vec3_expr, unity_constant_expr],
            },
        )
    })
}

#[cfg(test)]
mod test {
    #![allow(clippy::dbg_macro)]

    use crate::scene::{
        BlinnPhongMaterial, DiffuseTexturedBlinnPhongMaterial, FixedColorMaterial,
        FixedTextureMaterial, GlobalAmbientColorMaterial, TexturedBlinnPhongMaterial,
        VertexColorMaterial,
    };

    use super::*;
    use naga::{
        back::wgsl::{self as wgsl_out, WriterFlags},
        front::wgsl as wgsl_in,
        valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
    };

    const INSTANCE_VERTEX_BINDING_START: u32 = 0;
    const MESH_VERTEX_BINDING_START: u32 = 10;
    const MATERIAL_VERTEX_BINDING_START: u32 = 20;

    const CAMERA_INPUT: CameraShaderInput = CameraShaderInput {
        projection_matrix_binding: 0,
    };

    const MODEL_VIEW_TRANSFORM_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::ModelViewTransform(ModelViewTransformShaderInput {
            rotation_location: INSTANCE_VERTEX_BINDING_START,
            translation_and_scaling_location: INSTANCE_VERTEX_BINDING_START + 1,
        });

    const MINIMAL_MESH_INPUT: MeshShaderInput = MeshShaderInput {
        locations: [Some(MESH_VERTEX_BINDING_START), None, None, None],
    };

    const FIXED_COLOR_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
            color_location: MATERIAL_VERTEX_BINDING_START,
        });

    const FIXED_TEXTURE_INPUT: MaterialShaderInput =
        MaterialShaderInput::Fixed(Some(FixedTextureShaderInput {
            color_texture_and_sampler_bindings: (0, 1),
        }));

    const BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START),
            specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
            shininess_location: MATERIAL_VERTEX_BINDING_START + 2,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            diffuse_color_location: None,
            specular_color_location: Some(MATERIAL_VERTEX_BINDING_START),
            shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
        });

    const GLOBAL_AMBIENT_COLOR_INPUT: MaterialShaderInput =
        MaterialShaderInput::GlobalAmbientColor(GlobalAmbientColorShaderInput {
            uniform_binding: 0,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialShaderInput =
        MaterialShaderInput::BlinnPhong(Some(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: None,
        }));

    const TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            diffuse_color_location: None,
            specular_color_location: None,
            shininess_location: MATERIAL_VERTEX_BINDING_START,
        });

    const TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialShaderInput =
        MaterialShaderInput::BlinnPhong(Some(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: Some((2, 3)),
        }));

    const POINT_LIGHT_INPUT: LightShaderInput =
        LightShaderInput::PointLight(PointLightShaderInput {
            uniform_binding: 0,
            max_light_count: 20,
            shadow_map_texture_and_sampler_binding: (1, 2, 3),
        });

    const UNIDIRECTIONAL_LIGHT_INPUT: LightShaderInput =
        LightShaderInput::UnidirectionalLight(UnidirectionalLightShaderInput {
            uniform_binding: 4,
            max_light_count: 20,
            shadow_map_texture_and_sampler_bindings: (5, 6, 7),
        });

    fn validate_module(module: &Module) -> ModuleInfo {
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        match validator.validate(module) {
            Ok(module_info) => module_info,
            Err(err) => {
                dbg!(module);
                dbg!(&err);
                eprintln!("{}", err);
                panic!("Shader validation failed")
            }
        }
    }

    #[test]
    fn parse() {
        match wgsl_in::parse_str(
            "
            fn generateVogelDiskSampleCoords(invSqrtSampleCount: f32, baseAngle: f32, sampleIdx: u32) -> vec2<f32> {
                let goldenAngle: f32 = 2.4;
                let radius = sqrt(sampleIdx + 0.5) * invSqrtSampleCount;
                let angle = baseAngle + goldenAngle * sampleIdx;
                return vec2<f32>(radius * cos(angle), radius * sin(angle));
            }

            fn computeVogelDiskComparisonSampleAverage(
                shadowMapTexture: texture_depth_2d_array,
                comparisonSampler: sampler_comparison,
                array_index: i32,
                //sampleCount: u32,
                //diskRadius: f32,
                centerTextureCoords: vec2<f32>,
                referenceDepth: f32,
            ) -> f32 {
                let sampleCount: u32 = 16u;
                let diskRadius: f32 = 0.01;

                let invSqrtSampleCount = inverseSqrt(sampleCount);
                var sampleAverage: f32 = 0.0;

                for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
                    let sampleTextureCoords = centerTextureCoords + diskRadius * generateVogelDiskSampleCoords(invSqrtSampleCount, 0.0, sampleIdx);
                    sampleAverage += textureSampleCompare(shadowMapTexture, comparisonSampler, sampleTextureCoords, array_index, referenceDepth);
                }

                sampleAverage /= sampleCount;

                return sampleAverage;
            }
            ",
        ) {
            Ok(module) => {
                dbg!(module);
            }
            Err(err) => {
                println!("{}", err);
                panic!()
            }
        }
    }

    #[test]
    #[should_panic]
    fn building_shader_with_no_inputs_fails() {
        ShaderGenerator::generate_shader_module(
            None,
            None,
            None,
            &[],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_input_fails() {
        ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            None,
            None,
            &[],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_and_mesh_input_fails() {
        ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap();
    }

    #[test]
    fn building_depth_prepass_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_point_light_shadow_map_update_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&POINT_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_unidirectional_light_shadow_map_update_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_global_ambient_color_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&GLOBAL_AMBIENT_COLOR_INPUT),
            GlobalAmbientColorMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_vertex_color_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&MaterialShaderInput::VertexColor),
            VertexColorMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_fixed_color_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT, &FIXED_COLOR_FEATURE_INPUT],
            Some(&MaterialShaderInput::Fixed(None)),
            FixedColorMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_fixed_texture_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                ],
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&FIXED_TEXTURE_INPUT),
            FixedTextureMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_blinn_phong_shader_with_point_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                ],
            }),
            Some(&POINT_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT, &BLINN_PHONG_FEATURE_INPUT],
            Some(&MaterialShaderInput::BlinnPhong(None)),
            BlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT, &BLINN_PHONG_FEATURE_INPUT],
            Some(&MaterialShaderInput::BlinnPhong(None)),
            BlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_diffuse_textured_blinn_phong_shader_with_point_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&POINT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
            DiffuseTexturedBlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_diffuse_textured_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
            DiffuseTexturedBlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_blinn_phong_shader_with_point_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&POINT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
            TexturedBlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
            TexturedBlinnPhongMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
        )
        .unwrap()
        .0;

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }
}
