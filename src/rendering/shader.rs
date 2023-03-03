//! Generation of graphics shaders.

mod ambient_color;
mod blinn_phong;
mod depth;
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
};
use ambient_color::GlobalAmbientColorShaderGenerator;
use anyhow::{anyhow, Result};
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use depth::LightSpaceDepthShaderGenerator;
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
    LightSpaceDepth,
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
    DirectionalLight(DirectionalLightShaderInput),
}

/// Input description for point light sources, specifying the bind group
/// binding and the total size of the point light uniform buffer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
}

/// Input description for directional light sources, specifying the bind group
/// binding and the total size of the directional light uniform buffer as well
/// as shadow map bindings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirectionalLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
    /// Bind group bindings of the shadow map texture and sampler.
    pub shadow_map_texture_and_sampler_binding: (u32, u32),
}

/// Shader generator for any kind of material.
#[derive(Clone, Debug)]
pub enum MaterialShaderGenerator<'a> {
    GlobalAmbientColor(GlobalAmbientColorShaderGenerator<'a>),
    VertexColor,
    FixedColor(FixedColorShaderGenerator<'a>),
    FixedTexture(FixedTextureShaderGenerator<'a>),
    BlinnPhong(BlinnPhongShaderGenerator<'a>),
    LightSpaceDepth,
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
    DirectionalLight(DirectionalLightProjectionExpressions),
}

/// Handle to expression for the camera projection matrix.
#[derive(Clone, Debug)]
pub struct CameraProjectionExpressions {
    matrix_expr_handle: Handle<Expression>,
}

/// Handle to expressions for the orthographic transform components associated
/// with a directional light.
#[derive(Clone, Debug)]
pub struct DirectionalLightProjectionExpressions {
    pub translation: Handle<Expression>,
    pub scaling: Handle<Expression>,
}

/// Handles to expression for accessing the fields of a light uniform for the
/// relevant light type in shader entry point functions.
#[derive(Clone, Debug)]
pub enum LightFieldExpressions {
    PointLight(PointLightFieldExpressions),
    DirectionalLight(DirectionalLightFieldExpressions),
}

/// Handles to expression for accessing the fields of a point light uniform in
/// shader entry point functions.
#[derive(Clone, Debug)]
pub struct PointLightFieldExpressions {
    pub in_fragment_function: PointLightFieldFragmentExpressions,
}

/// Handles to expression for accessing the fields of a directional light
/// uniform in shader entry point functions.
#[derive(Clone, Debug)]
pub struct DirectionalLightFieldExpressions {
    pub in_vertex_function: DirectionalLightVertexFieldExpressions,
    pub in_fragment_function: Option<DirectionalLightFragmentFieldExpressions>,
}

/// Handles to expression for accessing the fields of a point light uniform in
/// the fragment entry point function.
#[derive(Clone, Debug)]
pub struct PointLightFieldFragmentExpressions {
    pub position: Handle<Expression>,
    pub radiance: Handle<Expression>,
}

/// Handles to expression for accessing the fields of a directional light
/// uniform in the vertex entry point function.
#[derive(Clone, Debug)]
pub struct DirectionalLightVertexFieldExpressions {
    pub camera_to_light_space_rotation_quaternion: Handle<Expression>,
    pub orthographic_projection: DirectionalLightProjectionExpressions,
}

/// Handles to expression for accessing the fields of a directional light
/// uniform in the fragment entry point function. Also holds a
/// [`SampledTexture`] that can be used for sampling the shadow map.
#[derive(Clone, Debug)]
pub struct DirectionalLightFragmentFieldExpressions {
    pub camera_space_direction: Handle<Expression>,
    pub radiance: Handle<Expression>,
    pub shadow_map: SampledTexture,
}

/// Indices of the fields holding the various mesh vertex attributes and related
/// quantities in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MeshVertexOutputFieldIndices {
    pub _clip_position: usize,
    pub position: Option<usize>,
    pub color: Option<usize>,
    pub normal_vector: Option<usize>,
    pub texture_coords: Option<usize>,
}

/// Indices of the fields holding the various light related properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct LightVertexOutputFieldIndices {
    pub light_clip_position: Option<usize>,
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
    input_field_expr_handles: Vec<Handle<Expression>>,
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
    input_expr_handles: Vec<Handle<Expression>>,
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
    texture_var_handle: Handle<GlobalVariable>,
    sampler_var_handle: Handle<GlobalVariable>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureType {
    Image,
    Depth,
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

/// A set of shader functions defined in source code that can be imported into
/// an existing [`Module`].
#[derive(Debug)]
pub struct SourceCodeFunctions {
    module: Module,
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

const IMAGE_TEXTURE_SAMPLER_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Sampler { comparison: false },
};

const DEPTH_TEXTURE_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Image {
        dim: ImageDimension::D2,
        arrayed: false,
        class: ImageClass::Depth { multi: false },
    },
};

const COMPARISON_SAMPLER_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Sampler { comparison: true },
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
        vertex_attribute_requirements: VertexAttributeSet,
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

        let light = light_shader_input.map(|light_shader_input| {
            Self::generate_code_for_lights(
                light_shader_input,
                &mut module,
                &mut vertex_function,
                if material_shader_generator.is_some() {
                    Some(&mut fragment_function)
                } else {
                    None
                },
                &mut bind_group_idx,
            )
        });

        let projection = if let Some(camera_projection) = camera_projection {
            ProjectionExpressions::Camera(camera_projection)
        } else {
            light
                .as_ref()
                .ok_or_else(|| {
                    anyhow!(
                        "Tried to build shader with no camera or light input (missing projection)"
                    )
                })?
                .try_get_projection_to_light_clip_space()?
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
            let light_vertex_output_field_indices = light.as_ref().map(|light| {
                light.generate_vertex_output_code(
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
                light.as_ref(),
            );

            let entry_point_names = EntryPointNames {
                vertex: Cow::Borrowed("mainVS"),
                fragment: Some(Cow::Borrowed("mainFS")),
            };

            module.entry_points.push(EntryPoint {
                name: entry_point_names.vertex.to_string(),
                stage: ShaderStage::Vertex,
                early_depth_test: None,
                workgroup_size: [0, 0, 0],
                function: vertex_function,
            });

            module.entry_points.push(EntryPoint {
                name: entry_point_names.fragment.as_ref().unwrap().to_string(),
                stage: ShaderStage::Fragment,
                early_depth_test: None,
                workgroup_size: [0, 0, 0],
                function: fragment_function,
            });

            entry_point_names
        } else {
            vertex_output_struct_builder
                .generate_output_code(&mut module.types, &mut vertex_function);

            let entry_point_names = EntryPointNames {
                vertex: Cow::Borrowed("mainVS"),
                fragment: None,
            };

            module.entry_points.push(EntryPoint {
                name: entry_point_names.vertex.to_string(),
                stage: ShaderStage::Vertex,
                early_depth_test: None,
                workgroup_size: [0, 0, 0],
                function: vertex_function,
            });

            entry_point_names
        };

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
            (None, None, Some(MaterialShaderInput::LightSpaceDepth)) => {
                Some(MaterialShaderGenerator::LightSpaceDepth)
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
        let vec4_type_handle = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let model_view_transform_type = Type {
            name: new_name("ModelViewTransform"),
            inner: TypeInner::Struct {
                members: vec![
                    StructMember {
                        name: new_name("rotationQuaternion"),
                        ty: vec4_type_handle,
                        binding: Some(Binding::Location {
                            location: model_view_transform_shader_input.rotation_location,
                            interpolation: None,
                            sampling: None,
                        }),
                        offset: 0,
                    },
                    StructMember {
                        name: new_name("translationAndScaling"),
                        ty: vec4_type_handle,
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

        let model_view_transform_type_handle =
            insert_in_arena(&mut module.types, model_view_transform_type);

        let model_view_transform_arg_ptr_expr_handle = generate_input_argument(
            vertex_function,
            new_name("modelViewTransform"),
            model_view_transform_type_handle,
            None,
        );

        let (rotation_quaternion_expr_handle, translation_expr_handle, scaling_expr_handle) =
            emit_in_func(vertex_function, |function| {
                let rotation_quaternion_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: model_view_transform_arg_ptr_expr_handle,
                        index: 0,
                    },
                );
                let translation_and_scaling_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: model_view_transform_arg_ptr_expr_handle,
                        index: 1,
                    },
                );
                let translation_expr_handle = include_expr_in_func(
                    function,
                    swizzle_xyz_expr(translation_and_scaling_expr_handle),
                );
                let scaling_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: translation_and_scaling_expr_handle,
                        index: 3,
                    },
                );
                (
                    rotation_quaternion_expr_handle,
                    translation_expr_handle,
                    scaling_expr_handle,
                )
            });

        ModelViewTransformExpressions {
            rotation_quaternion: rotation_quaternion_expr_handle,
            translation_vector: translation_expr_handle,
            scaling_factor: scaling_expr_handle,
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

        let mat4x4_type_handle = insert_in_arena(&mut module.types, MATRIX_4X4_TYPE);

        let projection_matrix_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("projectionMatrix"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: bind_group,
                    binding: camera_shader_input.projection_matrix_binding,
                }),
                ty: mat4x4_type_handle,
                init: None,
            },
        );

        let projection_matrix_ptr_expr_handle = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(projection_matrix_var_handle),
        );

        let matrix_expr_handle = emit_in_func(vertex_function, |function| {
            include_named_expr_in_func(
                function,
                "projectionMatrix",
                Expression::Load {
                    pointer: projection_matrix_ptr_expr_handle,
                },
            )
        });

        CameraProjectionExpressions { matrix_expr_handle }
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
        model_view_transform_expressions: &ModelViewTransformExpressions,
        projection: ProjectionExpressions,
    ) -> Result<(MeshVertexOutputFieldIndices, OutputStructBuilder)> {
        let function_handles = SourceCodeFunctions::from_wgsl_source(
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

        let rotation_function_handle = function_handles[0];
        let transformation_function_handle = function_handles[1];

        let vec2_type_handle = insert_in_arena(&mut module.types, VECTOR_2_TYPE);
        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let input_model_position_expr_handle =
            Self::add_vertex_attribute_input_argument::<VertexPosition<fre>>(
                vertex_function,
                mesh_shader_input,
                new_name("modelSpacePosition"),
                vec3_type_handle,
            )?;

        let input_color_expr_handle = if requirements.contains(VertexAttributeSet::COLOR) {
            Some(
                Self::add_vertex_attribute_input_argument::<VertexColor<fre>>(
                    vertex_function,
                    mesh_shader_input,
                    new_name("color"),
                    vec3_type_handle,
                )?,
            )
        } else {
            None
        };

        let input_model_normal_vector_expr_handle =
            if requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
                Some(Self::add_vertex_attribute_input_argument::<
                    VertexNormalVector<fre>,
                >(
                    vertex_function,
                    mesh_shader_input,
                    new_name("modelSpaceNormalVector"),
                    vec3_type_handle,
                )?)
            } else {
                None
            };

        let input_texture_coord_expr_handle =
            if requirements.contains(VertexAttributeSet::TEXTURE_COORDS) {
                Some(Self::add_vertex_attribute_input_argument::<
                    VertexTextureCoords<fre>,
                >(
                    vertex_function,
                    mesh_shader_input,
                    new_name("textureCoords"),
                    vec2_type_handle,
                )?)
            } else {
                None
            };

        let position_expr_handle = SourceCodeFunctions::generate_call(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            transformation_function_handle,
            vec![
                model_view_transform_expressions.rotation_quaternion,
                model_view_transform_expressions.translation_vector,
                model_view_transform_expressions.scaling_factor,
                input_model_position_expr_handle,
            ],
        );

        let mut output_struct_builder = OutputStructBuilder::new("VertexOutput");

        let clip_position_expr_handle =
            projection.generate_clip_position_expr(module, vertex_function, position_expr_handle);
        let output_clip_position_field_idx = output_struct_builder.add_builtin_position_field(
            "clipSpacePosition",
            vec4_type_handle,
            VECTOR_4_SIZE,
            clip_position_expr_handle,
        );

        let mut output_field_indices = MeshVertexOutputFieldIndices {
            _clip_position: output_clip_position_field_idx,
            position: None,
            color: None,
            normal_vector: None,
            texture_coords: None,
        };

        if requirements.contains(VertexAttributeSet::POSITION) {
            output_field_indices.position = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "position",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    position_expr_handle,
                ),
            );
        }

        if let Some(input_color_expr_handle) = input_color_expr_handle {
            output_field_indices.color = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "color",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    input_color_expr_handle,
                ),
            );
        }

        if let Some(input_model_normal_vector_expr_handle) = input_model_normal_vector_expr_handle {
            let normal_vector_expr_handle = SourceCodeFunctions::generate_call(
                &mut vertex_function.body,
                &mut vertex_function.expressions,
                rotation_function_handle,
                vec![
                    model_view_transform_expressions.rotation_quaternion,
                    input_model_normal_vector_expr_handle,
                ],
            );

            output_field_indices.normal_vector = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "normalVector",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    normal_vector_expr_handle,
                ),
            );
        }

        if let Some(input_texture_coord_expr_handle) = input_texture_coord_expr_handle {
            output_field_indices.texture_coords = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "textureCoords",
                    vec2_type_handle,
                    VECTOR_2_SIZE,
                    input_texture_coord_expr_handle,
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

    /// Generates declarations for the uniform type of the active light type,
    /// the type the corresponding light uniform buffer will be mapped to, the
    /// global variable this is bound to and expressions for the fields of the
    /// light at the active index (which is set in a push constant).
    ///
    /// # Returns
    /// Handle to expressions for accessing the fields of the point light in the
    /// relevant entry point functions.
    fn generate_code_for_lights(
        light_shader_input: &LightShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        fragment_function: Option<&mut Function>,
        bind_group_idx: &mut u32,
    ) -> LightFieldExpressions {
        match light_shader_input {
            LightShaderInput::PointLight(light_shader_input) => {
                Self::generate_code_for_point_lights(
                    light_shader_input,
                    module,
                    fragment_function.unwrap(),
                    bind_group_idx,
                )
            }
            LightShaderInput::DirectionalLight(light_shader_input) => {
                Self::generate_code_for_directional_lights(
                    light_shader_input,
                    module,
                    vertex_function,
                    fragment_function,
                    bind_group_idx,
                )
            }
        }
    }

    /// Generates declarations for the point light uniform type, the type the
    /// point light uniform buffer will be mapped to, the global variable this
    /// is bound to and expressions for the fields of the light at the active
    /// index (which is set in a push constant).
    ///
    /// # Returns
    /// Handle to expressions for accessing the fields of the active point light
    /// in the fragment entry point function.
    fn generate_code_for_point_lights(
        light_shader_input: &PointLightShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
    ) -> LightFieldExpressions {
        let u32_type_handle = insert_in_arena(&mut module.types, U32_TYPE);
        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        // The struct is padded to 16 byte alignment as required for uniforms
        let single_light_struct_size = 2 * (VECTOR_3_SIZE + F32_WIDTH);

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let single_light_struct_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("PointLight"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("position"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("radiance"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: VECTOR_3_SIZE + F32_WIDTH,
                        },
                    ],
                    span: single_light_struct_size,
                },
            },
        );

        let max_light_count_constant_handle = define_constant_if_missing(
            &mut module.constants,
            u32_constant(light_shader_input.max_light_count),
        );

        let light_array_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type_handle,
                    size: ArraySize::Constant(max_light_count_constant_handle),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("PointLights"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("numLights"),
                            ty: u32_type_handle,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("lights"),
                            ty: light_array_type_handle,
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

        let lights_struct_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("pointLights"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: light_shader_input.uniform_binding,
                }),
                ty: lights_struct_type_handle,
                init: None,
            },
        );

        *bind_group_idx += 1;

        let active_light_idx_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("activeLightIdx"),
                space: AddressSpace::PushConstant,
                binding: None,
                ty: u32_type_handle,
                init: None,
            },
        );

        LightFieldExpressions::generate_point_light_code(
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
        )
    }

    /// Generates declarations for the directional light uniform type, the type
    /// the directional light uniform buffer will be mapped to, the global
    /// variable this is bound to and expressions for the fields of the light at
    /// the active index (which is set in a push constant).
    ///
    /// # Returns
    /// Handle to expressions for accessing the fields of the point light in the
    /// vertex and fragment entry point functions.
    fn generate_code_for_directional_lights(
        light_shader_input: &DirectionalLightShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        fragment_function: Option<&mut Function>,
        bind_group_idx: &mut u32,
    ) -> LightFieldExpressions {
        let u32_type_handle = insert_in_arena(&mut module.types, U32_TYPE);
        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        // The struct is padded to 16 byte alignment as required for uniforms
        let single_light_struct_size = 5 * VECTOR_4_SIZE;

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let single_light_struct_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("DirectionalLight"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("cameraToLightRotationQuaternion"),
                            ty: vec4_type_handle,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("cameraSpaceDirection"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("radiance"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: 2 * VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("orthographicTranslation"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: 3 * VECTOR_4_SIZE,
                        },
                        StructMember {
                            name: new_name("orthographicScaling"),
                            ty: vec3_type_handle,
                            binding: None,
                            offset: 4 * VECTOR_4_SIZE,
                        },
                        // <-- The rest of the struct is not needed in the
                        // shader and is mainly for padding
                    ],
                    span: single_light_struct_size,
                },
            },
        );

        let max_light_count_constant_handle = define_constant_if_missing(
            &mut module.constants,
            u32_constant(light_shader_input.max_light_count),
        );

        let lights_array_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type_handle,
                    size: ArraySize::Constant(max_light_count_constant_handle),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type_handle = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("DirectionalLights"),
                inner: TypeInner::Struct {
                    members: vec![
                        StructMember {
                            name: new_name("numLights"),
                            ty: u32_type_handle,
                            binding: None,
                            offset: 0,
                        },
                        StructMember {
                            name: new_name("lights"),
                            ty: lights_array_type_handle,
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

        let lights_struct_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("directionalLights"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: light_shader_input.uniform_binding,
                }),
                ty: lights_struct_type_handle,
                init: None,
            },
        );

        *bind_group_idx += 1;

        let (shadow_map_texture_binding, shadow_map_sampler_binding) =
            light_shader_input.shadow_map_texture_and_sampler_binding;

        let shadow_map = if fragment_function.is_some() {
            Some(SampledTexture::declare(
                &mut module.types,
                &mut module.global_variables,
                TextureType::Depth,
                "shadowMap",
                *bind_group_idx,
                shadow_map_texture_binding,
                shadow_map_sampler_binding,
            ))
        } else {
            None
        };

        *bind_group_idx += 1;

        let active_light_idx_var_handle = append_to_arena(
            &mut module.global_variables,
            GlobalVariable {
                name: new_name("activeLightIdx"),
                space: AddressSpace::PushConstant,
                binding: None,
                ty: u32_type_handle,
                init: None,
            },
        );

        LightFieldExpressions::generate_directional_light_code(
            vertex_function,
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
            shadow_map,
        )
    }
}

