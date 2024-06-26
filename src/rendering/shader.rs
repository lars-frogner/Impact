//! Generation of graphics shaders.

mod ambient_occlusion;
mod blinn_phong;
mod fixed;
mod gaussian_blur;
mod microfacet;
mod passthrough;
mod prepass;
mod skybox;
mod tone_mapping;
mod vertex_color;

pub use ambient_occlusion::{AmbientOcclusionCalculationShaderInput, AmbientOcclusionShaderInput};
pub use blinn_phong::BlinnPhongTextureShaderInput;
pub use fixed::{FixedColorFeatureShaderInput, FixedTextureShaderInput};
pub use gaussian_blur::{GaussianBlurShaderGenerator, GaussianBlurShaderInput};
pub use microfacet::{
    DiffuseMicrofacetShadingModel, MicrofacetShadingModel, MicrofacetTextureShaderInput,
    SpecularMicrofacetShadingModel,
};
pub use passthrough::{PassthroughShaderGenerator, PassthroughShaderInput};
pub use prepass::{
    BumpMappingTextureShaderInput, NormalMappingShaderInput, ParallaxMappingShaderInput,
    PrepassShaderGenerator, PrepassTextureShaderInput,
};
pub use skybox::{SkyboxShaderGenerator, SkyboxTextureShaderInput};
pub use tone_mapping::{ToneMappingShaderGenerator, ToneMappingShaderInput};

use crate::{
    geometry::{
        VertexAttribute, VertexAttributeSet, VertexColor, VertexNormalVector, VertexPosition,
        VertexTangentSpaceQuaternion, VertexTextureCoords, N_VERTEX_ATTRIBUTES,
    },
    rendering::{fre, CoreRenderingSystem, RenderAttachmentQuantitySet},
    scene::MAX_SHADOW_MAP_CASCADES,
};
use ambient_occlusion::AmbientOcclusionShaderGenerator;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use fixed::{
    FixedColorShaderGenerator, FixedColorVertexOutputFieldIdx, FixedTextureShaderGenerator,
};
use lazy_static::lazy_static;
use microfacet::{MicrofacetShaderGenerator, MicrofacetVertexOutputFieldIndices};
use naga::{
    AddressSpace, Arena, ArraySize, BinaryOperator, Binding, Block, BuiltIn, Bytes, Constant,
    EntryPoint, Expression, Function, FunctionArgument, FunctionResult, GlobalVariable, Handle,
    ImageClass, ImageDimension, ImageQuery, Interpolation, Literal, LocalVariable, Module,
    Override, ResourceBinding, SampleLevel, Sampling, Scalar, ScalarKind, ShaderStage, Span,
    Statement, StructMember, SwitchCase, SwizzleComponent, Type, TypeInner, UnaryOperator,
    UniqueArena, VectorSize,
};
use prepass::PrepassVertexOutputFieldIndices;
use skybox::SkyboxVertexOutputFieldIndices;
use std::{
    borrow::Cow, collections::HashMap, fs, hash::Hash, mem, num::NonZeroU32, path::Path, vec,
};
use vertex_color::VertexColorShaderGenerator;

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
    LightMaterial(LightMaterialFeatureShaderInput),
    /// For convenience in unit tests.
    #[cfg(test)]
    None,
}

/// Input description specifying the vertex attribute locations of material
/// properties of a a light shaded material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightMaterialFeatureShaderInput {
    /// Vertex attribute location for the instance feature representing the
    /// albedo of the material.
    pub albedo_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing the
    /// specular reflectance of the material.
    pub specular_reflectance_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing the
    /// emissive luminance of the material.
    pub emissive_luminance_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing the
    /// roughness of the material.
    pub roughness_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing the
    /// displacement scale for parallax mapping.
    pub parallax_displacement_scale_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing the
    /// change in UV texture coordinates per world space distance for parallax
    /// mapping.
    pub parallax_uv_per_distance_location: Option<u32>,
}

/// Input description for any kind of material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaterialShaderInput {
    VertexColor,
    Fixed(Option<FixedTextureShaderInput>),
    BlinnPhong(BlinnPhongTextureShaderInput),
    Microfacet((MicrofacetShadingModel, MicrofacetTextureShaderInput)),
    Prepass(PrepassTextureShaderInput),
    Skybox(SkyboxTextureShaderInput),
    Passthrough(PassthroughShaderInput),
    AmbientOcclusion(AmbientOcclusionShaderInput),
    GaussianBlur(GaussianBlurShaderInput),
    ToneMapping(ToneMappingShaderInput),
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
    AmbientLight(AmbientLightShaderInput),
    OmnidirectionalLight(OmnidirectionalLightShaderInput),
    UnidirectionalLight(UnidirectionalLightShaderInput),
}

/// Input description for ambient light sources, specifying the bind group
/// binding and the total size of the ambient light uniform buffer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AmbientLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
}

