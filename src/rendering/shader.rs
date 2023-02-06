//! Generation of graphics shaders.

mod blinn_phong;
mod fixed;
mod vertex_color;

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use fixed::{
    FixedColorShaderGenerator, FixedColorVertexOutputFieldIdx, FixedTextureShaderGenerator,
};
use naga::{
    AddressSpace, Arena, ArraySize, BinaryOperator, Binding, Block, BuiltIn, Bytes, Constant,
    ConstantInner, EntryPoint, Expression, Function, FunctionArgument, FunctionResult,
    GlobalVariable, Handle, ImageClass, ImageDimension, Interpolation, LocalVariable, Module,
    ResourceBinding, SampleLevel, Sampling, ScalarKind, ScalarValue, ShaderStage, Span, Statement,
    StructMember, SwizzleComponent, Type, TypeInner, UniqueArena, VectorSize,
};
use std::{borrow::Cow, hash::Hash, mem, vec};
use vertex_color::VertexColorShaderGenerator;

pub use blinn_phong::{BlinnPhongFeatureShaderInput, BlinnPhongTextureShaderInput};
pub use fixed::{FixedColorFeatureShaderInput, FixedTextureShaderInput};

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
    pub vertex: Cow<'static, str>,
    pub fragment: Cow<'static, str>,
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

/// Input description specifying the locations of the vertex
/// properties of the mesh to use in the shader. Only properties
/// required for the specific shader will actually be included.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshShaderInput {
    /// Vertex attribute location for vertex positions.
    pub position_location: u32,
    /// Vertex attribute location for vertex colors, or
    /// [`None`] if the mesh does not include colors.
    pub color_location: Option<u32>,
    /// Vertex attribute location for vertex normal vectors,
    /// or [`None`] if the mesh does not include normal
    /// vectors.
    pub normal_vector_location: Option<u32>,
    /// Vertex attribute location for vertex texture coordinates,
    /// or [`None`] if the mesh does not include texture
    /// coordinates.
    pub texture_coord_location: Option<u32>,
}

/// Input description for any kind of per-instance feature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InstanceFeatureShaderInput {
    ModelViewTransform(ModelViewTransformShaderInput),
    FixedColorMaterial(FixedColorFeatureShaderInput),
    BlinnPhongMaterial(BlinnPhongFeatureShaderInput),
    /// For convenice in unit tests.
    #[cfg(test)]
    None,
}

/// Input description specifying the vertex attribute
/// locations of the columns of the model view matrix to
/// use for transforming the mesh in the shader.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelViewTransformShaderInput {
    /// Vertex attribute locations for the four columns of
    /// the model view matrix.
    pub model_view_matrix_column_locations: (u32, u32, u32, u32),
    /// Vertex attribute locations for the four columns of
    /// the model view matrix for transforming normal vectors.
    pub normal_model_view_matrix_column_locations: (u32, u32, u32, u32),
}

/// Input description for any kind of material that may
/// require a texture.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaterialTextureShaderInput {
    FixedMaterial(FixedTextureShaderInput),
    BlinnPhongMaterial(BlinnPhongTextureShaderInput),
    None,
}

/// Input description specifying the bind group binding and the total size of
/// each light source uniform buffer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightShaderInput {
    /// Bind group binding of the uniform buffer for point lights.
    pub point_light_binding: u32,
    /// Maximum number of lights in the point light uniform buffer.
    pub max_point_light_count: u64,
}

bitflags! {
    /// Bitflag encoding a set of vertex properties.
    pub struct VertexPropertySet: u32 {
        const POSITION = 0b00000001;
        const COLOR = 0b00000010;
        const NORMAL_VECTOR = 0b00000100;
        const TEXTURE_COORDS = 0b00001000;
    }
}