impl MaterialShaderInput {
    /// Whether the material requires light sources.
    pub fn requires_lights(&self) -> bool {
        match self {
            Self::GlobalAmbientColor(_) | Self::VertexColor | Self::Fixed(_) => false,
            Self::BlinnPhong(_) | Self::LightSpaceDepth => true,
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
        light_expressions: Option<&LightFieldExpressions>,
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
                light_expressions,
            ),
            (Self::LightSpaceDepth, MaterialVertexOutputFieldIndices::None) => {
                LightSpaceDepthShaderGenerator::generate_fragment_code(
                    module,
                    fragment_function,
                    fragment_input_struct,
                    light_input_field_indices,
                );
            }
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
        position_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        match self {
            Self::Camera(camera_projection_matrix) => camera_projection_matrix
                .generate_clip_position_expr(module, vertex_function, position_expr_handle),
            Self::DirectionalLight(directional_light_orthographic_projection) => {
                directional_light_orthographic_projection.generate_clip_position_expr(
                    module,
                    vertex_function,
                    position_expr_handle,
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
        position_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let homogeneous_position_expr_handle = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            vertex_function,
            position_expr_handle,
        );

        emit_in_func(vertex_function, |function| {
            include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: self.matrix_expr_handle,
                    right: homogeneous_position_expr_handle,
                },
            )
        })
    }
}