/// Input description for omnidirectional light sources, specifying the bind
/// group binding and the total size of the omnidirectional light uniform buffer
/// as well as shadow map bindings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OmnidirectionalLightShaderInput {
    /// Bind group binding of the light uniform buffer.
    pub uniform_binding: u32,
    /// Maximum number of lights in the uniform buffer.
    pub max_light_count: u64,
    /// Bind group bindings of the shadow map texture, sampler and comparison
    /// sampler, respectively.
    pub shadow_map_texture_and_sampler_bindings: (u32, u32, u32),
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
    VertexColor,
    FixedColor(FixedColorShaderGenerator<'a>),
    FixedTexture(FixedTextureShaderGenerator<'a>),
    BlinnPhong(BlinnPhongShaderGenerator<'a>),
    Microfacet(MicrofacetShaderGenerator<'a>),
    Prepass(PrepassShaderGenerator<'a>),
    Skybox(SkyboxShaderGenerator<'a>),
    Passthrough(PassthroughShaderGenerator<'a>),
    AmbientOcclusion(AmbientOcclusionShaderGenerator<'a>),
    GaussianBlur(GaussianBlurShaderGenerator<'a>),
    ToneMapping(ToneMappingShaderGenerator<'a>),
}

bitflags! {
    /// Bitflag encoding a set of "tricks" that can be made to achieve certain
    /// effects.
    pub struct ShaderTricks: u8 {
        /// Ignore the translational part of the model-to-camera transform when
        /// transforming the position.
        const FOLLOW_CAMERA = 0b00000001;
        /// Make the depth of every fragment in framebuffer space 1.0.
        const DRAW_AT_MAX_DEPTH = 0b00000010;
        /// Do not apply the projection to the vertex position.
        const NO_VERTEX_PROJECTION = 0b00000100;
    }
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
    Camera(CameraProjectionVariable),
    OmnidirectionalLight(OmnidirectionalLightProjectionExpressions),
    UnidirectionalLight(UnidirectionalLightProjectionExpressions),
}

/// Handle to the global variable for the camera projection matrix.
#[derive(Clone, Debug)]
pub struct CameraProjectionVariable {
    projection_matrix_var: Handle<GlobalVariable>,
}

/// Marker type with method for projecting points onto a face of a shadow
/// cubemap.
#[derive(Clone, Debug)]
pub struct OmnidirectionalLightProjectionExpressions;

/// Handle to expressions for the orthographic transform components associated
/// with a unidirectional light.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightProjectionExpressions {
    pub translation: Handle<Expression>,
    pub scaling: Handle<Expression>,
}

#[allow(clippy::enum_variant_names)]
/// Generator for shader code associated with a light source.
#[derive(Clone, Debug)]
pub enum LightShaderGenerator {
    AmbientLight(AmbientLightShaderGenerator),
    OmnidirectionalLight(OmnidirectionalLightShaderGenerator),
    UnidirectionalLight(UnidirectionalLightShaderGenerator),
}

/// Generator for shader code for shading a fragment with the light from an
/// ambient light.
#[derive(Clone, Debug)]
pub struct AmbientLightShaderGenerator {
    pub luminance: Handle<Expression>,
}

/// Generator for shader code associated with an omnidirectional light source.
#[derive(Clone, Debug)]
pub enum OmnidirectionalLightShaderGenerator {
    ForShadowMapUpdate(OmnidirectionalLightShadowMapUpdateShaderGenerator),
    ForShading(OmnidirectionalLightShadingShaderGenerator),
}

/// Generator for shader code associated with a unidirectional light source.
#[derive(Clone, Debug)]
pub enum UnidirectionalLightShaderGenerator {
    ForShadowMapUpdate(UnidirectionalLightShadowMapUpdateShaderGenerator),
    ForShading(UnidirectionalLightShadingShaderGenerator),
}

/// Generator for shader code for updating the shadow cubemap of an
/// omnidirectional light.
#[derive(Clone, Debug)]
pub struct OmnidirectionalLightShadowMapUpdateShaderGenerator {
    pub near_distance: Handle<Expression>,
    pub inverse_distance_span: Handle<Expression>,
}

/// Generator for shader code for shading a fragment with the light from an
/// omnidirectional light.
#[derive(Clone, Debug)]
pub struct OmnidirectionalLightShadingShaderGenerator {
    pub camera_to_light_space_rotation_quaternion: Handle<Expression>,
    pub camera_space_position: Handle<Expression>,
    pub luminous_intensity: Handle<Expression>,
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

/// Expressions for vertex attributes passed as input to the vertex entry point
/// function.
pub struct MeshVertexInputExpressions {
    pub position: Handle<Expression>,
    pub color: Option<Handle<Expression>>,
    pub normal_vector: Option<Handle<Expression>>,
    pub texture_coords: Option<Handle<Expression>>,
    pub tangent_space_quaternion: Option<Handle<Expression>>,
}

/// Indices of the fields holding the various mesh vertex attributes and related
/// quantities in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MeshVertexOutputFieldIndices {
    pub framebuffer_position: usize,
    /// Camera space position.
    pub position: Option<usize>,
    pub color: Option<usize>,
    /// Camera space normal vector.
    pub normal_vector: Option<usize>,
    pub texture_coords: Option<usize>,
    /// Quaternion for rotation from tangent space to camera space.
    pub tangent_space_quaternion: Option<usize>,
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
    pub light_space_normal_vector: Option<usize>,
}

/// Indices of any fields holding the properties of a
/// specific material in the vertex shader output struct.
#[derive(Clone, Debug)]
pub enum MaterialVertexOutputFieldIndices {
    FixedColor(FixedColorVertexOutputFieldIdx),
    BlinnPhong(BlinnPhongVertexOutputFieldIndices),
    Microfacet(MicrofacetVertexOutputFieldIndices),
    Prepass(PrepassVertexOutputFieldIndices),
    Skybox(SkyboxVertexOutputFieldIndices),
    None,
}

/// Helper for constructing a struct containing push constants.
#[derive(Clone, Debug)]
pub struct PushConstantStruct {
    builder: StructBuilder,
    field_indices: PushConstantFieldIndices,
    global_variable: Option<Handle<GlobalVariable>>,
}

/// Indices of fields in the struct containing push constants.
#[derive(Clone, Debug)]
pub struct PushConstantFieldIndices {
    inverse_window_dimensions: usize,
    light_idx: Option<usize>,
    cascade_idx: Option<usize>,
    exposure: Option<usize>,
}

/// Expressions for accessing fields in the struct containing push constants.
#[derive(Clone, Debug)]
pub struct PushConstantFieldExpressions {
    pub inverse_window_dimensions: Handle<Expression>,
    pub light_idx: Option<Handle<Expression>>,
    pub cascade_idx: Option<Handle<Expression>>,
    pub exposure: Option<Handle<Expression>>,
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

/// Helper for generating code for a for-loop.
pub struct ForLoop {
    /// Expression for the index in the for-loop body.
    pub idx_expr: Handle<Expression>,
    /// Set of statements making up the for-loop body.
    pub body: Block,
    continuing: Block,
    break_if: Option<Handle<Expression>>,
    start_expr: Handle<Expression>,
    end_expr: Handle<Expression>,
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
pub struct ModuleImporter<'a, 'b> {
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
pub struct SourceCode {
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

lazy_static! {
    pub static ref SHADER_SOURCE_LIB: SourceCode = SourceCode::from_wgsl_source(concat!(
        include_str!("../../shader/util.wgsl"),
        include_str!("../../shader/light.wgsl"),
        include_str!("../../shader/normal_map.wgsl"),
        include_str!("../../shader/blinn_phong.wgsl"),
        include_str!("../../shader/microfacet.wgsl"),
        include_str!("../../shader/ambient_occlusion.wgsl"),
        include_str!("../../shader/gaussian_blur.wgsl"),
        include_str!("../../shader/tone_mapping.wgsl")
    ))
    .unwrap_or_else(|err| panic!("Error when including shader source library: {}", err));
}

impl Shader {
    /// Creates a new shader by reading the source from the given file.
    ///
    /// # Errors
    /// Returns an error if the shader file can not be found or read.
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
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
    ) -> Result<(Module, EntryPointNames)> {
        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let (model_view_transform_shader_input, material_shader_generator) =
            Self::interpret_inputs(instance_feature_shader_inputs, material_shader_input)?;

        let mut module = Module::default();
        let mut vertex_function = Function::default();
        let mut fragment_function = Function::default();

        let mut source_code_lib = SHADER_SOURCE_LIB.clone();

        let mut push_constant_struct = PushConstantStruct::new(&mut module);

        if let Some(light_shader_input) = light_shader_input {
            Self::add_light_push_constants(
                light_shader_input,
                &mut module,
                &mut push_constant_struct,
                material_shader_generator.is_some(),
            );
        }

        if let Some(material_shader_input) = material_shader_input {
            Self::add_material_push_constants(
                material_shader_input,
                &mut module,
                &mut push_constant_struct,
            );
        }

        let push_constant_vertex_expressions =
            push_constant_struct.generate_expressions(&mut module, &mut vertex_function);
        let push_constant_fragment_expressions =
            push_constant_struct.generate_expressions(&mut module, &mut fragment_function);

        // Caution: The order in which the shader generators use and increment
        // the bind group index must match the order in which the bind groups
        // are set in `RenderPassRecorder::record_render_pass`, that is:
        // 1. Camera.
        // 2. Lights.
        // 3. Shadow map textures.
        // 4. Fixed material resources.
        // 5. Render attachment textures.
        // 6. Material property textures.
        let mut bind_group_idx = 0;

        let camera_projection = camera_shader_input.map(|camera_shader_input| {
            Self::generate_code_for_projection_matrix(
                camera_shader_input,
                &mut module,
                &mut bind_group_idx,
            )
        });

        let model_view_transform =
            model_view_transform_shader_input.map(|model_view_transform_shader_input| {
                Self::generate_vertex_code_for_model_view_transform(
                    model_view_transform_shader_input,
                    &mut module,
                    &mut vertex_function,
                )
            });

        let light_shader_generator = light_shader_input.map(|light_shader_input| {
            Self::create_light_shader_generator(
                light_shader_input,
                &mut module,
                &mut vertex_function,
                &mut fragment_function,
                &mut bind_group_idx,
                &mut vertex_attribute_requirements,
                &push_constant_vertex_expressions,
                &push_constant_fragment_expressions,
                material_shader_generator.is_some(),
            )
        });

        let projection = if let Some(camera_projection) = camera_projection.clone() {
            Some(ProjectionExpressions::Camera(camera_projection))
        } else if let Some(light_shader_generator) = &light_shader_generator {
            light_shader_generator.get_projection_to_light_clip_space()
        } else {
            None
        };

        let tricks = material_shader_generator
            .as_ref()
            .map_or_else(ShaderTricks::empty, |generator| generator.tricks());

        let (
            mesh_vertex_input_expressions,
            mesh_vertex_output_field_indices,
            mut vertex_output_struct_builder,
        ) = Self::generate_vertex_code_for_vertex_attributes(
            mesh_shader_input,
            vertex_attribute_requirements,
            input_render_attachment_quantities,
            tricks,
            &mut module,
            &mut source_code_lib,
            &mut vertex_function,
            model_view_transform.as_ref(),
            projection,
        )?;

        let entry_point_names = if let Some(material_shader_generator) = material_shader_generator {
            let light_vertex_output_field_indices =
                light_shader_generator.as_ref().and_then(|light| {
                    light.generate_vertex_output_code_for_shading(
                        &mut module,
                        &mut source_code_lib,
                        &mut vertex_function,
                        &mut vertex_output_struct_builder,
                        &mesh_vertex_output_field_indices,
                    )
                });

            let material_vertex_output_field_indices = material_shader_generator
                .generate_vertex_code(
                    &mut module,
                    &mut vertex_function,
                    &mesh_vertex_input_expressions,
                    &mut vertex_output_struct_builder,
                );

            vertex_output_struct_builder
                .clone()
                .generate_output_code(&mut module.types, &mut vertex_function);

            let fragment_input_struct = vertex_output_struct_builder
                .generate_input_code(&mut module.types, &mut fragment_function);

            material_shader_generator.generate_fragment_code(
                &mut module,
                &mut source_code_lib,
                &mut fragment_function,
                &mut bind_group_idx,
                input_render_attachment_quantities,
                output_render_attachment_quantities,
                &push_constant_fragment_expressions,
                camera_projection.as_ref(),
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
                            &mut source_code_lib,
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
    /// Returns an error if `instance_feature_shader_inputs` and
    /// `material_shader_input` do not provide a consistent and supported
    /// material description.
    ///
    /// # Panics
    /// If `instance_feature_shader_inputs` contain multiple inputs of the same
    /// type.
    fn interpret_inputs<'a>(
        instance_feature_shader_inputs: &'a [&'a InstanceFeatureShaderInput],
        material_shader_input: Option<&'a MaterialShaderInput>,
    ) -> Result<(
        Option<&'a ModelViewTransformShaderInput>,
        Option<MaterialShaderGenerator<'a>>,
    )> {
        let mut model_view_transform_shader_input = None;
        let mut fixed_color_feature_shader_input = None;
        let mut light_material_feature_shader_input = None;

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
                InstanceFeatureShaderInput::LightMaterial(shader_input) => {
                    let old = light_material_feature_shader_input.replace(shader_input);
                    assert!(old.is_none());
                }
                #[cfg(test)]
                InstanceFeatureShaderInput::None => {}
            }
        }

        let material_shader_builder = match (
            fixed_color_feature_shader_input,
            light_material_feature_shader_input,
            material_shader_input,
        ) {
            (None, None, None) => None,
            (None, None, Some(MaterialShaderInput::VertexColor)) => {
                Some(MaterialShaderGenerator::VertexColor)
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
                    BlinnPhongShaderGenerator::new(feature_input, texture_input),
                ))
            }
            (
                None,
                Some(feature_input),
                Some(MaterialShaderInput::Microfacet((model, texture_input))),
            ) => Some(MaterialShaderGenerator::Microfacet(
                MicrofacetShaderGenerator::new(model, feature_input, texture_input),
            )),
            (None, Some(feature_input), Some(MaterialShaderInput::Prepass(texture_input))) => {
                Some(MaterialShaderGenerator::Prepass(
                    PrepassShaderGenerator::new(feature_input, texture_input),
                ))
            }
            (None, None, Some(MaterialShaderInput::Skybox(input))) => Some(
                MaterialShaderGenerator::Skybox(SkyboxShaderGenerator::new(input)),
            ),
            (None, None, Some(MaterialShaderInput::Passthrough(input))) => Some(
                MaterialShaderGenerator::Passthrough(PassthroughShaderGenerator::new(input)),
            ),
            (None, None, Some(MaterialShaderInput::AmbientOcclusion(input))) => {
                Some(MaterialShaderGenerator::AmbientOcclusion(
                    AmbientOcclusionShaderGenerator::new(input),
                ))
            }
            (None, None, Some(MaterialShaderInput::GaussianBlur(input))) => Some(
                MaterialShaderGenerator::GaussianBlur(GaussianBlurShaderGenerator::new(input)),
            ),
            (None, None, Some(MaterialShaderInput::ToneMapping(input))) => Some(
                MaterialShaderGenerator::ToneMapping(ToneMappingShaderGenerator::new(input)),
            ),
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
                            second_blend_source: false,
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
                            second_blend_source: false,
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
    /// projection matrix and returns a new [`CameraProjectionVariable`]
    /// representing the matrix.
    fn generate_code_for_projection_matrix(
        camera_shader_input: &CameraShaderInput,
        module: &mut Module,
        bind_group_idx: &mut u32,
    ) -> CameraProjectionVariable {
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

        CameraProjectionVariable {
            projection_matrix_var,
        }
    }

    /// Generates the arguments for the required mesh vertex attributes in the
    /// main vertex shader function and begins generating the struct of output
    /// to pass from the vertex entry point to the fragment entry point.
    ///
    /// Only vertex attributes required by the material are included as input
    /// arguments.
    ///
    /// The output struct always includes the @builtin(position) field, and the
    /// expression computing this by transforming the vertex position with the
    /// model view and projection transformations is generated here. Other
    /// vertex attributes are included in the output struct as required by the
    /// material. If the vertex position or normal vector is required, this is
    /// transformed to camera space before assigned to the output struct. If the
    /// tangent space quaternion is needed, this is rotated with the model view
    /// rotation before assigned to the output struct
    ///
    /// # Returns
    /// Because the output struct may have to include material properties, its
    /// code can not be fully generated at this point. Instead, the
    /// [`OutputStructBuilder`] is returned so that the material shader
    /// generator can complete it. The indices of the included vertex attribute
    /// fields are also returned for access in the fragment shader. The function
    /// also returns the expressions for the vertex attributes passed to the
    /// vertex entry point, for access in the vertex shader.
    ///
    /// # Errors
    /// Returns an error if not all vertex attributes required by the material
    /// are available in the input mesh.
    fn generate_vertex_code_for_vertex_attributes(
        mesh_shader_input: &MeshShaderInput,
        vertex_attribute_requirements: VertexAttributeSet,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        tricks: ShaderTricks,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        vertex_function: &mut Function,
        model_view_transform: Option<&ModelViewTransformExpressions>,
        projection: Option<ProjectionExpressions>,
    ) -> Result<(
        MeshVertexInputExpressions,
        MeshVertexOutputFieldIndices,
        OutputStructBuilder,
    )> {
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

        let mut input_expressions = MeshVertexInputExpressions {
            position: input_model_position_expr,
            color: None,
            normal_vector: None,
            texture_coords: None,
            tangent_space_quaternion: None,
        };

        input_expressions.color =
            if vertex_attribute_requirements.contains(VertexAttributeSet::COLOR) {
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

        input_expressions.normal_vector = if vertex_attribute_requirements
            .contains(VertexAttributeSet::NORMAL_VECTOR)
            && !input_render_attachment_quantities
                .contains(RenderAttachmentQuantitySet::NORMAL_VECTOR)
        {
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

        input_expressions.texture_coords = if vertex_attribute_requirements
            .contains(VertexAttributeSet::TEXTURE_COORDS)
            && !input_render_attachment_quantities
                .contains(RenderAttachmentQuantitySet::TEXTURE_COORDS)
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

        input_expressions.tangent_space_quaternion = if vertex_attribute_requirements
            .contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION)
        {
            Some(Self::add_vertex_attribute_input_argument::<
                VertexTangentSpaceQuaternion<fre>,
            >(
                vertex_function,
                mesh_shader_input,
                new_name("tangentToModelSpaceRotationQuaternion"),
                vec4_type,
            )?)
        } else {
            None
        };

        let position_expr =
            model_view_transform.map_or(input_model_position_expr, |model_view_transform| {
                if tricks.contains(ShaderTricks::FOLLOW_CAMERA) {
                    source_code_lib.generate_function_call(
                        module,
                        vertex_function,
                        "transformPositionWithoutTranslation",
                        vec![
                            model_view_transform.rotation_quaternion,
                            model_view_transform.scaling_factor,
                            input_model_position_expr,
                        ],
                    )
                } else {
                    source_code_lib.generate_function_call(
                        module,
                        vertex_function,
                        "transformPosition",
                        vec![
                            model_view_transform.rotation_quaternion,
                            model_view_transform.translation_vector,
                            model_view_transform.scaling_factor,
                            input_model_position_expr,
                        ],
                    )
                }
            });

        let mut output_struct_builder = OutputStructBuilder::new("VertexOutput");

        let projected_position_expr = match projection {
            Some(projection) if !tricks.contains(ShaderTricks::NO_VERTEX_PROJECTION) => projection
                .generate_projected_position_expr(
                    module,
                    source_code_lib,
                    vertex_function,
                    tricks,
                    position_expr,
                ),
            _ => append_unity_component_to_vec3(&mut module.types, vertex_function, position_expr),
        };

        let framebuffer_position_field_idx = output_struct_builder.add_builtin_position_field(
            "projectedPosition",
            vec4_type,
            VECTOR_4_SIZE,
            projected_position_expr,
        );

        let mut output_field_indices = MeshVertexOutputFieldIndices {
            framebuffer_position: framebuffer_position_field_idx,
            position: None,
            color: None,
            normal_vector: None,
            texture_coords: None,
            tangent_space_quaternion: None,
        };

        if vertex_attribute_requirements.contains(VertexAttributeSet::POSITION) {
            output_field_indices.position = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "position",
                    vec3_type,
                    VECTOR_3_SIZE,
                    position_expr,
                ),
            );
        }

        if let Some(input_color_expr) = input_expressions.color {
            output_field_indices.color = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "color",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_color_expr,
                ),
            );
        }

        if let Some(input_model_normal_vector_expr) = input_expressions.normal_vector {
            let normal_vector_expr = model_view_transform.map_or(
                input_model_normal_vector_expr,
                |model_view_transform| {
                    source_code_lib.generate_function_call(
                        module,
                        vertex_function,
                        "rotateVectorWithQuaternion",
                        vec![
                            model_view_transform.rotation_quaternion,
                            input_model_normal_vector_expr,
                        ],
                    )
                },
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

        if let Some(input_texture_coord_expr) = input_expressions.texture_coords {
            output_field_indices.texture_coords = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "textureCoords",
                    vec2_type,
                    VECTOR_2_SIZE,
                    input_texture_coord_expr,
                ),
            );
        }

        if let Some(input_tangent_to_model_space_quaternion_expr) =
            input_expressions.tangent_space_quaternion
        {
            let tangent_space_quaternion_expr = model_view_transform.map_or(
                input_tangent_to_model_space_quaternion_expr,
                |model_view_transform| {
                    source_code_lib.generate_function_call(
                        module,
                        vertex_function,
                        "applyRotationToTangentSpaceQuaternion",
                        vec![
                            model_view_transform.rotation_quaternion,
                            input_tangent_to_model_space_quaternion_expr,
                        ],
                    )
                },
            );

            output_field_indices.tangent_space_quaternion = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "tangentToCameraSpaceQuaternion",
                    vec4_type,
                    VECTOR_4_SIZE,
                    tangent_space_quaternion_expr,
                ),
            );
        }

        Ok((
            input_expressions,
            output_field_indices,
            output_struct_builder,
        ))
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

    fn add_light_push_constants(
        light_shader_input: &LightShaderInput,
        module: &mut Module,
        push_constant_struct: &mut PushConstantStruct,
        has_material: bool,
    ) {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);

        push_constant_struct.field_indices.light_idx = Some(
            push_constant_struct
                .builder
                .add_field("activeLightIdx", u32_type, None, U32_WIDTH),
        );

        if matches!(light_shader_input, LightShaderInput::UnidirectionalLight(_)) && !has_material {
            push_constant_struct.field_indices.cascade_idx =
                Some(push_constant_struct.builder.add_field(
                    "activeCascadeIdx",
                    u32_type,
                    None,
                    U32_WIDTH,
                ));
        }
    }

    fn add_material_push_constants(
        material_shader_input: &MaterialShaderInput,
        module: &mut Module,
        push_constant_struct: &mut PushConstantStruct,
    ) {
        if matches!(material_shader_input, &MaterialShaderInput::ToneMapping(_)) {
            let f32_type = insert_in_arena(&mut module.types, F32_TYPE);
            push_constant_struct.field_indices.exposure = Some(
                push_constant_struct
                    .builder
                    .add_field("exposure", f32_type, None, F32_WIDTH),
            );
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
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        has_material: bool,
    ) -> LightShaderGenerator {
        match light_shader_input {
            LightShaderInput::AmbientLight(light_shader_input) => {
                Self::create_ambient_light_shader_generator(
                    light_shader_input,
                    module,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
                )
            }
            LightShaderInput::OmnidirectionalLight(light_shader_input) => {
                Self::create_omnidirectional_light_shader_generator(
                    light_shader_input,
                    module,
                    fragment_function,
                    bind_group_idx,
                    vertex_attribute_requirements,
                    push_constant_fragment_expressions,
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
                    push_constant_vertex_expressions,
                    push_constant_fragment_expressions,
                    has_material,
                )
            }
        }
    }

    /// Creates a generator of shader code for ambient lights.
    ///
    /// This involves generating declarations for the ambient light uniform
    /// type, the type the ambient light uniform buffer will be mapped to, the
    /// global variable this is bound to, and expressions for the fields of the
    /// light at the active index (which is set in a push constant).
    fn create_ambient_light_shader_generator(
        light_shader_input: &AmbientLightShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
    ) -> LightShaderGenerator {
        let u32_type = insert_in_arena(&mut module.types, U32_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        // The struct is padded to 16 byte alignment as required for uniforms
        let single_light_struct_size = VECTOR_4_SIZE;

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let single_light_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("AmbientLight"),
                inner: TypeInner::Struct {
                    members: vec![StructMember {
                        name: new_name("luminance"),
                        ty: vec3_type,
                        binding: None,
                        offset: 0,
                    }],
                    span: single_light_struct_size,
                },
            },
        );

        let light_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type,
                    size: ArraySize::Constant(
                        NonZeroU32::new(light_shader_input.max_light_count as u32).unwrap(),
                    ),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("AmbientLights"),
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
                name: new_name("ambientLights"),
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

        LightShaderGenerator::new_for_ambient_light_shading(
            fragment_function,
            lights_struct_var,
            push_constant_fragment_expressions,
        )
    }

    /// Creates a generator of shader code for omnidirectional lights.
    ///
    /// This involves generating declarations for the omnidirectional light
    /// uniform type, the type the omnidirectional light uniform buffer will be
    /// mapped to, the global variable this is bound to, the global variables
    /// referring to the shadow map texture and sampler if required, and
    /// expressions for the fields of the light at the active index (which is
    /// set in a push constant).
    fn create_omnidirectional_light_shader_generator(
        light_shader_input: &OmnidirectionalLightShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        vertex_attribute_requirements: &mut VertexAttributeSet,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
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
                name: new_name("OmnidirectionalLight"),
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
                            name: new_name("luminousIntensityAndEmissionRadius"),
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

        let light_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type,
                    size: ArraySize::Constant(
                        NonZeroU32::new(light_shader_input.max_light_count as u32).unwrap(),
                    ),
                    stride: single_light_struct_size,
                },
            },
        );

        let lights_struct_type = insert_in_arena(
            &mut module.types,
            Type {
                name: new_name("OmnidirectionalLights"),
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
                name: new_name("omnidirectionalLights"),
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
            // If we have a material, we will do shading that involves the
            // shadow cubemap
            let (
                shadow_map_texture_binding,
                shadow_map_sampler_binding,
                shadow_map_comparison_sampler_binding,
            ) = light_shader_input.shadow_map_texture_and_sampler_bindings;

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

            LightShaderGenerator::new_for_omnidirectional_light_shading(
                fragment_function,
                lights_struct_var,
                push_constant_fragment_expressions,
                shadow_map,
            )
        } else {
            // For updating the shadow map, we need access to the unprojected
            // cubemap space position in the fragment shader
            *vertex_attribute_requirements |= VertexAttributeSet::POSITION;

            LightShaderGenerator::new_for_omnidirectional_light_shadow_map_update(
                fragment_function,
                lights_struct_var,
                push_constant_fragment_expressions,
            )
        }
    }

    /// Creates a generator of shader code for omnidirectional lights.
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
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
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

        let orthographic_transform_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: orthographic_transform_struct_type,
                    size: ArraySize::Constant(NonZeroU32::new(MAX_SHADOW_MAP_CASCADES).unwrap()),
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
                            name: new_name("perpendicularIlluminanceAndTanAngularRadius"),
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

        let lights_array_type = insert_in_arena(
            &mut module.types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: single_light_struct_type,
                    size: ArraySize::Constant(
                        NonZeroU32::new(light_shader_input.max_light_count as u32).unwrap(),
                    ),
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
                push_constant_vertex_expressions,
                push_constant_fragment_expressions,
                shadow_map,
            )
        } else {
            LightShaderGenerator::new_for_unidirectional_light_shadow_map_update(
                vertex_function,
                lights_struct_var,
                push_constant_vertex_expressions,
            )
        }
    }
}

impl<'a> MaterialShaderGenerator<'a> {
    /// Any [`ShaderTricks`] employed by the material.
    pub fn tricks(&self) -> ShaderTricks {
        match self {
            Self::Skybox(_) => SkyboxShaderGenerator::TRICKS,
            Self::Passthrough(_) => PassthroughShaderGenerator::TRICKS,
            Self::AmbientOcclusion(_) => AmbientOcclusionShaderGenerator::TRICKS,
            Self::GaussianBlur(_) => GaussianBlurShaderGenerator::TRICKS,
            Self::ToneMapping(_) => ToneMappingShaderGenerator::TRICKS,
            _ => ShaderTricks::empty(),
        }
    }

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
        mesh_vertex_input_expressions: &MeshVertexInputExpressions,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> MaterialVertexOutputFieldIndices {
        match self {
            Self::FixedColor(generator) => {
                MaterialVertexOutputFieldIndices::FixedColor(generator.generate_vertex_code(
                    module,
                    vertex_function,
                    vertex_output_struct_builder,
                ))
            }
            Self::BlinnPhong(generator) => {
                MaterialVertexOutputFieldIndices::BlinnPhong(generator.generate_vertex_code(
                    module,
                    vertex_function,
                    vertex_output_struct_builder,
                ))
            }
            Self::Microfacet(generator) => {
                MaterialVertexOutputFieldIndices::Microfacet(generator.generate_vertex_code(
                    module,
                    vertex_function,
                    vertex_output_struct_builder,
                ))
            }
            Self::Prepass(generator) => {
                MaterialVertexOutputFieldIndices::Prepass(generator.generate_vertex_code(
                    module,
                    vertex_function,
                    vertex_output_struct_builder,
                ))
            }
            Self::Skybox(generator) => {
                MaterialVertexOutputFieldIndices::Skybox(generator.generate_vertex_code(
                    module,
                    mesh_vertex_input_expressions,
                    vertex_output_struct_builder,
                ))
            }
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
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        input_render_attachment_quantities: RenderAttachmentQuantitySet,
        output_render_attachment_quantities: RenderAttachmentQuantitySet,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        camera_projection: Option<&CameraProjectionVariable>,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &MaterialVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        match (self, material_input_field_indices) {
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
                source_code_lib,
                fragment_function,
                bind_group_idx,
                input_render_attachment_quantities,
                push_constant_fragment_expressions,
                fragment_input_struct,
                mesh_input_field_indices,
                light_input_field_indices,
                material_input_field_indices,
                light_shader_generator,
            ),
            (
                Self::Microfacet(generator),
                MaterialVertexOutputFieldIndices::Microfacet(material_input_field_indices),
            ) => generator.generate_fragment_code(
                module,
                source_code_lib,
                fragment_function,
                bind_group_idx,
                input_render_attachment_quantities,
                push_constant_fragment_expressions,
                fragment_input_struct,
                mesh_input_field_indices,
                light_input_field_indices,
                material_input_field_indices,
                light_shader_generator,
            ),
            (
                Self::Prepass(generator),
                MaterialVertexOutputFieldIndices::Prepass(material_input_field_indices),
            ) => {
                generator.generate_fragment_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    output_render_attachment_quantities,
                    fragment_input_struct,
                    mesh_input_field_indices,
                    material_input_field_indices,
                    light_shader_generator,
                );
            }
            (
                Self::Skybox(generator),
                MaterialVertexOutputFieldIndices::Skybox(material_input_field_indices),
            ) => {
                generator.generate_fragment_code(
                    module,
                    fragment_function,
                    bind_group_idx,
                    fragment_input_struct,
                    material_input_field_indices,
                );
            }
            (Self::Passthrough(generator), MaterialVertexOutputFieldIndices::None) => {
                generator.generate_fragment_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (Self::AmbientOcclusion(generator), MaterialVertexOutputFieldIndices::None) => {
                generator.generate_fragment_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
                    camera_projection,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (Self::GaussianBlur(generator), MaterialVertexOutputFieldIndices::None) => {
                generator.generate_fragment_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (Self::ToneMapping(generator), MaterialVertexOutputFieldIndices::None) => {
                generator.generate_fragment_code(
                    module,
                    source_code_lib,
                    fragment_function,
                    bind_group_idx,
                    push_constant_fragment_expressions,
                    fragment_input_struct,
                    mesh_input_field_indices,
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
    pub fn generate_projected_position_expr(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        vertex_function: &mut Function,
        tricks: ShaderTricks,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        match self {
            Self::Camera(camera_projection_matrix) => camera_projection_matrix
                .generate_projected_position_expr(module, vertex_function, tricks, position_expr),
            Self::OmnidirectionalLight(omnidirectional_light_cubemap_projection) => {
                omnidirectional_light_cubemap_projection.generate_projected_position_expr(
                    module,
                    source_code_lib,
                    vertex_function,
                    position_expr,
                )
            }
            Self::UnidirectionalLight(unidirectional_light_orthographic_projection) => {
                unidirectional_light_orthographic_projection.generate_projected_position_expr(
                    module,
                    source_code_lib,
                    vertex_function,
                    position_expr,
                )
            }
        }
    }
}

impl CameraProjectionVariable {
    /// Generates the expression for the projection matrix in the given
    /// function.
    pub fn generate_projection_matrix_expr(&self, function: &mut Function) -> Handle<Expression> {
        let projection_matrix_ptr_expr = include_expr_in_func(
            function,
            Expression::GlobalVariable(self.projection_matrix_var),
        );

        let matrix_expr = emit_in_func(function, |function| {
            include_named_expr_in_func(
                function,
                "projectionMatrix",
                Expression::Load {
                    pointer: projection_matrix_ptr_expr,
                },
            )
        });

        matrix_expr
    }

    /// Generates an expression for the given position (as a vec3) projected
    /// with the projection matrix in the vertex entry point function. The
    /// projected position will be a vec4.
    pub fn generate_projected_position_expr(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        tricks: ShaderTricks,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let matrix_expr = self.generate_projection_matrix_expr(vertex_function);

        let homogeneous_position_expr =
            append_unity_component_to_vec3(&mut module.types, vertex_function, position_expr);

        emit_in_func(vertex_function, |function| {
            let projected_position_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: matrix_expr,
                    right: homogeneous_position_expr,
                },
            );

            if tricks.contains(ShaderTricks::DRAW_AT_MAX_DEPTH) {
                include_expr_in_func(
                    function,
                    Expression::Swizzle {
                        size: VectorSize::Quad,
                        vector: projected_position_expr,
                        pattern: [
                            SwizzleComponent::X,
                            SwizzleComponent::Y,
                            SwizzleComponent::W,
                            SwizzleComponent::W,
                        ],
                    },
                )
            } else {
                projected_position_expr
            }
        })
    }
}

impl OmnidirectionalLightProjectionExpressions {
    #[allow(clippy::unused_self)]
    pub fn generate_projected_position_expr(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        source_code_lib.generate_function_call(
            module,
            vertex_function,
            "applyCubemapFaceProjectionToPosition",
            vec![position_expr],
        )
    }
}

impl UnidirectionalLightProjectionExpressions {
    /// Generates an expression for the given position (as a vec3) projected
    /// with the orthographic projection in the vertex entry point function. The
    /// projected position will be a vec4 with w = 1.0;
    pub fn generate_projected_position_expr(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        vertex_function: &mut Function,
        position_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let light_clip_space_position_expr = source_code_lib.generate_function_call(
            module,
            vertex_function,
            "applyOrthographicProjectionToPosition",
            vec![self.translation, self.scaling, position_expr],
        );

        append_unity_component_to_vec3(
            &mut module.types,
            vertex_function,
            light_clip_space_position_expr,
        )
    }
}

impl LightShaderGenerator {
    pub fn new_for_ambient_light_shading(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        Self::AmbientLight(AmbientLightShaderGenerator::new(
            fragment_function,
            lights_struct_var,
            push_constant_fragment_expressions,
        ))
    }

    pub fn new_for_omnidirectional_light_shadow_map_update(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        Self::OmnidirectionalLight(OmnidirectionalLightShaderGenerator::ForShadowMapUpdate(
            OmnidirectionalLightShadowMapUpdateShaderGenerator::new(
                fragment_function,
                lights_struct_var,
                push_constant_fragment_expressions,
            ),
        ))
    }

    pub fn new_for_omnidirectional_light_shading(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        shadow_map: SampledTexture,
    ) -> Self {
        Self::OmnidirectionalLight(OmnidirectionalLightShaderGenerator::ForShading(
            OmnidirectionalLightShadingShaderGenerator::new(
                fragment_function,
                lights_struct_var,
                push_constant_fragment_expressions,
                shadow_map,
            ),
        ))
    }

    pub fn new_for_unidirectional_light_shadow_map_update(
        vertex_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShadowMapUpdate(
            UnidirectionalLightShadowMapUpdateShaderGenerator::new(
                vertex_function,
                lights_struct_var,
                push_constant_vertex_expressions,
            ),
        ))
    }

    pub fn new_for_unidirectional_light_shading(
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        shadow_map: SampledTexture,
    ) -> Self {
        Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShading(
            UnidirectionalLightShadingShaderGenerator::new(
                vertex_function,
                fragment_function,
                lights_struct_var,
                push_constant_vertex_expressions,
                push_constant_fragment_expressions,
                shadow_map,
            ),
        ))
    }

    pub fn get_projection_to_light_clip_space(&self) -> Option<ProjectionExpressions> {
        match self {
            Self::OmnidirectionalLight(_) => Some(ProjectionExpressions::OmnidirectionalLight(
                OmnidirectionalLightProjectionExpressions,
            )),
            Self::UnidirectionalLight(UnidirectionalLightShaderGenerator::ForShadowMapUpdate(
                shader_generator,
            )) => Some(shader_generator.get_projection_to_light_clip_space()),
            Self::UnidirectionalLight(_) | Self::AmbientLight(_) => None,
        }
    }

    pub fn generate_vertex_output_code_for_shading(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
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
                    source_code_lib,
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
            Self::OmnidirectionalLight(OmnidirectionalLightShaderGenerator::ForShadowMapUpdate(_))
        )
    }

    pub fn generate_fragment_output_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        if let Self::OmnidirectionalLight(
            OmnidirectionalLightShaderGenerator::ForShadowMapUpdate(
                shadow_map_update_shader_generator,
            ),
        ) = self
        {
            shadow_map_update_shader_generator.generate_fragment_output_code(
                module,
                source_code_lib,
                fragment_function,
                fragment_input_struct,
                mesh_input_field_indices,
            );
        }
    }

    fn generate_active_light_ptr_expr(
        function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_expressions: &PushConstantFieldExpressions,
    ) -> Handle<Expression> {
        let lights_struct_ptr_expr =
            include_expr_in_func(function, Expression::GlobalVariable(lights_struct_var));

        Self::generate_single_light_ptr_expr(
            function,
            lights_struct_ptr_expr,
            push_constant_expressions
                .light_idx
                .expect("Missing light index push constant"),
        )
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

impl AmbientLightShaderGenerator {
    pub fn new(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        let active_light_ptr_expr = LightShaderGenerator::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var,
            push_constant_fragment_expressions,
        );

        let luminance = LightShaderGenerator::generate_named_field_access_expr(
            fragment_function,
            "lightLuminance",
            active_light_ptr_expr,
            0,
        );

        Self { luminance }
    }
}

impl OmnidirectionalLightShadowMapUpdateShaderGenerator {
    pub fn new(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        let active_light_ptr_expr = LightShaderGenerator::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var,
            push_constant_fragment_expressions,
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
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let f32_type = insert_in_arena(&mut module.types, F32_TYPE);

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for omnidirectional light shadow map update"),
        );

        let depth = source_code_lib.generate_function_call(
            module,
            fragment_function,
            "computeShadowMapFragmentDepthOmniLight",
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

impl OmnidirectionalLightShadingShaderGenerator {
    pub fn new(
        fragment_function: &mut Function,
        lights_struct_var: Handle<GlobalVariable>,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        shadow_map: SampledTexture,
    ) -> Self {
        let active_light_ptr_expr = LightShaderGenerator::generate_active_light_ptr_expr(
            fragment_function,
            lights_struct_var,
            push_constant_fragment_expressions,
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

        let luminous_intensity_and_emission_radius =
            LightShaderGenerator::generate_named_field_access_expr(
                fragment_function,
                "lightLuminousIntensityAndEmissionRadius",
                active_light_ptr_expr,
                2,
            );

        let (luminous_intensity, emission_radius) = emit_in_func(fragment_function, |function| {
            (
                include_expr_in_func(
                    function,
                    swizzle_xyz_expr(luminous_intensity_and_emission_radius),
                ),
                include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: luminous_intensity_and_emission_radius,
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
            luminous_intensity,
            emission_radius,
            near_distance,
            inverse_distance_span,
            shadow_map,
        }
    }

    pub fn generate_fragment_shading_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        framebuffer_position_expr: Handle<Expression>,
        position_expr: Handle<Expression>,
        normal_vector_expr: Handle<Expression>,
        view_dir_expr: Handle<Expression>,
        roughness_expr: Option<Handle<Expression>>,
        emulate_area_light_reflection: bool,
    ) -> (Handle<Expression>, Handle<Expression>) {
        source_code_lib.use_type(module, "OmniLightQuantities");

        let light_quantities = if emulate_area_light_reflection {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeOmniAreaLightQuantities",
                vec![
                    self.camera_space_position,
                    self.luminous_intensity,
                    self.emission_radius,
                    self.camera_to_light_space_rotation_quaternion,
                    self.near_distance,
                    self.inverse_distance_span,
                    position_expr,
                    normal_vector_expr,
                    view_dir_expr,
                    roughness_expr.expect(
                        "Missing roughness for omnidirectional area light luminance modification",
                    ),
                ],
            )
        } else {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeOmniLightQuantities",
                vec![
                    self.camera_space_position,
                    self.luminous_intensity,
                    self.camera_to_light_space_rotation_quaternion,
                    self.near_distance,
                    self.inverse_distance_span,
                    position_expr,
                    normal_vector_expr,
                    view_dir_expr,
                ],
            )
        };

        let (light_space_fragment_displacement_expr, depth_reference_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 1,
                        },
                    ),
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 2,
                        },
                    ),
                )
            });

        let light_access_factor_expr = self
            .shadow_map
            .generate_light_access_factor_expr_for_shadow_cubemap(
                module,
                source_code_lib,
                fragment_function,
                self.emission_radius,
                framebuffer_position_expr,
                light_space_fragment_displacement_expr,
                depth_reference_expr,
            );

        emit_in_func(fragment_function, |function| {
            let reflection_dot_products_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_quantities,
                    index: 3,
                },
            );

            let incident_luminance_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_quantities,
                    index: 0,
                },
            );

            let shadow_masked_incident_luminance_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: light_access_factor_expr,
                    right: incident_luminance_expr,
                },
            );

            (
                reflection_dot_products_expr,
                shadow_masked_incident_luminance_expr,
            )
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
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
    ) -> Self {
        let lights_struct_ptr_expr = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(lights_struct_var),
        );

        let active_light_ptr_expr = LightShaderGenerator::generate_single_light_ptr_expr(
            vertex_function,
            lights_struct_ptr_expr,
            push_constant_vertex_expressions
                .light_idx
                .expect("Missing light index push constant"),
        );

        let orthographic_transform_ptr_expr =
            UnidirectionalLightShaderGenerator::generate_single_orthographic_transform_ptr_expr(
                vertex_function,
                active_light_ptr_expr,
                push_constant_vertex_expressions
                    .cascade_idx
                    .expect("Missing cascade index push constant"),
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
        push_constant_vertex_expressions: &PushConstantFieldExpressions,
        push_constant_fragment_expressions: &PushConstantFieldExpressions,
        shadow_map: SampledTexture,
    ) -> Self {
        let active_light_ptr_expr_in_vertex_function =
            LightShaderGenerator::generate_active_light_ptr_expr(
                vertex_function,
                lights_struct_var,
                push_constant_vertex_expressions,
            );

        let active_light_ptr_expr_in_fragment_function =
            LightShaderGenerator::generate_active_light_ptr_expr(
                fragment_function,
                lights_struct_var,
                push_constant_fragment_expressions,
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
        source_code_lib: &mut SourceCode,
        vertex_function: &mut Function,
        output_struct_builder: &mut OutputStructBuilder,
        mesh_output_field_indices: &MeshVertexOutputFieldIndices,
    ) -> UnidirectionalLightVertexOutputFieldIndices {
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let camera_to_light_space_rotation_quaternion_expr =
            LightShaderGenerator::generate_named_field_access_expr(
                vertex_function,
                "cameraToLightSpaceRotationQuaternion",
                self.active_light_ptr_expr_in_vertex_function,
                0,
            );

        let camera_space_position_expr = output_struct_builder
            .get_field_expr(
                mesh_output_field_indices
                    .position
                    .expect("Missing position for shading with unidirectional light"),
            )
            .unwrap();

        let light_space_position_expr = source_code_lib.generate_function_call(
            module,
            vertex_function,
            "rotateVectorWithQuaternion",
            vec![
                camera_to_light_space_rotation_quaternion_expr,
                camera_space_position_expr,
            ],
        );

        let light_space_normal_vector_expr =
            mesh_output_field_indices
                .normal_vector
                .map(|normal_vector_idx| {
                    let camera_space_normal_vector_expr = output_struct_builder
                        .get_field_expr(normal_vector_idx)
                        .unwrap();

                    source_code_lib.generate_function_call(
                        module,
                        vertex_function,
                        "rotateVectorWithQuaternion",
                        vec![
                            camera_to_light_space_rotation_quaternion_expr,
                            camera_space_normal_vector_expr,
                        ],
                    )
                });

        UnidirectionalLightVertexOutputFieldIndices {
            light_space_position: output_struct_builder.add_field_with_perspective_interpolation(
                "lightSpacePosition",
                vec3_type,
                VECTOR_3_SIZE,
                light_space_position_expr,
            ),
            light_space_normal_vector: light_space_normal_vector_expr.map(
                |light_space_normal_vector_expr| {
                    output_struct_builder.add_field_with_perspective_interpolation(
                        "lightSpaceNormalVector",
                        vec3_type,
                        VECTOR_3_SIZE,
                        light_space_normal_vector_expr,
                    )
                },
            ),
        }
    }

    pub fn generate_fragment_shading_code(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        light_input_field_indices: &UnidirectionalLightVertexOutputFieldIndices,
        framebuffer_position_expr: Handle<Expression>,
        camera_space_normal_vector_expr: Handle<Expression>,
        camera_space_view_dir_expr: Handle<Expression>,
        roughness_expr: Option<Handle<Expression>>,
        emulate_area_light_reflection: bool,
    ) -> (Handle<Expression>, Handle<Expression>) {
        let camera_space_direction_of_light_expr =
            LightShaderGenerator::generate_named_field_access_expr(
                fragment_function,
                "cameraSpaceLightDirection",
                self.active_light_ptr_expr_in_fragment_function,
                1,
            );

        let perpendicular_illuminance_and_tan_angular_radius_expr =
            LightShaderGenerator::generate_named_field_access_expr(
                fragment_function,
                "lightPerpendicularIlluminanceAndTanAngularRadius",
                self.active_light_ptr_expr_in_fragment_function,
                2,
            );

        let (perpendicular_illuminance_expr, tan_angular_radius_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        swizzle_xyz_expr(perpendicular_illuminance_and_tan_angular_radius_expr),
                    ),
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: perpendicular_illuminance_and_tan_angular_radius_expr,
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

        let cascade_idx_expr = source_code_lib.generate_function_call(
            module,
            fragment_function,
            &format!("determineCascadeIdxMax{}", MAX_SHADOW_MAP_CASCADES),
            vec![partition_depths_expr, framebuffer_position_expr],
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

        let light_space_position_expr =
            fragment_input_struct.get_field_expr(light_input_field_indices.light_space_position);

        let light_space_normal_vector_expr = light_input_field_indices
            .light_space_normal_vector
            .map_or_else(
                || {
                    let camera_to_light_space_rotation_quaternion_expr =
                        LightShaderGenerator::generate_named_field_access_expr(
                            fragment_function,
                            "cameraToLightSpaceRotationQuaternion",
                            self.active_light_ptr_expr_in_fragment_function,
                            0,
                        );

                    source_code_lib.generate_function_call(
                        module,
                        fragment_function,
                        "rotateVectorWithQuaternion",
                        vec![
                            camera_to_light_space_rotation_quaternion_expr,
                            camera_space_normal_vector_expr,
                        ],
                    )
                },
                |light_space_normal_vector_idx| {
                    fragment_input_struct.get_field_expr(light_space_normal_vector_idx)
                },
            );

        let light_quantities = if emulate_area_light_reflection {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeUniAreaLightQuantities",
                vec![
                    camera_space_direction_of_light_expr,
                    perpendicular_illuminance_expr,
                    tan_angular_radius_expr,
                    orthographic_translation_expr,
                    orthographic_scaling_expr,
                    light_space_position_expr,
                    light_space_normal_vector_expr,
                    camera_space_normal_vector_expr,
                    camera_space_view_dir_expr,
                    roughness_expr.expect(
                        "Missing roughness for omnidirectional area light luminance modification",
                    ),
                ],
            )
        } else {
            source_code_lib.generate_function_call(
                module,
                fragment_function,
                "computeUniLightQuantities",
                vec![
                    camera_space_direction_of_light_expr,
                    perpendicular_illuminance_expr,
                    orthographic_translation_expr,
                    orthographic_scaling_expr,
                    light_space_position_expr,
                    light_space_normal_vector_expr,
                    camera_space_normal_vector_expr,
                    camera_space_view_dir_expr,
                ],
            )
        };

        let (incident_luminance_expr, light_clip_position_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 0,
                        },
                    ),
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 1,
                        },
                    ),
                )
            });

        let light_access_factor_expr = self
            .shadow_map
            .generate_light_access_factor_expr_for_cascaded_shadow_map(
                module,
                source_code_lib,
                fragment_function,
                tan_angular_radius_expr,
                world_to_light_clip_space_xy_scale_expr,
                world_to_light_clip_space_z_scale_expr,
                framebuffer_position_expr,
                light_clip_position_expr,
                cascade_idx_expr,
            );

        let (reflection_dot_products_expr, shadow_masked_incident_luminance_expr) =
            emit_in_func(fragment_function, |function| {
                (
                    include_expr_in_func(
                        function,
                        Expression::AccessIndex {
                            base: light_quantities,
                            index: 2,
                        },
                    ),
                    include_expr_in_func(
                        function,
                        Expression::Binary {
                            op: BinaryOperator::Multiply,
                            left: light_access_factor_expr,
                            right: incident_luminance_expr,
                        },
                    ),
                )
            });

        (
            reflection_dot_products_expr,
            shadow_masked_incident_luminance_expr,
        )
    }
}