/// Shader generator for any kind of material.
#[derive(Clone, Debug)]
pub enum MaterialShaderGenerator<'a> {
    /// Use vertex colors included in the mesh.
    VertexColor,
    FixedColor(FixedColorShaderGenerator<'a>),
    FixedTexture(FixedTextureShaderGenerator<'a>),
    BlinnPhong(BlinnPhongShaderGenerator<'a>),
}

/// Handles to expressions for accessing the model view matrix
/// variable, and optionally the model view matrix for normals
/// variable, in the main vertex shader function.
#[derive(Clone, Debug)]
pub struct ModelViewTransformExpressions {
    model_view_matrix: Handle<Expression>,
    normal_model_view_matrix: Option<Handle<Expression>>,
}

/// Handles to expressions for accessing the light uniform variables in the main
/// fragment shader function.
#[derive(Clone, Debug)]
pub struct LightExpressions {
    point_lights: Handle<Expression>,
}

/// Indices of the fields holding the various mesh vertex
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MeshVertexOutputFieldIndices {
    _clip_position: usize,
    position: Option<usize>,
    color: Option<usize>,
    normal_vector: Option<usize>,
    texture_coords: Option<usize>,
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
pub struct SampledTexture {
    texture_var_handle: Handle<GlobalVariable>,
    sampler_var_handle: Handle<GlobalVariable>,
}

/// Helper for generating code for a for-loop.
pub struct ForLoop {
    /// Expression for the index in the for-loop body.
    pub idx_expr_handle: Handle<Expression>,
    /// Set of statements making up the for-loop body.
    pub body: Block,
    continuing: Block,
    break_if: Option<Handle<Expression>>,
    n_iterations_expr_handle: Handle<Expression>,
    zero_constant_handle: Handle<Constant>,
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

    /// Returns the name of the fragment entry point function.
    pub fn fragment_entry_point_name(&self) -> &str {
        &self.entry_point_names.fragment
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
    /// Uses the given camera, mesh, model and material input
    /// descriptions to generate an appropriate shader [`Module`],
    /// containing both a vertex and fragment entry point.
    ///
    /// # Returns
    /// The generated shader [`Module`] and its [`ShaderEntryPointNames`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - There is no camera input (no shaders witout camera supported).
    /// - There is no mesh input (no shaders witout a mesh supported).
    /// - `instance_feature_shader_inputs` does not contain a
    ///   [`ModelInstanceTransformShaderInput`] (no shaders without a model
    ///   view transform supported).
    /// - `instance_feature_shader_inputs` and `material_texture_shader_input`
    ///   do not provide a consistent and supproted material description.
    /// - Not all vertex properties required by the material are available in
    ///   the input mesh.
    pub fn generate_shader_module(
        camera_shader_input: Option<&CameraShaderInput>,
        mesh_shader_input: Option<&MeshShaderInput>,
        light_shader_input: Option<&LightShaderInput>,
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&MaterialTextureShaderInput>,
    ) -> Result<(Module, EntryPointNames)> {
        let camera_shader_input = camera_shader_input
            .ok_or_else(|| anyhow!("Tried to build shader with no camera input"))?;

        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let (model_view_transform_shader_input, material_shader_builder) = Self::interpret_inputs(
            instance_feature_shader_inputs,
            material_texture_shader_input,
        )?;

        let vertex_property_requirements = material_shader_builder.vertex_property_requirements();
        let material_requires_lights = material_shader_builder.requires_lights();

        let mut module = Module::default();
        let mut vertex_function = Function::default();
        let mut fragment_function = Function::default();

        // Caution: The order in which the shader generators use and increment
        // the bind group index must match the order in which the bind groups
        // are set in `RenderPassRecorder::record_render_pass`, that is:
        // 1. Camera.
        // 2. Lights.
        // 3. Material textures.
        let mut bind_group_idx = 0;

        let projection_matrix_var_expr_handle = Self::generate_vertex_code_for_projection_matrix(
            camera_shader_input,
            &mut module.types,
            &mut module.global_variables,
            &mut vertex_function,
            &mut bind_group_idx,
        );

        let model_view_transform_expressions = Self::generate_vertex_code_for_model_view_transform(
            model_view_transform_shader_input,
            vertex_property_requirements,
            &mut module.types,
            &mut vertex_function,
        );

        let (mesh_vertex_output_field_indices, mut vertex_output_struct_builder) =
            Self::generate_vertex_code_for_vertex_properties(
                mesh_shader_input,
                vertex_property_requirements,
                &mut module.types,
                &mut module.constants,
                &mut vertex_function,
                &model_view_transform_expressions,
                projection_matrix_var_expr_handle,
            )?;

        let material_vertex_output_field_indices = material_shader_builder.generate_vertex_code(
            &mut module.types,
            &mut vertex_function,
            &mut vertex_output_struct_builder,
        );

        vertex_output_struct_builder
            .clone()
            .generate_output_code(&mut module.types, &mut vertex_function);

        let fragment_input_struct = vertex_output_struct_builder
            .generate_input_code(&mut module.types, &mut fragment_function);

        let light_expressions = if material_requires_lights {
            let light_shader_input =
                light_shader_input.ok_or_else(|| anyhow!("Missing lights for material"))?;

            Some(Self::generate_fragment_code_for_lights(
                light_shader_input,
                &mut module.types,
                &mut module.global_variables,
                &mut module.constants,
                &mut fragment_function,
                &mut bind_group_idx,
            ))
        } else {
            None
        };

        material_shader_builder.generate_fragment_code(
            &mut module.types,
            &mut module.constants,
            &mut module.functions,
            &mut module.global_variables,
            &mut fragment_function,
            &mut bind_group_idx,
            &fragment_input_struct,
            &mesh_vertex_output_field_indices,
            &material_vertex_output_field_indices,
            light_expressions.as_ref(),
        );

        let entry_point_names = EntryPointNames {
            vertex: Cow::Borrowed("mainVS"),
            fragment: Cow::Borrowed("mainFS"),
        };

        module.entry_points.push(EntryPoint {
            name: entry_point_names.vertex.to_string(),
            stage: ShaderStage::Vertex,
            early_depth_test: None,
            workgroup_size: [0, 0, 0],
            function: vertex_function,
        });

        module.entry_points.push(EntryPoint {
            name: entry_point_names.fragment.to_string(),
            stage: ShaderStage::Fragment,
            early_depth_test: None,
            workgroup_size: [0, 0, 0],
            function: fragment_function,
        });

        Ok((module, entry_point_names))
    }

    /// Interprets the set of instance feature and texture inputs
    /// to gather them groups of inputs that belong together, most
    /// notably gathering the inputs representing the material into
    /// a [`MaterialShaderGenerator`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - `instance_feature_shader_inputs` does not contain a
    ///   [`ModelInstanceTransformShaderInput`].
    /// - `instance_feature_shader_inputs` and `material_texture_shader_input`
    ///   do not provide a consistent and supproted material description.
    ///
    /// # Panics
    /// If `instance_feature_shader_inputs` contain multiple
    /// inputs of the same type.
    fn interpret_inputs<'a>(
        instance_feature_shader_inputs: &'a [&'a InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&'a MaterialTextureShaderInput>,
    ) -> Result<(
        &'a ModelViewTransformShaderInput,
        MaterialShaderGenerator<'a>,
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
            material_texture_shader_input,
        ) {
            (Some(feature_input), None, Some(MaterialTextureShaderInput::None)) => {
                MaterialShaderGenerator::FixedColor(FixedColorShaderGenerator::new(feature_input))
            }
            (None, None, Some(MaterialTextureShaderInput::FixedMaterial(texture_input))) => {
                MaterialShaderGenerator::FixedTexture(FixedTextureShaderGenerator::new(
                    texture_input,
                ))
            }
            (None, Some(feature_input), Some(texture_input)) => {
                #[allow(clippy::match_wildcard_for_single_variants)]
                let texture_input = match texture_input {
                    MaterialTextureShaderInput::None => None,
                    MaterialTextureShaderInput::BlinnPhongMaterial(texture_input) => {
                        Some(texture_input)
                    }
                    _ => {
                        return Err(anyhow!(
                            "Tried to use Blinn-Phong material with texture from another material"
                        ));
                    }
                };
                MaterialShaderGenerator::BlinnPhong(BlinnPhongShaderGenerator::new(
                    feature_input,
                    texture_input,
                ))
            }
            (None, None, Some(MaterialTextureShaderInput::None)) => {
                MaterialShaderGenerator::VertexColor
            }
            _ => {
                return Err(anyhow!("Tried to build shader with invalid material"));
            }
        };

        Ok((model_view_transform_shader_input, material_shader_builder))
    }

    /// Generates the declaration of the model view transform type,
    /// adds it as an argument to the main vertex shader function and
    /// generates the code for constructing the matrix from its columns
    /// in the body of the function. If the material requires normal
    /// vectors, corresponding code for the model view transform for
    /// normals will also be generated.
    ///
    /// # Returns
    /// A [`ModelViewTransformExpressions`] with handles to expressions
    /// for the generated matrix variables.
    fn generate_vertex_code_for_model_view_transform(
        model_view_transform_shader_input: &ModelViewTransformShaderInput,
        vertex_property_requirements: VertexPropertySet,
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
    ) -> ModelViewTransformExpressions {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);
        let mat4x4_type_handle = insert_in_arena(types, MATRIX_4X4_TYPE);

        let new_struct_field = |name: &'static str, location: u32, offset: u32| StructMember {
            name: new_name(name),
            ty: vec4_type_handle,
            binding: Some(Binding::Location {
                location,
                interpolation: None,
                sampling: None,
            }),
            offset,
        };

        let (loc_0, loc_1, loc_2, loc_3) =
            model_view_transform_shader_input.model_view_matrix_column_locations;

        let column_fields =
            if vertex_property_requirements.contains(VertexPropertySet::NORMAL_VECTOR) {
                let (loc_4, loc_5, loc_6, loc_7) =
                    model_view_transform_shader_input.normal_model_view_matrix_column_locations;

                vec![
                    new_struct_field("col0", loc_0, 0),
                    new_struct_field("col1", loc_1, VECTOR_4_SIZE),
                    new_struct_field("col2", loc_2, 2 * VECTOR_4_SIZE),
                    new_struct_field("col3", loc_3, 3 * VECTOR_4_SIZE),
                    new_struct_field("col4", loc_4, 4 * VECTOR_4_SIZE),
                    new_struct_field("col5", loc_5, 5 * VECTOR_4_SIZE),
                    new_struct_field("col6", loc_6, 6 * VECTOR_4_SIZE),
                    new_struct_field("col7", loc_7, 7 * VECTOR_4_SIZE),
                ]
            } else {
                vec![
                    new_struct_field("col0", loc_0, 0),
                    new_struct_field("col1", loc_1, VECTOR_4_SIZE),
                    new_struct_field("col2", loc_2, 2 * VECTOR_4_SIZE),
                    new_struct_field("col3", loc_3, 3 * VECTOR_4_SIZE),
                ]
            };

        let struct_size = VECTOR_4_SIZE * column_fields.len() as u32;

        let model_view_transform_type = Type {
            name: new_name("ModelViewTransform"),
            inner: TypeInner::Struct {
                members: column_fields,
                span: struct_size,
            },
        };

        let model_view_transform_type_handle = insert_in_arena(types, model_view_transform_type);

        let model_view_transform_arg_ptr_expr_handle = generate_input_argument(
            vertex_function,
            new_name("modelViewTransform"),
            model_view_transform_type_handle,
            None,
        );

        let mut define_matrix = |name: &str, start_field_idx: u32| {
            // Create expression constructing a 4x4 matrix from the columns
            // (each a field in the input struct)
            let matrix_expr_handle = emit(
                &mut vertex_function.body,
                &mut vertex_function.expressions,
                |expressions| {
                    let compose_expr = Expression::Compose {
                        ty: mat4x4_type_handle,
                        components: (start_field_idx..(start_field_idx + 4))
                            .into_iter()
                            .map(|index| {
                                append_to_arena(
                                    expressions,
                                    Expression::AccessIndex {
                                        base: model_view_transform_arg_ptr_expr_handle,
                                        index,
                                    },
                                )
                            })
                            .collect(),
                    };
                    append_to_arena(expressions, compose_expr)
                },
            );

            let matrix_var_ptr_expr_handle = append_to_arena(
                &mut vertex_function.expressions,
                Expression::LocalVariable(append_to_arena(
                    &mut vertex_function.local_variables,
                    LocalVariable {
                        name: new_name(name),
                        ty: mat4x4_type_handle,
                        init: None,
                    },
                )),
            );

            push_to_block(
                &mut vertex_function.body,
                Statement::Store {
                    pointer: matrix_var_ptr_expr_handle,
                    value: matrix_expr_handle,
                },
            );

            let matrix_var_expr_handle = emit(
                &mut vertex_function.body,
                &mut vertex_function.expressions,
                |expressions| {
                    append_to_arena(
                        expressions,
                        Expression::Load {
                            pointer: matrix_var_ptr_expr_handle,
                        },
                    )
                },
            );

            #[allow(clippy::let_and_return)]
            matrix_var_expr_handle
        };

        let model_view_matrix_var_expr_handle = define_matrix("modelViewMatrix", 0);

        let normal_model_view_matrix_var_expr_handle =
            if vertex_property_requirements.contains(VertexPropertySet::NORMAL_VECTOR) {
                Some(define_matrix("normalModelViewMatrix", 4))
            } else {
                None
            };

        ModelViewTransformExpressions {
            model_view_matrix: model_view_matrix_var_expr_handle,
            normal_model_view_matrix: normal_model_view_matrix_var_expr_handle,
        }
    }

    /// Generates the declaration of the global uniform variable for the
    /// camera projection matrix.
    ///
    /// # Returns
    /// Handle to the expression for the the projection matrix in the main
    /// vertex shader function.
    fn generate_vertex_code_for_projection_matrix(
        camera_shader_input: &CameraShaderInput,
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        vertex_function: &mut Function,
        bind_group_idx: &mut u32,
    ) -> Handle<Expression> {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let mat4x4_type_handle = insert_in_arena(types, MATRIX_4X4_TYPE);

        let projection_matrix_var_handle = append_to_arena(
            global_variables,
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

        let projection_matrix_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::GlobalVariable(projection_matrix_var_handle),
        );

        let projection_matrix_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: projection_matrix_ptr_expr_handle,
                    },
                )
            },
        );

        #[allow(clippy::let_and_return)]
        projection_matrix_expr_handle
    }

    /// Generates the declaration of the struct of mesh vertex properties,
    /// adds it as an argument to the main vertex shader function and
    /// begins generating the struct of output to pass from the vertex
    /// entry point to the fragment entry point.
    ///
    /// Only vertex properties required by the material are included in
    /// the input struct.
    ///
    /// The output struct always includes the clip space position, and
    /// the expression computing this by transforming the vertex position
    /// with the model view matrix and projection matrix is generated
    /// here. Other vertex properties are included in the output struct
    /// as required by the material. If the vertex position or normal
    /// vector is required, this is transformed to camera space before
    /// assigned to the output struct.
    ///
    /// # Returns
    /// Because the output struct may have to include additional material
    /// properties, its code can not be fully generated at this point.
    /// Instead, the [`OutputStructBuilder`] is returned so that the
    /// material shader genrator can complete it. The indices of the
    /// included vertex property fields are also returned for access in
    /// the fragment shader.
    ///
    /// # Errors
    /// Returns an error if not all vertex properties required by the material
    /// are available in the input mesh.
    fn generate_vertex_code_for_vertex_properties(
        mesh_shader_input: &MeshShaderInput,
        requirements: VertexPropertySet,
        types: &mut UniqueArena<Type>,
        constants: &mut Arena<Constant>,
        vertex_function: &mut Function,
        model_view_transform_expressions: &ModelViewTransformExpressions,
        projection_matrix_var_expr_handle: Handle<Expression>,
    ) -> Result<(MeshVertexOutputFieldIndices, OutputStructBuilder)> {
        let vec2_type_handle = insert_in_arena(types, VECTOR_2_TYPE);
        let vec3_type_handle = insert_in_arena(types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let input_model_position_expr_handle = generate_location_bound_input_argument(
            vertex_function,
            new_name("modelSpacePosition"),
            vec3_type_handle,
            mesh_shader_input.position_location,
        );

        let input_color_expr_handle = if requirements.contains(VertexPropertySet::COLOR) {
            if let Some(location) = mesh_shader_input.color_location {
                Some(generate_location_bound_input_argument(
                    vertex_function,
                    new_name("color"),
                    vec4_type_handle,
                    location,
                ))
            } else {
                return Err(anyhow!("Missing required vertex property `color`"));
            }
        } else {
            None
        };

        let input_model_normal_vector_expr_handle =
            if requirements.contains(VertexPropertySet::NORMAL_VECTOR) {
                if let Some(location) = mesh_shader_input.normal_vector_location {
                    Some(generate_location_bound_input_argument(
                        vertex_function,
                        new_name("modelSpaceNormalVector"),
                        vec3_type_handle,
                        location,
                    ))
                } else {
                    return Err(anyhow!("Missing required vertex property `normal_vector`"));
                }
            } else {
                None
            };

        let input_texture_coord_expr_handle =
            if requirements.contains(VertexPropertySet::TEXTURE_COORDS) {
                if let Some(location) = mesh_shader_input.texture_coord_location {
                    Some(generate_location_bound_input_argument(
                        vertex_function,
                        new_name("textureCoords"),
                        vec2_type_handle,
                        location,
                    ))
                } else {
                    return Err(anyhow!("Missing required vertex property `texture_coords`"));
                }
            } else {
                None
            };

        let unity_constant_expr = append_to_arena(
            &mut vertex_function.expressions,
            Expression::Constant(append_to_arena(constants, float32_constant(1.0))),
        );

        // Create expression converting the xyz vertex position to an
        // xyzw homogeneous coordinate (with w = 1.0) and transforming
        // it to camera space with the model view matrix
        let position_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                let compose_expr = Expression::Compose {
                    ty: vec4_type_handle,
                    components: vec![input_model_position_expr_handle, unity_constant_expr],
                };
                let homogeneous_position_expr_handle = append_to_arena(expressions, compose_expr);

                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: model_view_transform_expressions.model_view_matrix,
                        right: homogeneous_position_expr_handle,
                    },
                )
            },
        );

        let position_var_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut vertex_function.local_variables,
                LocalVariable {
                    name: new_name("cameraSpacePosition"),
                    ty: vec4_type_handle,
                    init: None,
                },
            )),
        );

        push_to_block(
            &mut vertex_function.body,
            Statement::Store {
                pointer: position_var_ptr_expr_handle,
                value: position_expr_handle,
            },
        );

        let position_var_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: position_var_ptr_expr_handle,
                    },
                )
            },
        );

        let mut output_struct_builder = OutputStructBuilder::new("VertexOutput");

        // Create expression multiplying the camera space homogeneous
        // vertex position with the projection matrix, yielding the
        // clip space position
        let clip_position_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: projection_matrix_var_expr_handle,
                        right: position_var_expr_handle,
                    },
                )
            },
        );
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

        if requirements.contains(VertexPropertySet::POSITION) {
            let output_position_expr_handle = emit(
                &mut vertex_function.body,
                &mut vertex_function.expressions,
                |expressions| {
                    append_to_arena(expressions, swizzle_xyz_expr(position_var_expr_handle))
                },
            );
            output_field_indices.position = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "position",
                    vec3_type_handle,
                    VECTOR_3_SIZE,
                    output_position_expr_handle,
                ),
            );
        }

        if let Some(input_color_expr_handle) = input_color_expr_handle {
            output_field_indices.color = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "color",
                    vec4_type_handle,
                    VECTOR_4_SIZE,
                    input_color_expr_handle,
                ),
            );
        }

        if let Some(input_model_normal_vector_expr_handle) = input_model_normal_vector_expr_handle {
            let zero_constant_expr = append_to_arena(
                &mut vertex_function.expressions,
                Expression::Constant(append_to_arena(constants, float32_constant(0.0))),
            );

            // Create expression converting the xyz normal vector to an xyzw
            // homogeneous vector (with w = 0.0) and transforming it to camera
            // space with the inverse transpose of the model view matrix
            emit(
                &mut vertex_function.body,
                &mut vertex_function.expressions,
                |expressions| {
                    let compose_expr = Expression::Compose {
                        ty: vec4_type_handle,
                        components: vec![input_model_normal_vector_expr_handle, zero_constant_expr],
                    };
                    let homogeneous_model_space_normal_vector_expr_handle =
                        append_to_arena(expressions, compose_expr);

                    let homogeneous_normal_vector_expr_handle = append_to_arena(
                        expressions,
                        Expression::Binary {
                            op: BinaryOperator::Multiply,
                            left: model_view_transform_expressions
                                .normal_model_view_matrix
                                .expect("Missing normal model view transform"),
                            right: homogeneous_model_space_normal_vector_expr_handle,
                        },
                    );

                    let normal_vector_expr_handle = append_to_arena(
                        expressions,
                        swizzle_xyz_expr(homogeneous_normal_vector_expr_handle),
                    );

                    output_field_indices.normal_vector = Some(
                        output_struct_builder.add_field_with_perspective_interpolation(
                            "normalVector",
                            vec3_type_handle,
                            VECTOR_3_SIZE,
                            normal_vector_expr_handle,
                        ),
                    );
                },
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

    /// Generates declarations for the light source uniform types, the types the
    /// light uniform buffers will be mapped to and the global variables these
    /// are bound to.
    ///
    /// # Returns
    /// Handle to the expression for accessing the point light uniform variable
    /// in the main fragment function.
    fn generate_fragment_code_for_lights(
        light_shader_input: &LightShaderInput,
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        constants: &mut Arena<Constant>,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
    ) -> LightExpressions {
        let u32_type_handle = insert_in_arena(types, U32_TYPE);
        let vec3_type_handle = insert_in_arena(types, VECTOR_3_TYPE);

        // The struct is padded to 16 byte alignment as required for uniforms
        let point_light_struct_size = 2 * (VECTOR_3_SIZE + F32_WIDTH);

        // The count at the beginning of the uniform buffer is padded to 16 bytes
        let light_count_size = 16;

        let point_light_struct_type_handle = insert_in_arena(
            types,
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
                    span: point_light_struct_size,
                },
            },
        );

        let max_point_light_count_constant_handle = append_to_arena(
            constants,
            u32_constant(light_shader_input.max_point_light_count),
        );

        let point_lights_array_type_handle = insert_in_arena(
            types,
            Type {
                name: None,
                inner: TypeInner::Array {
                    base: point_light_struct_type_handle,
                    size: ArraySize::Constant(max_point_light_count_constant_handle),
                    stride: point_light_struct_size,
                },
            },
        );

        let point_lights_struct_type_handle = insert_in_arena(
            types,
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
                            ty: point_lights_array_type_handle,
                            binding: None,
                            offset: light_count_size,
                        },
                    ],
                    span: point_light_struct_size
                        .checked_mul(
                            u32::try_from(light_shader_input.max_point_light_count).unwrap(),
                        )
                        .unwrap()
                        .checked_add(light_count_size)
                        .unwrap(),
                },
            },
        );

        let point_lights_var_handle = append_to_arena(
            global_variables,
            GlobalVariable {
                name: new_name("pointLights"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: *bind_group_idx,
                    binding: light_shader_input.point_light_binding,
                }),
                ty: point_lights_struct_type_handle,
                init: None,
            },
        );
        *bind_group_idx += 1;

        let point_lights_ptr_expr_handle = append_to_arena(
            &mut fragment_function.expressions,
            Expression::GlobalVariable(point_lights_var_handle),
        );

        LightExpressions {
            point_lights: point_lights_ptr_expr_handle,
        }
    }
}