impl DirectionalLightProjectionExpressions {
    /// Generates an expression for the given position (as a vec3) projected
    /// with the orthographic projection in the vertex entry point function. The
    /// projected position will be a vec4 with w = 1.0;
    pub fn generate_clip_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        position_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let function_handles = SourceCodeFunctions::from_wgsl_source(
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

        let projection_function_handle = function_handles[0];

        let light_clip_space_position_expr_handle = SourceCodeFunctions::generate_call(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            projection_function_handle,
            vec![self.translation, self.scaling, position_expr_handle],
        );

        append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            vertex_function,
            light_clip_space_position_expr_handle,
        )
    }
}

impl LightFieldExpressions {
    pub fn generate_point_light_code(
        fragment_function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
    ) -> Self {
        Self::PointLight(PointLightFieldExpressions::generate_code(
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
        ))
    }

    pub fn generate_directional_light_code(
        vertex_function: &mut Function,
        fragment_function: Option<&mut Function>,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
        shadow_map: Option<SampledTexture>,
    ) -> Self {
        Self::DirectionalLight(DirectionalLightFieldExpressions::generate_code(
            vertex_function,
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
            shadow_map,
        ))
    }

    pub fn try_get_projection_to_light_clip_space(&self) -> Result<ProjectionExpressions> {
        if let Self::DirectionalLight(directional_light_field_expressions) = self {
            Ok(directional_light_field_expressions.get_projection_to_light_clip_space())
        } else {
            Err(anyhow!(
                "Requested projection for point light in shader (unsupported)"
            ))
        }
    }