impl PushConstantStruct {
    fn new(module: &mut Module) -> Self {
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);

        let mut builder = StructBuilder::new("PushConstants");

        let inverse_window_dimensions_idx =
            builder.add_field("inverseWindowDimensions", vec2_type, None, VECTOR_2_SIZE);

        let field_indices = PushConstantFieldIndices {
            inverse_window_dimensions: inverse_window_dimensions_idx,
            light_idx: None,
            cascade_idx: None,
            exposure: None,
        };

        Self {
            builder,
            field_indices,
            global_variable: None,
        }
    }

    fn generate_expressions(
        &mut self,
        module: &mut Module,
        function: &mut Function,
    ) -> PushConstantFieldExpressions {
        let struct_var_ptr_expr = self.global_variable.get_or_insert_with(|| {
            let struct_type = insert_in_arena(&mut module.types, self.builder.clone().into_type());

            append_to_arena(
                &mut module.global_variables,
                GlobalVariable {
                    name: new_name("pushConstants"),
                    space: AddressSpace::PushConstant,
                    binding: None,
                    ty: struct_type,
                    init: None,
                },
            )
        });

        let struct_ptr_expr =
            include_expr_in_func(function, Expression::GlobalVariable(*struct_var_ptr_expr));

        let inverse_window_dimensions_expr = emit_in_func(function, |function| {
            let inverse_window_dimensions_ptr_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: struct_ptr_expr,
                    index: self.field_indices.inverse_window_dimensions as u32,
                },
            );
            include_expr_in_func(
                function,
                Expression::Load {
                    pointer: inverse_window_dimensions_ptr_expr,
                },
            )
        });

        let mut field_expressions = PushConstantFieldExpressions {
            inverse_window_dimensions: inverse_window_dimensions_expr,
            light_idx: None,
            cascade_idx: None,
            exposure: None,
        };

        if let Some(light_idx) = self.field_indices.light_idx {
            field_expressions.light_idx = Some(emit_in_func(function, |function| {
                let light_idx_ptr_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: struct_ptr_expr,
                        index: light_idx as u32,
                    },
                );
                include_expr_in_func(
                    function,
                    Expression::Load {
                        pointer: light_idx_ptr_expr,
                    },
                )
            }));
        }

        if let Some(cascade_idx) = self.field_indices.cascade_idx {
            field_expressions.cascade_idx = Some(emit_in_func(function, |function| {
                let cascade_idx_ptr_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: struct_ptr_expr,
                        index: cascade_idx as u32,
                    },
                );
                include_expr_in_func(
                    function,
                    Expression::Load {
                        pointer: cascade_idx_ptr_expr,
                    },
                )
            }));
        }

        if let Some(exposure) = self.field_indices.exposure {
            field_expressions.exposure = Some(emit_in_func(function, |function| {
                let exposure_ptr_expr = include_expr_in_func(
                    function,
                    Expression::AccessIndex {
                        base: struct_ptr_expr,
                        index: exposure as u32,
                    },
                );
                include_expr_in_func(
                    function,
                    Expression::Load {
                        pointer: exposure_ptr_expr,
                    },
                )
            }));
        }

        field_expressions
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
                second_blend_source: false,
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
                second_blend_source: false,
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