impl LightExpressions {
    /// Generates the expression for the number of active point lights.
    pub fn generate_point_light_count_expr(&self, function: &mut Function) -> Handle<Expression> {
        Self::generate_light_count_expr(function, self.point_lights)
    }

    /// Takes an index expression and generates expressions for the position and
    /// radiance, respectively, of the point light at that index.
    pub fn generate_point_light_field_expressions(
        &self,
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        light_idx_expr_handle: Handle<Expression>,
    ) -> (Handle<Expression>, Handle<Expression>) {
        let point_light_ptr_expr = Self::generate_light_ptr_expr(
            block,
            expressions,
            self.point_lights,
            light_idx_expr_handle,
        );
        let position_expr_handle =
            Self::generate_field_access_expr(block, expressions, point_light_ptr_expr, 0);
        let radiance_expr_handle =
            Self::generate_field_access_expr(block, expressions, point_light_ptr_expr, 1);
        (position_expr_handle, radiance_expr_handle)
    }

    fn generate_light_count_expr(
        function: &mut Function,
        struct_ptr_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        Self::generate_field_access_expr(
            &mut function.body,
            &mut function.expressions,
            struct_ptr_expr_handle,
            0,
        )
    }

    fn generate_light_ptr_expr(
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        struct_ptr_expr_handle: Handle<Expression>,
        light_idx_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let lights_field_ptr_handle =
            Self::generate_field_access_ptr_expr(block, expressions, struct_ptr_expr_handle, 1);

        emit(block, expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Access {
                    base: lights_field_ptr_handle,
                    index: light_idx_expr_handle,
                },
            )
        })
    }

    fn generate_field_access_expr(
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        struct_ptr_expr_handle: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        let field_ptr_handle = Self::generate_field_access_ptr_expr(
            block,
            expressions,
            struct_ptr_expr_handle,
            field_idx,
        );
        emit(block, expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Load {
                    pointer: field_ptr_handle,
                },
            )
        })
    }

    fn generate_field_access_ptr_expr(
        block: &mut Block,
        expressions: &mut Arena<Expression>,
        struct_ptr_expr_handle: Handle<Expression>,
        field_idx: u32,
    ) -> Handle<Expression> {
        emit(block, expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::AccessIndex {
                    base: struct_ptr_expr_handle,
                    index: field_idx,
                },
            )
        })
    }
}