    pub fn generate_vertex_output_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        output_struct_builder: &mut OutputStructBuilder,
        mesh_output_field_indices: &MeshVertexOutputFieldIndices,
    ) -> LightVertexOutputFieldIndices {
        match self {
            Self::PointLight(_) => LightVertexOutputFieldIndices {
                light_clip_position: None,
            },
            Self::DirectionalLight(directional_light_field_expressions) => {
                directional_light_field_expressions.generate_vertex_output_code(
                    module,
                    vertex_function,
                    output_struct_builder,
                    mesh_output_field_indices,
                )
            }
        }
    }

    fn generate_active_light_ptr_expr(
        function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
    ) -> Handle<Expression> {
        let lights_struct_ptr_expr_handle = include_expr_in_func(
            function,
            Expression::GlobalVariable(lights_struct_var_handle),
        );

        let active_light_idx_ptr_expr_handle = include_expr_in_func(
            function,
            Expression::GlobalVariable(active_light_idx_var_handle),
        );

        let active_light_idx_expr_handle = emit_in_func(function, |function| {
            include_named_expr_in_func(
                function,
                "activeLightIdx",
                Expression::Load {
                    pointer: active_light_idx_ptr_expr_handle,
                },
            )
        });

        Self::generate_single_light_ptr_expr(
            function,
            lights_struct_ptr_expr_handle,
            active_light_idx_expr_handle,
        )
    }

    fn generate_light_count_expr(
        function: &mut Function,
        lights_struct_ptr_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        Self::generate_field_access_expr(function, lights_struct_ptr_expr_handle, 0)
    }

    fn generate_single_light_ptr_expr(
        function: &mut Function,
        lights_struct_ptr_expr_handle: Handle<Expression>,
        light_idx_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let lights_field_ptr_handle =
            Self::generate_field_access_ptr_expr(function, lights_struct_ptr_expr_handle, 1);

        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::Access {
                    base: lights_field_ptr_handle,
                    index: light_idx_expr_handle,
                },
            )
        })
    }

    fn generate_field_access_expr(
        function: &mut Function,
        struct_ptr_expr_handle: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        let field_ptr_handle =
            Self::generate_field_access_ptr_expr(function, struct_ptr_expr_handle, field_idx);
        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::Load {
                    pointer: field_ptr_handle,
                },
            )
        })
    }

    fn generate_field_access_ptr_expr(
        function: &mut Function,
        struct_ptr_expr_handle: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: struct_ptr_expr_handle,
                    index: field_idx,
                },
            )
        })
    }
}