impl ForLoop {
    /// Generates code for a new for-loop with the start index given by
    /// `start_expr` (zero is used if `start_expr` is `None`) and end index
    /// given by `end_expr` and returns a new [`ForLoop`]. The loop index starts
    /// at zero, and is available as the `idx_expr` field of the returned
    /// `ForLoop` struct. The main body of the loop is empty, and statements can
    /// be added to it by pushing to the `body` field of the returned `ForLoop`.
    pub fn new(
        types: &mut UniqueArena<Type>,
        function: &mut Function,
        name: &str,
        start_expr: Option<Handle<Expression>>,
        end_expr: Handle<Expression>,
    ) -> Self {
        let u32_type = insert_in_arena(types, U32_TYPE);

        let start_expr = start_expr.unwrap_or_else(|| {
            append_to_arena(
                &mut function.expressions,
                Expression::Literal(Literal::U32(0)),
            )
        });

        let idx_ptr_expr = append_to_arena(
            &mut function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut function.local_variables,
                LocalVariable {
                    name: Some(format!("{}Idx", name)),
                    ty: u32_type,
                    init: Some(start_expr),
                },
            )),
        );

        let mut body_block = Block::new();

        let idx_expr = emit(&mut body_block, &mut function.expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Load {
                    pointer: idx_ptr_expr,
                },
            )
        });

        let mut continuing_block = Block::new();

        let unity_constant_expr =
            include_expr_in_func(function, Expression::Literal(Literal::U32(1)));

        let incremented_idx_expr = emit(
            &mut continuing_block,
            &mut function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: idx_expr,
                        right: unity_constant_expr,
                    },
                )
            },
        );

        push_to_block(
            &mut continuing_block,
            Statement::Store {
                pointer: idx_ptr_expr,
                value: incremented_idx_expr,
            },
        );

        let break_if_expr = emit(
            &mut continuing_block,
            &mut function.expressions,
            |expressions| {
                let idx_expr = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: idx_ptr_expr,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::GreaterEqual,
                        left: idx_expr,
                        right: end_expr,
                    },
                )
            },
        );

        Self {
            body: body_block,
            continuing: continuing_block,
            break_if: Some(break_if_expr),
            idx_expr,
            start_expr,
            end_expr,
        }
    }

    /// Generates the actual loop statement. Call this when the `body` field has
    /// been filled with all required statements.
    pub fn generate_code(self, block: &mut Block, expressions: &mut Arena<Expression>) {
        let mut loop_block = Block::new();

        push_to_block(
            &mut loop_block,
            Statement::Loop {
                body: self.body,
                continuing: self.continuing,
                break_if: self.break_if,
            },
        );

        let n_iter_above_zero_expr = emit(block, expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Binary {
                    op: BinaryOperator::Greater,
                    left: self.end_expr,
                    right: self.start_expr,
                },
            )
        });

        push_to_block(
            block,
            Statement::If {
                condition: n_iter_above_zero_expr,
                accept: loop_block,
                reject: Block::new(),
            },
        );
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

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression, and extracting the RGB
    /// values of the sampled RGBA color.
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

    /// Generates and returns an expression sampling the texture at the texture
    /// coordinates specified by the given expression, and extracting the RG
    /// values of the sampled RGBA color.
    pub fn generate_rg_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, None, None, None);

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
        channel_index: u32,
    ) -> Handle<Expression> {
        let sampling_expr =
            self.generate_sampling_expr(function, texture_coord_expr, None, None, None);

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

    /// Generates and returns an expression for the fraction of light reaching
    /// the fragment based on sampling of the specified shadow map cascade
    /// around the texture coordinates converted from the x- and y-component of
    /// the given light clip space position, using the z-component as the
    /// reference depth.
    pub fn generate_light_access_factor_expr_for_shadow_cubemap(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        function: &mut Function,
        emission_radius_expr: Handle<Expression>,
        framebuffer_position_expr: Handle<Expression>,
        light_space_fragment_displacement_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        self.generate_pcss_light_access_factor_expr_for_shadow_cubemap(
            module,
            source_code_lib,
            function,
            emission_radius_expr,
            framebuffer_position_expr,
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
        source_code_lib: &mut SourceCode,
        function: &mut Function,
        tan_angular_radius_expr: Handle<Expression>,
        world_to_light_clip_space_xy_scale_expr: Handle<Expression>,
        world_to_light_clip_space_z_scale_expr: Handle<Expression>,
        framebuffer_position_expr: Handle<Expression>,
        light_clip_position_expr: Handle<Expression>,
        cascade_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
        let vec2_type = insert_in_arena(&mut module.types, VECTOR_2_TYPE);

        let unity_constant_expr =
            include_expr_in_func(function, Expression::Literal(Literal::F32(1.0)));

        let half_constant_expr =
            include_expr_in_func(function, Expression::Literal(Literal::F32(0.5)));

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
            source_code_lib,
            function,
            tan_angular_radius_expr,
            world_to_light_clip_space_xy_scale_expr,
            world_to_light_clip_space_z_scale_expr,
            framebuffer_position_expr,
            texture_coord_expr,
            depth_reference_expr,
            cascade_idx_expr,
        )
    }

    fn generate_pcss_light_access_factor_expr_for_shadow_cubemap(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        function: &mut Function,
        emission_radius_expr: Handle<Expression>,
        framebuffer_position_expr: Handle<Expression>,
        light_space_fragment_displacement_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
    ) -> Handle<Expression> {
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

        source_code_lib.generate_function_call(
            module,
            function,
            "computePCSSLightAccessFactorOmniLight",
            vec![
                texture_var_expr,
                sampler_var_expr,
                comparison_sampler_var_expr,
                emission_radius_expr,
                framebuffer_position_expr,
                light_space_fragment_displacement_expr,
                depth_reference_expr,
            ],
        )
    }

    fn generate_pcss_light_access_factor_expr_for_cascaded_shadow_map(
        &self,
        module: &mut Module,
        source_code_lib: &mut SourceCode,
        function: &mut Function,
        tan_angular_radius_expr: Handle<Expression>,
        world_to_light_clip_space_xy_scale_expr: Handle<Expression>,
        world_to_light_clip_space_z_scale_expr: Handle<Expression>,
        framebuffer_position_expr: Handle<Expression>,
        texture_coord_expr: Handle<Expression>,
        depth_reference_expr: Handle<Expression>,
        array_idx_expr: Handle<Expression>,
    ) -> Handle<Expression> {
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

        source_code_lib.generate_function_call(
            module,
            function,
            "computePCSSLightAccessFactorUniLight",
            vec![
                texture_var_expr,
                sampler_var_expr,
                comparison_sampler_var_expr,
                array_idx_expr,
                tan_angular_radius_expr,
                world_to_light_clip_space_xy_scale_expr,
                world_to_light_clip_space_z_scale_expr,
                framebuffer_position_expr,
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

/// Executes the given closure that adds [`Expression`]s to the given [`Arena`]
/// before pushing to the given [`Block`] a [`Statement::Emit`] emitting the
/// range of added expressions.
///
/// # Returns
/// The value returned from the closure.
fn emit<T>(
    block: &mut Block,
    arena: &mut Arena<Expression>,
    add_expressions: impl FnOnce(&mut Arena<Expression>) -> T,
) -> T {
    let start_length = arena.len();
    let ret = add_expressions(arena);
    push_to_block(block, Statement::Emit(arena.range_from(start_length)));
    ret
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
    let vec4_type = insert_in_arena(types, VECTOR_4_TYPE);

    let unity_constant_expr =
        include_expr_in_func(function, Expression::Literal(Literal::F32(1.0)));

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

// Ignore tests if running with Miri, since `naga::front::wgsl::parse_str`
// becomes extremely slow
#[cfg(test)]
// #[cfg(not(miri))]
#[allow(clippy::dbg_macro)]
mod test {
    use super::*;
    use crate::scene::{
        FixedColorMaterial, FixedTextureMaterial, GaussianBlurDirection, ToneMapping,
        VertexColorMaterial,
    };
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
        locations: [Some(MESH_VERTEX_BINDING_START), None, None, None, None],
    };

    const FIXED_COLOR_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
            color_location: MATERIAL_VERTEX_BINDING_START,
        });

    const FIXED_TEXTURE_INPUT: MaterialShaderInput =
        MaterialShaderInput::Fixed(Some(FixedTextureShaderInput {
            color_texture_and_sampler_bindings: (0, 1),
        }));

    const AMBIENT_LIGHT_INPUT: LightShaderInput =
        LightShaderInput::AmbientLight(AmbientLightShaderInput {
            uniform_binding: 8,
            max_light_count: 20,
        });

    const OMNIDIRECTIONAL_LIGHT_INPUT: LightShaderInput =
        LightShaderInput::OmnidirectionalLight(OmnidirectionalLightShaderInput {
            uniform_binding: 0,
            max_light_count: 20,
            shadow_map_texture_and_sampler_bindings: (1, 2, 3),
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
                eprintln!("{}", err.emit_to_string("test"));
                panic!("Shader validation failed")
            }
        }
    }

    #[test]
    fn parse() {
        match wgsl_in::parse_str("") {
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
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_omnidirectional_light_shadow_map_update_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
                    None,
                ],
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&MaterialShaderInput::VertexColor),
            VertexColorMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
            FixedColorMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
                    None,
                ],
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&FIXED_TEXTURE_INPUT),
            FixedTextureMaterial::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_diffuse_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_diffuse_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_diffuse_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_diffuse_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_diffuse_uniform_specular_blinn_phong_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_diffuse_uniform_specular_blinn_phong_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                },
            )),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_diffuse_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                },
            )),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_diffuse_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::BlinnPhong(
                BlinnPhongTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                },
            )),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_lambertian_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_lambertian_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works()
    {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_diffuse_microfacet_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_diffuse_microfacet_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::NO_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::NO_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: None,
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_lambertian_diffuse_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_lambertian_diffuse_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_ggx_diffuse_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_ggx_diffuse_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: None,
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_lambertian_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_lambertian_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_ggx_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_textured_ggx_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Microfacet((
                MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                MicrofacetTextureShaderInput {
                    albedo_texture_and_sampler_bindings: Some((0, 1)),
                    specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                    roughness_texture_and_sampler_bindings: None,
                },
            ))),
            VertexAttributeSet::FOR_TEXTURED_LIGHT_SHADING,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
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
    fn building_minimal_prepass_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            None,
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: None,
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: None,
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_prepass_shader_with_emissive_luminance_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            None,
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    roughness_location: None,
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: None,
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_prepass_shader_with_normal_mapping_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            None,
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: None,
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: None,
                bump_mapping_input: Some(BumpMappingTextureShaderInput::NormalMapping(
                    NormalMappingShaderInput {
                        normal_map_texture_and_sampler_bindings: (0, 1),
                    },
                )),
            })),
            VertexAttributeSet::POSITION
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_prepass_shader_with_parallax_mapping_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            None,
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: None,
                    parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: None,
                bump_mapping_input: Some(BumpMappingTextureShaderInput::ParallaxMapping(
                    ParallaxMappingShaderInput {
                        height_map_texture_and_sampler_bindings: (0, 1),
                    },
                )),
            })),
            VertexAttributeSet::POSITION
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::TEXTURE_COORDS
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_lambertian_diffuse_prepass_shader_with_ambient_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: None,
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: None,
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_microfacet_specular_prepass_shader_with_ambient_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: Some((0, 1)),
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    None,
                    None,
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: Some(MATERIAL_VERTEX_BINDING_START),
                    specular_reflectance_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: None,
                specular_reflectance_texture_and_sampler_bindings: None,
                roughness_texture_and_sampler_bindings: None,
                specular_reflectance_lookup_texture_and_sampler_bindings: Some((0, 1)),
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION | VertexAttributeSet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                    None,
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: Some((0, 1)),
                specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                roughness_texture_and_sampler_bindings: Some((4, 5)),
                specular_reflectance_lookup_texture_and_sampler_bindings: Some((6, 7)),
                bump_mapping_input: None,
            })),
            VertexAttributeSet::POSITION
                | VertexAttributeSet::NORMAL_VECTOR
                | VertexAttributeSet::TEXTURE_COORDS,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_and_normal_mapping_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: None,
                    parallax_uv_per_distance_location: None,
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: Some((0, 1)),
                specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                roughness_texture_and_sampler_bindings: Some((4, 5)),
                specular_reflectance_lookup_texture_and_sampler_bindings: Some((6, 7)),
                bump_mapping_input: Some(BumpMappingTextureShaderInput::NormalMapping(
                    NormalMappingShaderInput {
                        normal_map_texture_and_sampler_bindings: (8, 9),
                    },
                )),
            })),
            VertexAttributeSet::POSITION
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_and_parallax_mapping_works(
    ) {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                locations: [
                    Some(MESH_VERTEX_BINDING_START),
                    None,
                    None,
                    Some(MESH_VERTEX_BINDING_START + 1),
                    Some(MESH_VERTEX_BINDING_START + 2),
                ],
            }),
            Some(&AMBIENT_LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &InstanceFeatureShaderInput::LightMaterial(LightMaterialFeatureShaderInput {
                    albedo_location: None,
                    specular_reflectance_location: None,
                    emissive_luminance_location: None,
                    roughness_location: Some(MATERIAL_VERTEX_BINDING_START),
                    parallax_displacement_scale_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
                    parallax_uv_per_distance_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
                }),
            ],
            Some(&MaterialShaderInput::Prepass(PrepassTextureShaderInput {
                albedo_texture_and_sampler_bindings: Some((0, 1)),
                specular_reflectance_texture_and_sampler_bindings: Some((2, 3)),
                roughness_texture_and_sampler_bindings: Some((4, 5)),
                specular_reflectance_lookup_texture_and_sampler_bindings: Some((6, 7)),
                bump_mapping_input: Some(BumpMappingTextureShaderInput::ParallaxMapping(
                    ParallaxMappingShaderInput {
                        height_map_texture_and_sampler_bindings: (8, 9),
                    },
                )),
            })),
            VertexAttributeSet::POSITION
                | VertexAttributeSet::TEXTURE_COORDS
                | VertexAttributeSet::TANGENT_SPACE_QUATERNION,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::TEXTURE_COORDS
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
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
    fn building_passthrough_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::Passthrough(PassthroughShaderInput {
                input_texture_and_sampler_bindings: (0, 1),
            })),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_ambient_occlusion_computation_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::AmbientOcclusion(
                AmbientOcclusionShaderInput::Calculation(AmbientOcclusionCalculationShaderInput {
                    sample_uniform_binding: 0,
                }),
            )),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::POSITION | RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::OCCLUSION,
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
    fn building_ambient_occlusion_application_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::AmbientOcclusion(
                AmbientOcclusionShaderInput::Application,
            )),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE
                | RenderAttachmentQuantitySet::OCCLUSION,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_horizontal_gaussian_blur_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::GaussianBlur(
                GaussianBlurShaderInput {
                    direction: GaussianBlurDirection::Horizontal,
                    sample_uniform_binding: 0,
                    input_texture_and_sampler_bindings: (0, 1),
                },
            )),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_vertical_gaussian_blur_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::GaussianBlur(
                GaussianBlurShaderInput {
                    direction: GaussianBlurDirection::Vertical,
                    sample_uniform_binding: 0,
                    input_texture_and_sampler_bindings: (0, 1),
                },
            )),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_no_tone_mapping_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::ToneMapping(ToneMappingShaderInput {
                mapping: ToneMapping::None,
                input_texture_and_sampler_bindings: (0, 1),
            })),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_aces_tone_mapping_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::ToneMapping(ToneMappingShaderInput {
                mapping: ToneMapping::ACES,
                input_texture_and_sampler_bindings: (0, 1),
            })),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
    fn building_khronos_pbr_neutral_tone_mapping_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[],
            Some(&MaterialShaderInput::ToneMapping(ToneMappingShaderInput {
                mapping: ToneMapping::KhronosPBRNeutral,
                input_texture_and_sampler_bindings: (0, 1),
            })),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            RenderAttachmentQuantitySet::empty(),
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