impl<'a> MaterialShaderGenerator<'a> {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    fn vertex_property_requirements(&self) -> VertexPropertySet {
        match self {
            Self::VertexColor => VertexColorShaderGenerator::vertex_property_requirements(),
            Self::FixedColor(_) => FixedColorShaderGenerator::vertex_property_requirements(),
            Self::FixedTexture(_) => FixedTextureShaderGenerator::vertex_property_requirements(),
            Self::BlinnPhong(builder) => builder.vertex_property_requirements(),
        }
    }

    /// Whether the material requires light sources.
    fn requires_lights(&self) -> bool {
        match self {
            Self::VertexColor => VertexColorShaderGenerator::requires_lights(),
            Self::FixedColor(_) => FixedColorShaderGenerator::requires_lights(),
            Self::FixedTexture(_) => FixedTextureShaderGenerator::requires_lights(),
            Self::BlinnPhong(_) => BlinnPhongShaderGenerator::requires_lights(),
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
    fn generate_vertex_code(
        &self,
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> MaterialVertexOutputFieldIndices {
        match self {
            Self::FixedColor(builder) => MaterialVertexOutputFieldIndices::FixedColor(
                builder.generate_vertex_code(types, vertex_function, vertex_output_struct_builder),
            ),
            Self::BlinnPhong(builder) => MaterialVertexOutputFieldIndices::BlinnPhong(
                builder.generate_vertex_code(types, vertex_function, vertex_output_struct_builder),
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
    fn generate_fragment_code(
        &self,
        types: &mut UniqueArena<Type>,
        constants: &mut Arena<Constant>,
        functions: &mut Arena<Function>,
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &MaterialVertexOutputFieldIndices,
        light_expressions: Option<&LightExpressions>,
    ) {
        match (self, material_input_field_indices) {
            (Self::VertexColor, MaterialVertexOutputFieldIndices::None) => {
                VertexColorShaderGenerator::generate_fragment_code(
                    types,
                    fragment_function,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (
                Self::FixedColor(_),
                MaterialVertexOutputFieldIndices::FixedColor(color_input_field_idx),
            ) => FixedColorShaderGenerator::generate_fragment_code(
                types,
                fragment_function,
                fragment_input_struct,
                color_input_field_idx,
            ),
            (Self::FixedTexture(builder), MaterialVertexOutputFieldIndices::None) => builder
                .generate_fragment_code(
                    types,
                    global_variables,
                    fragment_function,
                    bind_group_idx,
                    fragment_input_struct,
                    mesh_input_field_indices,
                ),
            (
                Self::BlinnPhong(builder),
                MaterialVertexOutputFieldIndices::BlinnPhong(material_input_field_indices),
            ) => builder.generate_fragment_code(
                types,
                constants,
                functions,
                global_variables,
                fragment_function,
                bind_group_idx,
                fragment_input_struct,
                mesh_input_field_indices,
                material_input_field_indices,
                light_expressions,
            ),
            _ => panic!("Mismatched material shader builder and output field indices type"),
        }
    }
}

impl InputStruct {
    /// Returns the handle to the expression for the struct
    /// field with the given index.
    ///
    /// # Panics
    /// If the index is out of bounds.
    fn get_field_expr_handle(&self, idx: usize) -> Handle<Expression> {
        self.input_field_expr_handles[idx]
    }
}

impl InputStructBuilder {
    /// Creates a builder for an input struct with the given
    /// type name and name to use when including the struct
    /// as an input argument.
    fn new<S: ToString, T: ToString>(type_name: S, input_arg_name: T) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_arg_name: input_arg_name.to_string(),
        }
    }

    fn n_fields(&self) -> usize {
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
    fn add_field<S: ToString>(
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
    fn generate_input_code(
        self,
        types: &mut UniqueArena<Type>,
        function: &mut Function,
    ) -> InputStruct {
        let n_fields = self.n_fields();

        let input_type_handle = insert_in_arena(types, self.builder.into_type());

        let input_arg_ptr_expr_handle =
            generate_input_argument(function, Some(self.input_arg_name), input_type_handle, None);

        let input_field_expr_handles = emit(
            &mut function.body,
            &mut function.expressions,
            |expressions| {
                (0..n_fields)
                    .into_iter()
                    .map(|idx| {
                        append_to_arena(
                            expressions,
                            Expression::AccessIndex {
                                base: input_arg_ptr_expr_handle,
                                index: idx as u32,
                            },
                        )
                    })
                    .collect()
            },
        );

        InputStruct {
            input_field_expr_handles,
        }
    }
}

impl OutputStructBuilder {
    /// Creates a builder for an output struct with the given
    /// type name.
    fn new<S: ToString>(type_name: S) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_expr_handles: Vec::new(),
            location: 0,
        }
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
    fn add_field<S: ToString>(
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
    fn add_field_with_perspective_interpolation<S: ToString>(
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
    fn add_builtin_position_field<S: ToString>(
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
    fn generate_output_code(self, types: &mut UniqueArena<Type>, function: &mut Function) {
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
            let output_struct_field_ptr_handle = emit(
                &mut function.body,
                &mut function.expressions,
                |expressions| {
                    append_to_arena(
                        expressions,
                        Expression::AccessIndex {
                            base: output_ptr_expr_handle,
                            index: idx as u32,
                        },
                    )
                },
            );
            push_to_block(
                &mut function.body,
                Statement::Store {
                    pointer: output_struct_field_ptr_handle,
                    value: input_expr_handle,
                },
            );
        }

        let output_expr_handle = emit(
            &mut function.body,
            &mut function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: output_ptr_expr_handle,
                    },
                )
            },
        );

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
    fn generate_input_code(
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
    fn new<S: ToString>(type_name: S) -> Self {
        Self {
            type_name: type_name.to_string(),
            fields: Vec::new(),
            offset: 0,
        }
    }

    fn n_fields(&self) -> usize {
        self.fields.len()
    }

    /// Adds a new struct field.
    ///
    /// # Returns
    /// The index of the added field.
    fn add_field<S: ToString>(
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
    fn into_type(self) -> Type {
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
    fn declare(
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        name: &'static str,
        group: u32,
        texture_binding: u32,
        sampler_binding: u32,
    ) -> Self {
        let texture_type_handle = insert_in_arena(types, IMAGE_TEXTURE_TYPE);
        let sampler_type_handle = insert_in_arena(types, IMAGE_TEXTURE_SAMPLER_TYPE);

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

    /// Generates and returns an expression sampling the texture at
    /// the texture coordinates specified by the given expression.
    fn generate_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let texture_var_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::GlobalVariable(self.texture_var_handle),
        );

        let sampler_var_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::GlobalVariable(self.sampler_var_handle),
        );

        let image_sampling_expr_handle = emit(
            &mut function.body,
            &mut function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::ImageSample {
                        image: texture_var_expr_handle,
                        sampler: sampler_var_expr_handle,
                        gather: None,
                        coordinate: texture_coord_expr_handle,
                        array_index: None,
                        offset: None,
                        level: SampleLevel::Auto,
                        depth_ref: None,
                    },
                )
            },
        );

        #[allow(clippy::let_and_return)]
        image_sampling_expr_handle
    }

    /// Generates and returns an expression sampling the texture at
    /// the texture coordinates specified by the given expression,
    /// and extracting the RGB values of the sampled RGBA color.
    fn generate_rgb_sampling_expr(
        &self,
        function: &mut Function,
        texture_coord_expr_handle: Handle<Expression>,
    ) -> Handle<Expression> {
        let sampling_expr_handle = self.generate_sampling_expr(function, texture_coord_expr_handle);

        emit(
            &mut function.body,
            &mut function.expressions,
            |expressions| append_to_arena(expressions, swizzle_xyz_expr(sampling_expr_handle)),
        )
    }
}

impl ForLoop {
    /// Generates code for a new for-loop with the number of iterations given by
    /// `n_iterations_expr_handle` and returns a new [`ForLoop`]. The loop index
    /// starts at zero, and is available as the `idx_expr_handle` field of the
    /// returned `ForLoop` struct. The main body of the loop is empty, and
    /// statements can be added to it by pushing to the `body` field of the
    /// returned `ForLoop`.
    pub fn new(
        types: &mut UniqueArena<Type>,
        constants: &mut Arena<Constant>,
        function: &mut Function,
        name: &str,
        n_iterations_expr_handle: Handle<Expression>,
    ) -> Self {
        let u32_type_handle = insert_in_arena(types, U32_TYPE);

        let zero_constant_handle = append_to_arena(constants, u32_constant(0));

        let idx_ptr_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut function.local_variables,
                LocalVariable {
                    name: Some(format!("{}_idx", name)),
                    ty: u32_type_handle,
                    init: Some(zero_constant_handle),
                },
            )),
        );

        let mut body_block = Block::new();

        let idx_expr_handle = emit(&mut body_block, &mut function.expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Load {
                    pointer: idx_ptr_expr_handle,
                },
            )
        });

        let mut continuing_block = Block::new();

        let unity_constant_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::Constant(append_to_arena(constants, u32_constant(1))),
        );

        let incremented_idx_expr = emit(
            &mut continuing_block,
            &mut function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Add,
                        left: idx_expr_handle,
                        right: unity_constant_expr_handle,
                    },
                )
            },
        );

        push_to_block(
            &mut continuing_block,
            Statement::Store {
                pointer: idx_ptr_expr_handle,
                value: incremented_idx_expr,
            },
        );

        let break_if_expr_handle = emit(
            &mut continuing_block,
            &mut function.expressions,
            |expressions| {
                let idx_expr_handle = append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: idx_ptr_expr_handle,
                    },
                );
                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::GreaterEqual,
                        left: idx_expr_handle,
                        right: n_iterations_expr_handle,
                    },
                )
            },
        );

        Self {
            body: body_block,
            continuing: continuing_block,
            break_if: Some(break_if_expr_handle),
            idx_expr_handle,
            n_iterations_expr_handle,
            zero_constant_handle,
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

        let zero_constant_expr_handle =
            append_to_arena(expressions, Expression::Constant(self.zero_constant_handle));

        let n_iter_above_zero_expr_handle = emit(block, expressions, |expressions| {
            append_to_arena(
                expressions,
                Expression::Binary {
                    op: BinaryOperator::Greater,
                    left: self.n_iterations_expr_handle,
                    right: zero_constant_expr_handle,
                },
            )
        });

        push_to_block(
            block,
            Statement::If {
                condition: n_iter_above_zero_expr_handle,
                accept: loop_block,
                reject: Block::new(),
            },
        );
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

    append_to_arena(
        &mut function.expressions,
        Expression::FunctionArgument(input_arg_idx),
    )
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

/// Executes the given closure that adds [`Expression`]s to
/// the given [`Arena`] before pushing to the given [`Block`]
/// a [`Statement::Emit`] emitting the range of added expressions.
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

#[cfg(test)]
mod test {
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
            model_view_matrix_column_locations: (
                INSTANCE_VERTEX_BINDING_START,
                INSTANCE_VERTEX_BINDING_START + 1,
                INSTANCE_VERTEX_BINDING_START + 2,
                INSTANCE_VERTEX_BINDING_START + 3,
            ),
            normal_model_view_matrix_column_locations: (
                INSTANCE_VERTEX_BINDING_START + 4,
                INSTANCE_VERTEX_BINDING_START + 5,
                INSTANCE_VERTEX_BINDING_START + 6,
                INSTANCE_VERTEX_BINDING_START + 7,
            ),
        });

    const MINIMAL_MESH_INPUT: MeshShaderInput = MeshShaderInput {
        position_location: MESH_VERTEX_BINDING_START,
        color_location: None,
        normal_vector_location: None,
        texture_coord_location: None,
    };

    const FIXED_COLOR_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
            color_location: MATERIAL_VERTEX_BINDING_START,
        });

    const FIXED_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::FixedMaterial(FixedTextureShaderInput {
            color_texture_and_sampler_bindings: (0, 1),
        });

    const BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: MATERIAL_VERTEX_BINDING_START,
            diffuse_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
            specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 2),
            shininess_location: MATERIAL_VERTEX_BINDING_START + 3,
            alpha_location: MATERIAL_VERTEX_BINDING_START + 4,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: MATERIAL_VERTEX_BINDING_START,
            diffuse_color_location: None,
            specular_color_location: Some(MATERIAL_VERTEX_BINDING_START + 1),
            shininess_location: MATERIAL_VERTEX_BINDING_START + 2,
            alpha_location: MATERIAL_VERTEX_BINDING_START + 3,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::BlinnPhongMaterial(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: None,
        });

    const TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: MATERIAL_VERTEX_BINDING_START,
            diffuse_color_location: None,
            specular_color_location: None,
            shininess_location: MATERIAL_VERTEX_BINDING_START + 1,
            alpha_location: MATERIAL_VERTEX_BINDING_START + 2,
        });

    const TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::BlinnPhongMaterial(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: Some((2, 3)),
        });

    const LIGHT_INPUT: LightShaderInput = LightShaderInput {
        point_light_binding: 0,
        max_point_light_count: 20,
    };

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
            struct VertexProperties {
                @location(0) position:       vec3<f32>,
                @location(1) texture_coords: vec2<f32>,
            }
            
            struct VertexOutput {
                @builtin(position) clip_position:  vec4<f32>,
                @location(0)       position:       vec3<f32>,
                @location(1)       texture_coords: vec2<f32>,
            }

            struct CameraUniform {
                view_proj: mat4x4<f32>,
            }

            struct PointLight {
                position: vec3<f32>,
                radiance: vec3<f32>,
            }

            struct PointLights {
                numLights: u32,
                lights: array<PointLight, 10>
            }

            @group(2) @binding(0)
            var<uniform> pointLights: PointLights;

            @group(0) @binding(0)
            var<uniform> camera: CameraUniform;
            
            @vertex
            fn main(vertex: VertexProperties) -> VertexOutput {
                var color: vec3<f32>;
                var out: VertexOutput;

                color = vertex.position.xyz;
                color += vertex.position.xyz;

                out.clip_position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
                out.position = vertex.position.xyz;
                out.texture_coords = vertex.texture_coords;
                return out;
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
        ShaderGenerator::generate_shader_module(None, None, None, &[], None).unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_input_fails() {
        ShaderGenerator::generate_shader_module(Some(&CAMERA_INPUT), None, None, &[], None)
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
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_without_material_and_no_color_in_mesh_fails() {
        ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
        )
        .unwrap();
    }

    #[test]
    fn building_vertex_color_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                position_location: MESH_VERTEX_BINDING_START,
                color_location: Some(MESH_VERTEX_BINDING_START + 1),
                normal_vector_location: None,
                texture_coord_location: None,
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&MaterialTextureShaderInput::None),
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
            Some(&MaterialTextureShaderInput::None),
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
                position_location: MESH_VERTEX_BINDING_START,
                color_location: None,
                normal_vector_location: None,
                texture_coord_location: Some(MESH_VERTEX_BINDING_START + 1),
            }),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            Some(&FIXED_TEXTURE_INPUT),
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
    fn building_blinn_phong_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                position_location: MESH_VERTEX_BINDING_START,
                color_location: None,
                normal_vector_location: Some(MESH_VERTEX_BINDING_START + 1),
                texture_coord_location: None,
            }),
            Some(&LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT, &BLINN_PHONG_FEATURE_INPUT],
            Some(&MaterialTextureShaderInput::None),
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
    fn building_diffuse_textured_blinn_phong_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                position_location: MESH_VERTEX_BINDING_START,
                color_location: None,
                normal_vector_location: Some(MESH_VERTEX_BINDING_START + 1),
                texture_coord_location: Some(MESH_VERTEX_BINDING_START + 2),
            }),
            Some(&LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
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
    fn building_textured_blinn_phong_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                position_location: MESH_VERTEX_BINDING_START,
                color_location: None,
                normal_vector_location: Some(MESH_VERTEX_BINDING_START + 1),
                texture_coord_location: Some(MESH_VERTEX_BINDING_START + 2),
            }),
            Some(&LIGHT_INPUT),
            &[
                &MODEL_VIEW_TRANSFORM_INPUT,
                &TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
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