impl PointLightFieldExpressions {
    pub fn generate_code(
        fragment_function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
    ) -> Self {
        Self {
            in_fragment_function: PointLightFieldFragmentExpressions::generate_code(
                fragment_function,
                lights_struct_var_handle,
                active_light_idx_var_handle,
            ),
        }
    }
}

impl PointLightFieldFragmentExpressions {
    pub fn generate_code(
        fragment_function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
    ) -> Self {
        let active_light_ptr_expr_handle = LightFieldExpressions::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
        );

        let position = LightFieldExpressions::generate_field_access_expr(
            fragment_function,
            active_light_ptr_expr_handle,
            0,
        );

        let radiance = LightFieldExpressions::generate_field_access_expr(
            fragment_function,
            active_light_ptr_expr_handle,
            1,
        );

        Self { position, radiance }
    }
}

impl DirectionalLightFieldExpressions {
    pub fn generate_code(
        vertex_function: &mut Function,
        fragment_function: Option<&mut Function>,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
        shadow_map: Option<SampledTexture>,
    ) -> Self {
        Self {
            in_vertex_function: DirectionalLightVertexFieldExpressions::generate_code(
                vertex_function,
                lights_struct_var_handle,
                active_light_idx_var_handle,
            ),
            in_fragment_function: fragment_function.map(|fragment_function| {
                DirectionalLightFragmentFieldExpressions::generate_code(
                    fragment_function,
                    lights_struct_var_handle,
                    active_light_idx_var_handle,
                    shadow_map.unwrap(),
                )
            }),
        }
    }

    pub fn generate_vertex_output_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        output_struct_builder: &mut OutputStructBuilder,
        mesh_output_field_indices: &MeshVertexOutputFieldIndices,
    ) -> LightVertexOutputFieldIndices {
        let function_handles = SourceCodeFunctions::from_wgsl_source(
            "\
            fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }

            fn transformCameraSpacePositionToDirectionalLightClipSpace(
                cameraToLightRotationQuaternion: vec4<f32>,
                orthographicTranslation: vec3<f32>,
                orthographicScaling: vec3<f32>,
                cameraSpacePosition: vec3<f32>
            ) -> vec3<f32> {
                let lightSpacePosition = rotateVectorWithQuaternion(cameraToLightRotationQuaternion, cameraSpacePosition);
                return (lightSpacePosition + orthographicTranslation) * orthographicScaling;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let light_clip_space_transformation_function_handle = function_handles[1];

        let vec3_type_handle = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let camera_space_position_expr_handle = output_struct_builder
            .get_field_expr(
                mesh_output_field_indices
                    .position
                    .expect("Missing position for shading with directional light"),
            )
            .unwrap();

        let light_clip_space_position_expr_handle = SourceCodeFunctions::generate_call(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            light_clip_space_transformation_function_handle,
            vec![
                self.in_vertex_function
                    .camera_to_light_space_rotation_quaternion,
                self.in_vertex_function.orthographic_projection.translation,
                self.in_vertex_function.orthographic_projection.scaling,
                camera_space_position_expr_handle,
            ],
        );

        LightVertexOutputFieldIndices {
            light_clip_position: Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "lightClipSpacePosition",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    light_clip_space_position_expr_handle,
                ),
            ),
        }
    }

    fn get_projection_to_light_clip_space(&self) -> ProjectionExpressions {
        self.in_vertex_function.get_projection_to_light_clip_space()
    }
}

impl DirectionalLightVertexFieldExpressions {
    pub fn generate_code(
        vertex_function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
    ) -> Self {
        let active_light_ptr_expr_handle = LightFieldExpressions::generate_active_light_ptr_expr(
            vertex_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
        );

        let camera_to_light_space_rotation_quaternion =
            LightFieldExpressions::generate_field_access_expr(
                vertex_function,
                active_light_ptr_expr_handle,
                0,
            );

        let orthographic_translation = LightFieldExpressions::generate_field_access_expr(
            vertex_function,
            active_light_ptr_expr_handle,
            3,
        );

        let orthographic_scaling = LightFieldExpressions::generate_field_access_expr(
            vertex_function,
            active_light_ptr_expr_handle,
            4,
        );

        let orthographic_projection = DirectionalLightProjectionExpressions {
            translation: orthographic_translation,
            scaling: orthographic_scaling,
        };

        Self {
            camera_to_light_space_rotation_quaternion,
            orthographic_projection,
        }
    }

    pub fn get_projection_to_light_clip_space(&self) -> ProjectionExpressions {
        ProjectionExpressions::DirectionalLight(self.orthographic_projection.clone())
    }
}

impl DirectionalLightFragmentFieldExpressions {
    pub fn generate_code(
        fragment_function: &mut Function,
        lights_struct_var_handle: Handle<GlobalVariable>,
        active_light_idx_var_handle: Handle<GlobalVariable>,
        shadow_map: SampledTexture,
    ) -> Self {
        let active_light_ptr_expr_handle = LightFieldExpressions::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var_handle,
            active_light_idx_var_handle,
        );

        let camera_space_direction = LightFieldExpressions::generate_field_access_expr(
            fragment_function,
            active_light_ptr_expr_handle,
            1,
        );

        let radiance = LightFieldExpressions::generate_field_access_expr(
            fragment_function,
            active_light_ptr_expr_handle,
            2,
        );

        Self {
            camera_space_direction,
            radiance,
            shadow_map,
        }
    }
}

impl InputStruct {
    /// Returns the handle to the expression for the struct
    /// field with the given index.
    ///
    /// # Panics
    /// If the index is out of bounds.
    pub fn get_field_expr_handle(&self, idx: usize) -> Handle<Expression> {
        self.input_field_expr_handles[idx]
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

        let input_type_handle = insert_in_arena(types, self.builder.into_type());

        let input_arg_ptr_expr_handle =
            generate_input_argument(function, Some(self.input_arg_name), input_type_handle, None);

        let input_field_expr_handles = emit_in_func(function, |function| {
            (0..n_fields)
                .into_iter()
                .map(|idx| {
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: input_arg_ptr_expr_handle,
                            index: idx as u32,
                        },
                    )
                })
                .collect()
        });

        InputStruct {
            input_field_expr_handles,
        }
    }
}

impl OutputStructBuilder {
    /// Creates a builder for an output struct with the given
    /// type name.
    pub fn new<S: ToString>(type_name: S) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_expr_handles: Vec::new(),
            location: 0,
        }
    }

    /// Returns the expression for the field with the given index, or [`None`]
    /// if no field exists for the index.
    pub fn get_field_expr(&self, field_idx: usize) -> Option<Handle<Expression>> {
        self.input_expr_handles.get(field_idx).copied()
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
        input_expr_handle: Handle<Expression>,
    ) -> usize {
        self.input_expr_handles.push(input_expr_handle);

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
        input_expr_handle: Handle<Expression>,
    ) -> usize {
        self.add_field(
            name,
            type_handle,
            Some(Interpolation::Perspective),
            Some(Sampling::Center),
            size,
            input_expr_handle,
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
        input_expr_handle: Handle<Expression>,
    ) -> usize {
        self.input_expr_handles.push(input_expr_handle);

        self.builder.add_field(
            name,
            type_handle,
            Some(Binding::BuiltIn(BuiltIn::Position { invariant: false })),
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
        let output_type_handle = insert_in_arena(types, self.builder.into_type());

        function.result = Some(FunctionResult {
            ty: output_type_handle,
            binding: None,
        });

        let output_ptr_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut function.local_variables,
                LocalVariable {
                    name: new_name("output"),
                    ty: output_type_handle,
                    init: None,
                },
            )),
        );

        for (idx, input_expr_handle) in self.input_expr_handles.into_iter().enumerate() {
            let output_struct_field_ptr_handle = emit_in_func(function, |function| {
                include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: output_ptr_expr_handle,
                        index: idx as u32,
                    },
                )
            });
            push_to_block(
                &mut function.body,
                Statement::Store {
                    pointer: output_struct_field_ptr_handle,
                    value: input_expr_handle,
                },
            );
        }

        let output_expr_handle = emit_in_func(function, |function| {
            include_named_expr_in_func(
                function,
                "output",
                Expression::Load {
                    pointer: output_ptr_expr_handle,
                },
            )
        });

        push_to_block(
            &mut function.body,
            Statement::Return {
                value: Some(output_expr_handle),
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
    /// Generates code declaring global variables for a texture
    /// and sampler with the given name root, group and bindings.
    ///
    /// # Returns
    /// A new [`SampledTexture`] with handles to the declared
    /// variables.
    pub fn declare(
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        texture_type: TextureType,
        name: &'static str,
        group: u32,
        texture_binding: u32,
        sampler_binding: u32,
    ) -> Self {
        let (texture_type_const, sampler_type_const) = match texture_type {
            TextureType::Image => (IMAGE_TEXTURE_TYPE, IMAGE_TEXTURE_SAMPLER_TYPE),
            TextureType::Depth => (DEPTH_TEXTURE_TYPE, COMPARISON_SAMPLER_TYPE),
        };

        let texture_type_handle = insert_in_arena(types, texture_type_const);
        let sampler_type_handle = insert_in_arena(types, sampler_type_const);

        let texture_var_handle = append_to_arena(
            global_variables,
            GlobalVariable {
                name: Some(format!("{}Texture", name)),
                space: AddressSpace::Handle,
                binding: Some(ResourceBinding {
                    group,
                    binding: texture_binding,
                }),
                ty: texture_type_handle,
                init: None,
            },
        );

        let sampler_var_handle = append_to_arena(
            global_variables,
            GlobalVariable {
                name: Some(format!("{}Sampler", name)),
                space: AddressSpace::Handle,
                binding: Some(ResourceBinding {
                    group,
                    binding: sampler_binding,
                }),
                ty: sampler_type_handle,
                init: None,
            },
        );

        Self {
            texture_var_handle,
            sampler_var_handle,
        }
    }

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression. If sampling a depth
    /// texture, a reference depth must also be provided for the comparison
    /// sampling.
    pub fn generate_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr_handle: Handle<Expression>,
        depth_reference_expr_handle: Option<Handle<Expression>>,
    ) -> Handle<Expression> {
        let texture_var_expr_handle = include_expr_in_func(
            function,
            Expression::GlobalVariable(self.texture_var_handle),
        );

        let sampler_var_expr_handle = include_expr_in_func(
            function,
            Expression::GlobalVariable(self.sampler_var_handle),
        );

        let image_sampling_expr_handle = emit_in_func(function, |function| {
            include_expr_in_func(
                function,
                Expression::ImageSample {
                    image: texture_var_expr_handle,
                    sampler: sampler_var_expr_handle,
                    gather: None,
                    coordinate: texture_coord_expr_handle,
                    array_index: None,
                    offset: None,
                    level: SampleLevel::Auto,
                    depth_ref: depth_reference_expr_handle,
                },
            )
        });

        image_sampling_expr_handle
    }

    /// Generates and returns an expression sampling the texture at
    /// the texture coordinates specified by the given expression,
    /// and extracting the RGB values of the sampled RGBA color.
    pub fn generate_rgb_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let sampling_expr_handle =
            self.generate_sampling_expr(function, texture_coord_expr_handle, None);

        emit_in_func(function, |function| {
            include_expr_in_func(function, swizzle_xyz_expr(sampling_expr_handle))
        })
    }

    /// Generates and returns an expression comparison sampling the depth
    /// texture with texture coordinates converted from the x- and y-component
    /// of the given light clip space position and the z-component as the
    /// reference depth.
    pub fn generate_shadow_map_sampling_expr(
        &self,
        types: &mut UniqueArena<Type>,
        constants: &mut Arena<Constant>,
        function: &mut Function,
        light_clip_position_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let vec2_type_handle = insert_in_arena(types, VECTOR_2_TYPE);

        let unity_constant_expr = include_expr_in_func(
            function,
            Expression::Constant(define_constant_if_missing(constants, float32_constant(1.0))),
        );

        let half_constant_expr = include_expr_in_func(
            function,
            Expression::Constant(define_constant_if_missing(constants, float32_constant(0.5))),
        );

        let (texture_coords_expr_handle, depth_reference_expr_handle) =
            emit_in_func(function, |function| {
                // Map x [-1, 1] to u [0, 1]

                let light_clip_position_x_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: light_clip_position_expr_handle,
                        index: 0,
                    },
                );

                let offset_light_clip_position_x_expr_handle = include_expr_in_func(
                    function,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: light_clip_position_x_expr_handle,
                        right: unity_constant_expr,
                    },
                );

                let u_texture_coord_expr_handle = include_expr_in_func(
                    function,
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: offset_light_clip_position_x_expr_handle,
                        right: half_constant_expr,
                    },
                );

                // Map y [-1, 1] to v [1, 0]

                let light_clip_position_y_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: light_clip_position_expr_handle,
                        index: 1,
                    },
                );

                let negated_light_clip_position_y_expr_handle = include_expr_in_func(
                    function,
                    Expression::Unary {
                        op: UnaryOperator::Negate,
                        expr: light_clip_position_y_expr_handle,
                    },
                );

                let offset_negated_light_clip_position_y_expr_handle = include_expr_in_func(
                    function,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: negated_light_clip_position_y_expr_handle,
                        right: unity_constant_expr,
                    },
                );

                let v_texture_coord_expr_handle = include_expr_in_func(
                    function,
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: offset_negated_light_clip_position_y_expr_handle,
                        right: half_constant_expr,
                    },
                );

                let texture_coords_expr_handle = include_expr_in_func(
                    function,
                    Expression::Compose {
                        ty: vec2_type_handle,
                        components: vec![u_texture_coord_expr_handle, v_texture_coord_expr_handle],
                    },
                );

                let depth_reference_expr_handle = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: light_clip_position_expr_handle,
                        index: 2,
                    },
                );

                (texture_coords_expr_handle, depth_reference_expr_handle)
            });

        self.generate_sampling_expr(
            function,
            texture_coords_expr_handle,
            Some(depth_reference_expr_handle),
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
    pub fn import_function(
        &mut self,
        function_handle: Handle<Function>,
    ) -> Result<Handle<Function>> {
        let func = self
            .imported_from_module
            .functions
            .try_get(function_handle)?;
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

impl SourceCodeFunctions {
    /// Parses the given WGSL source code into a new set of
    /// [`SourceCodeFunctions`].
    ///
    /// # Errors
    /// Returns an error if the string contains invalid source code.
    pub fn from_wgsl_source(source: &str) -> Result<Self> {
        let module = naga::front::wgsl::parse_str(source)?;
        Ok(Self { module })
    }

    /// Imports the functions into the given module.
    ///
    /// # Returns
    /// The handles to the imported functions.
    pub fn import_to_module(&self, module: &mut Module) -> Vec<Handle<Function>> {
        let mut importer = ModuleImporter::new(&self.module, module);

        let mut function_handles = Vec::with_capacity(self.module.functions.len());
        for (function_handle, _) in self.module.functions.iter() {
            function_handles.push(importer.import_function(function_handle).unwrap());
        }
        function_handles
    }

    /// Generates the code calling a function with the given handle with the
    /// given argument expressions.
    ///
    /// # Returns
    /// The return value expression.
    pub fn generate_call(
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        function_handle: Handle<Function>,
        arguments: Vec<Handle<Expression>>,
    ) -> Handle<Expression> {
        let return_expr_handle =
            append_to_arena(expressions, Expression::CallResult(function_handle));

        push_to_block(
            block,
            Statement::Call {
                function: function_handle,
                arguments,
                result: Some(return_expr_handle),
            },
        );

        return_expr_handle
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
    input_type_handle: Handle<Type>,
    location: u32,
) -> Handle<Expression> {
    generate_input_argument(
        function,
        input_arg_name,
        input_type_handle,
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
    input_type_handle: Handle<Type>,
    binding: Option<Binding>,
) -> Handle<Expression> {
    let input_arg_idx = u32::try_from(function.arguments.len()).unwrap();

    function.arguments.push(FunctionArgument {
        name: input_arg_name,
        ty: input_type_handle,
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

fn swizzle_xyz_expr(expr_handle: Handle<Expression>) -> Expression {
    Expression::Swizzle {
        size: VectorSize::Tri,
        vector: expr_handle,
        pattern: [
            SwizzleComponent::X,
            SwizzleComponent::Y,
            SwizzleComponent::Z,
            SwizzleComponent::X,
        ],
    }
}

fn swizzle_xy_expr(expr_handle: Handle<Expression>) -> Expression {
    Expression::Swizzle {
        size: VectorSize::Bi,
        vector: expr_handle,
        pattern: [
            SwizzleComponent::X,
            SwizzleComponent::Y,
            SwizzleComponent::X,
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
    vec3_expr_handle: Handle<Expression>,
) -> Handle<Expression> {
    let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

    let unity_constant_expr = include_expr_in_func(
        function,
        Expression::Constant(define_constant_if_missing(constants, float32_constant(1.0))),
    );

    emit_in_func(function, |function| {
        include_expr_in_func(
            function,
            Expression::Compose {
                ty: vec4_type_handle,
                components: vec![vec3_expr_handle, unity_constant_expr],
            },
        )
    })
}

#[cfg(test)]
mod test {
    #![allow(clippy::dbg_macro)]

    use crate::scene::{
        BlinnPhongMaterial, DiffuseTexturedBlinnPhongMaterial, FixedColorMaterial,
        FixedTextureMaterial, GlobalAmbientColorMaterial, LightSpaceDepthMaterial,
        TexturedBlinnPhongMaterial, VertexColorMaterial,
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
        });

    const DIRECTIONAL_LIGHT_INPUT: LightShaderInput =
        LightShaderInput::DirectionalLight(DirectionalLightShaderInput {
            uniform_binding: 1,
            max_light_count: 20,
            shadow_map_texture_and_sampler_binding: (2, 3),
        });

    fn validate_module(module: &Module) -> ModuleInfo {
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        match validator.validate(module) {
            Ok(module_info) => module_info,
            Err(err) => {
                eprintln!("{}", err);
                dbg!(err);
                dbg!(module);
                panic!("Shader validation failed")
            }
        }
    }

    #[test]
    fn parse() {
        match wgsl_in::parse_str(
            "
            var<push_constant> pc: f32;
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
    fn building_directional_light_shadow_map_update_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&DIRECTIONAL_LIGHT_INPUT),
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
    fn building_blinn_phong_shader_with_directional_light_works() {
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
            Some(&DIRECTIONAL_LIGHT_INPUT),
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
    fn building_diffuse_textured_blinn_phong_shader_with_directional_light_works() {
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
            Some(&DIRECTIONAL_LIGHT_INPUT),
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
    fn building_textured_blinn_phong_shader_with_directional_light_works() {
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
            Some(&DIRECTIONAL_LIGHT_INPUT),
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
    fn building_light_space_depth_shader_with_directional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            Some(&DIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&MaterialShaderInput::LightSpaceDepth),
            LightSpaceDepthMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS,
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
