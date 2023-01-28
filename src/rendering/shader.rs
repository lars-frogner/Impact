//! Generation of graphics shaders.

mod blinn_phong;
mod fixed;

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use fixed::{
    FixedColorShaderGenerator, FixedColorVertexOutputFieldIdx, FixedTextureShaderGenerator,
};
use naga::{
    AddressSpace, Arena, BinaryOperator, Binding, Block, BuiltIn, Bytes, Constant, ConstantInner,
    EntryPoint, Expression, Function, FunctionArgument, FunctionResult, GlobalVariable, Handle,
    ImageClass, ImageDimension, Interpolation, LocalVariable, MathFunction, Module,
    ResourceBinding, SampleLevel, Sampling, ScalarKind, ScalarValue, ShaderStage, Span, Statement,
    StructMember, SwizzleComponent, Type, TypeInner, UniqueArena, VectorSize,
};
use std::{borrow::Cow, hash::Hash, mem, vec};

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
}

#[derive(Clone, Debug)]
pub struct ShaderGenerator;

/// Input description specifying the uniform binding of the
/// projection matrix of the camera to use in the shader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraShaderInput {
    /// Bind group binding of the uniform buffer holding the
    /// camera projection matrix.
    pub projection_matrix_binding: u32,
}

/// Input description specifying the locations of the vertex
/// properties of the mesh to use in the shader. Only properties
/// required for the specific shader will actually be included.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceFeatureShaderInput {
    ModelInstanceTransform(ModelInstanceTransformShaderInput),
    FixedColorMaterial(FixedColorFeatureShaderInput),
    BlinnPhongMaterial(BlinnPhongFeatureShaderInput),
    /// For convenice in unit tests.
    #[cfg(test)]
    None,
}

/// Input description specifying the vertex attribute
/// locations of the columns of the model view matrix to
/// use for transforming the mesh in the shader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInstanceTransformShaderInput {
    /// Vertex attribute locations for the four columns of
    /// the model view matrix.
    pub model_matrix_locations: (u32, u32, u32, u32),
}

/// Input description for any kind of material that may
/// require a texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialTextureShaderInput {
    FixedMaterial(FixedTextureShaderInput),
    BlinnPhongMaterial(BlinnPhongTextureShaderInput),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformShaderInput {}

bitflags! {
    /// Bitflag encoding a set of vertex properties required
    /// by a material.
    pub struct VertexPropertyRequirements: u32 {
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

/// Shader generator for the case when vertex colors
/// included in the mesh are used to obtain the fragment
/// color.
#[derive(Copy, Clone, Debug)]
pub struct VertexColorShaderGenerator;

/// Indices of the fields holding the various mesh vertex
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MeshVertexOutputFieldIndices {
    clip_position: usize,
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

const FLOAT32_WIDTH: u32 = mem::size_of::<f32>() as u32;

const FLOAT_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Scalar {
        kind: ScalarKind::Float,
        width: FLOAT32_WIDTH as Bytes,
    },
};

const VECTOR_2_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Bi,
        kind: ScalarKind::Float,
        width: FLOAT32_WIDTH as Bytes,
    },
};
const VECTOR_2_SIZE: u32 = 2 * FLOAT32_WIDTH;

const VECTOR_3_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Tri,
        kind: ScalarKind::Float,
        width: FLOAT32_WIDTH as Bytes,
    },
};
const VECTOR_3_SIZE: u32 = 3 * FLOAT32_WIDTH;

const VECTOR_4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Vector {
        size: VectorSize::Quad,
        kind: ScalarKind::Float,
        width: FLOAT32_WIDTH as Bytes,
    },
};
const VECTOR_4_SIZE: u32 = 4 * FLOAT32_WIDTH;

const MATRIX_4X4_TYPE: Type = Type {
    name: None,
    inner: TypeInner::Matrix {
        columns: VectorSize::Quad,
        rows: VectorSize::Quad,
        width: FLOAT32_WIDTH as Bytes,
    },
};
const MATRIX_4X4_SIZE: u32 = 4 * VECTOR_4_SIZE;

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

impl ShaderGenerator {
    /// Uses the given camera, mesh, model and material input
    /// descriptions to generate an appropriate shader [`Module`],
    /// containing both a vertex and fragment entry point.
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
        instance_feature_shader_inputs: &[&InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&MaterialTextureShaderInput>,
    ) -> Result<Module> {
        let camera_shader_input = camera_shader_input
            .ok_or_else(|| anyhow!("Tried to build shader with no camera input"))?;

        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let (model_instance_transform_shader_input, material_shader_builder) =
            Self::interpret_inputs(
                instance_feature_shader_inputs,
                material_texture_shader_input,
            )?;

        let vertex_property_requirements = material_shader_builder.vertex_property_requirements();

        let mut module = Module::default();
        let mut vertex_function = Function::default();
        let mut fragment_function = Function::default();

        let projection_matrix_var_expr_handle = Self::generate_vertex_code_for_projection_matrix(
            camera_shader_input,
            &mut module.types,
            &mut module.global_variables,
            &mut vertex_function,
        );

        let model_matrix_var_expr_handle = Self::generate_vertex_code_for_model_matrix(
            model_instance_transform_shader_input,
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
                model_matrix_var_expr_handle,
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

        material_shader_builder.generate_fragment_code(
            &mut module.types,
            &mut module.global_variables,
            &mut fragment_function,
            &fragment_input_struct,
            &mesh_vertex_output_field_indices,
            &material_vertex_output_field_indices,
        );

        module.entry_points.push(EntryPoint {
            name: "mainVS".to_string(),
            stage: ShaderStage::Vertex,
            early_depth_test: None,
            workgroup_size: [0, 0, 0],
            function: vertex_function,
        });

        module.entry_points.push(EntryPoint {
            name: "mainFS".to_string(),
            stage: ShaderStage::Fragment,
            early_depth_test: None,
            workgroup_size: [0, 0, 0],
            function: fragment_function,
        });

        Ok(module)
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
        &'a ModelInstanceTransformShaderInput,
        MaterialShaderGenerator<'a>,
    )> {
        let mut model_instance_transform_shader_input = None;
        let mut fixed_color_feature_shader_input = None;
        let mut blinn_phong_feature_shader_input = None;

        for &instance_feature_shader_input in instance_feature_shader_inputs {
            match instance_feature_shader_input {
                InstanceFeatureShaderInput::ModelInstanceTransform(shader_input) => {
                    let old = model_instance_transform_shader_input.replace(shader_input);
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

        let model_instance_transform_shader_input = model_instance_transform_shader_input
            .ok_or_else(|| {
                anyhow!("Tried to build shader with no model instance transform input")
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
            (None, None, None) => MaterialShaderGenerator::VertexColor,
            _ => {
                return Err(anyhow!("Tried to build shader with invalid material"));
            }
        };

        Ok((
            model_instance_transform_shader_input,
            material_shader_builder,
        ))
    }

    /// Generates the declaration of the model view transform type,
    /// adds it as an argument to the main vertex shader function and
    /// generates the code for constructing the matrix from its columns
    /// in the body of the function.
    ///
    /// # Returns
    /// Handle to the expression for the model view matrix in the main
    /// vertex shader function.
    fn generate_vertex_code_for_model_matrix(
        model_instance_transform_shader_input: &ModelInstanceTransformShaderInput,
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
    ) -> Handle<Expression> {
        let (loc_0, loc_1, loc_2, loc_3) =
            model_instance_transform_shader_input.model_matrix_locations;

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

        let model_transform_type = Type {
            name: new_name("ModelTransform"),
            inner: TypeInner::Struct {
                members: vec![
                    new_struct_field("col0", loc_0, 0),
                    new_struct_field("col1", loc_1, VECTOR_4_SIZE),
                    new_struct_field("col2", loc_2, 2 * VECTOR_4_SIZE),
                    new_struct_field("col3", loc_3, 3 * VECTOR_4_SIZE),
                ],
                span: MATRIX_4X4_SIZE,
            },
        };

        let model_transform_type_handle = insert_in_arena(types, model_transform_type);

        let model_transform_arg_idx = u32::try_from(vertex_function.arguments.len()).unwrap();

        vertex_function.arguments.push(FunctionArgument {
            name: new_name("modelTransform"),
            ty: model_transform_type_handle,
            binding: None,
        });

        let model_transform_arg_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::FunctionArgument(model_transform_arg_idx),
        );

        // Create expression constructing a 4x4 matrix from the columns
        // (each a field in the input struct)
        let model_matrix_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                let compose_expr = Expression::Compose {
                    ty: mat4x4_type_handle,
                    components: (0..4_u32)
                        .into_iter()
                        .map(|index| {
                            append_to_arena(
                                expressions,
                                Expression::AccessIndex {
                                    base: model_transform_arg_ptr_expr_handle,
                                    index,
                                },
                            )
                        })
                        .collect(),
                };
                append_to_arena(expressions, compose_expr)
            },
        );

        let model_matrix_var_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut vertex_function.local_variables,
                LocalVariable {
                    name: new_name("modelMatrix"),
                    ty: mat4x4_type_handle,
                    init: None,
                },
            )),
        );

        push_to_block(
            &mut vertex_function.body,
            Statement::Store {
                pointer: model_matrix_var_ptr_expr_handle,
                value: model_matrix_expr_handle,
            },
        );

        let model_matrix_var_expr_handle = emit(
            &mut vertex_function.body,
            &mut vertex_function.expressions,
            |expressions| {
                append_to_arena(
                    expressions,
                    Expression::Load {
                        pointer: model_matrix_var_ptr_expr_handle,
                    },
                )
            },
        );

        #[allow(clippy::let_and_return)]
        model_matrix_var_expr_handle
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
    ) -> Handle<Expression> {
        let mat4x4_type_handle = insert_in_arena(types, MATRIX_4X4_TYPE);

        let projection_matrix_var_handle = append_to_arena(
            global_variables,
            GlobalVariable {
                name: new_name("projectionMatrix"),
                space: AddressSpace::Uniform,
                binding: Some(ResourceBinding {
                    group: 0,
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
        requirements: VertexPropertyRequirements,
        types: &mut UniqueArena<Type>,
        constants: &mut Arena<Constant>,
        vertex_function: &mut Function,
        model_matrix_var_expr_handle: Handle<Expression>,
        projection_matrix_var_expr_handle: Handle<Expression>,
    ) -> Result<(MeshVertexOutputFieldIndices, OutputStructBuilder)> {
        let vec2_type_handle = insert_in_arena(types, VECTOR_2_TYPE);
        let vec3_type_handle = insert_in_arena(types, VECTOR_3_TYPE);
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("VertexAttributes", "vertex");

        let input_model_position_field_idx = input_struct_builder.add_field(
            "modelSpacePosition",
            vec3_type_handle,
            mesh_shader_input.position_location,
            VECTOR_3_SIZE,
        );

        let input_color_field_idx = if requirements.contains(VertexPropertyRequirements::COLOR) {
            if let Some(location) = mesh_shader_input.color_location {
                Some(input_struct_builder.add_field(
                    "color",
                    vec4_type_handle,
                    location,
                    VECTOR_4_SIZE,
                ))
            } else {
                return Err(anyhow!("Missing required vertex property `color`"));
            }
        } else {
            None
        };

        let input_model_normal_vector_field_idx =
            if requirements.contains(VertexPropertyRequirements::NORMAL_VECTOR) {
                if let Some(location) = mesh_shader_input.normal_vector_location {
                    Some(input_struct_builder.add_field(
                        "modelSpaceNormalVector",
                        vec3_type_handle,
                        location,
                        VECTOR_3_SIZE,
                    ))
                } else {
                    return Err(anyhow!("Missing required vertex property `normal_vector`"));
                }
            } else {
                None
            };

        let input_texture_coord_field_idx =
            if requirements.contains(VertexPropertyRequirements::TEXTURE_COORDS) {
                if let Some(location) = mesh_shader_input.texture_coord_location {
                    Some(input_struct_builder.add_field(
                        "textureCoords",
                        vec2_type_handle,
                        location,
                        VECTOR_2_SIZE,
                    ))
                } else {
                    return Err(anyhow!("Missing required vertex property `texture_coords`"));
                }
            } else {
                None
            };

        let input_struct = input_struct_builder.generate_input_code(types, vertex_function);

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
                    components: vec![
                        input_struct.get_field_expr_handle(input_model_position_field_idx),
                        unity_constant_expr,
                    ],
                };
                let homogeneous_position_expr_handle = append_to_arena(expressions, compose_expr);

                append_to_arena(
                    expressions,
                    Expression::Binary {
                        op: BinaryOperator::Multiply,
                        left: model_matrix_var_expr_handle,
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
                    name: new_name("position"),
                    ty: vec4_type_handle,
                    init: None,
                },
            )),
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

        push_to_block(
            &mut vertex_function.body,
            Statement::Store {
                pointer: position_var_ptr_expr_handle,
                value: position_expr_handle,
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
            clip_position: output_clip_position_field_idx,
            position: None,
            color: None,
            normal_vector: None,
            texture_coords: None,
        };

        if requirements.contains(VertexPropertyRequirements::POSITION) {
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

        if let Some(idx) = input_color_field_idx {
            output_field_indices.color = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "color",
                    vec4_type_handle,
                    VECTOR_4_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        if let Some(idx) = input_model_normal_vector_field_idx {
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
                    // Inverse not supported by WGSL, should be precalculated
                    // and included in vertex buffer instead
                    // let inverse_model_matrix_expr_handle = append_to_arena(
                    //     expressions,
                    //     Expression::Math {
                    //         fun: MathFunction::Inverse,
                    //         arg: model_matrix_var_expr_handle,
                    //         arg1: None,
                    //         arg2: None,
                    //         arg3: None,
                    //     },
                    // );

                    let inverse_transpose_model_matrix_expr_handle = append_to_arena(
                        expressions,
                        Expression::Math {
                            fun: MathFunction::Transpose,
                            // arg: inverse_model_matrix_expr_handle,
                            arg: model_matrix_var_expr_handle,
                            arg1: None,
                            arg2: None,
                            arg3: None,
                        },
                    );

                    let compose_expr = Expression::Compose {
                        ty: vec4_type_handle,
                        components: vec![
                            input_struct.get_field_expr_handle(idx),
                            zero_constant_expr,
                        ],
                    };
                    let homogeneous_model_space_normal_vector_expr_handle =
                        append_to_arena(expressions, compose_expr);

                    let homogeneous_normal_vector_expr_handle = append_to_arena(
                        expressions,
                        Expression::Binary {
                            op: BinaryOperator::Multiply,
                            left: inverse_transpose_model_matrix_expr_handle,
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

        if let Some(idx) = input_texture_coord_field_idx {
            output_field_indices.texture_coords = Some(
                output_struct_builder.add_field_with_perspective_interpolation(
                    "textureCoords",
                    vec2_type_handle,
                    VECTOR_2_SIZE,
                    input_struct.get_field_expr_handle(idx),
                ),
            );
        }

        Ok((output_field_indices, output_struct_builder))
    }
}

impl<'a> MaterialShaderGenerator<'a> {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    fn vertex_property_requirements(&self) -> VertexPropertyRequirements {
        match self {
            Self::VertexColor => VertexColorShaderGenerator::vertex_property_requirements(),
            Self::FixedColor(_) => FixedColorShaderGenerator::vertex_property_requirements(),
            Self::FixedTexture(_) => FixedTextureShaderGenerator::vertex_property_requirements(),
            Self::BlinnPhong(builder) => builder.vertex_property_requirements(),
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
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        material_input_field_indices: &MaterialVertexOutputFieldIndices,
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
                    fragment_input_struct,
                    mesh_input_field_indices,
                ),
            (
                Self::BlinnPhong(builder),
                MaterialVertexOutputFieldIndices::BlinnPhong(material_input_field_indices),
            ) => builder.generate_fragment_code(
                types,
                global_variables,
                fragment_function,
                fragment_input_struct,
                mesh_input_field_indices,
                material_input_field_indices,
            ),
            _ => panic!("Mismatched material shader builder and output field indices type"),
        }
    }
}

impl VertexColorShaderGenerator {
    /// Returns a bitflag encoding the vertex properties required
    /// by the material.
    const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::COLOR
    }

    /// Generates the fragment shader code specific to this material
    /// by adding code representation to the given [`naga`] objects.
    ///
    /// The interpolated vertex color passed from the main vertex shader
    /// function is simply returned from the main fragment shader function
    /// in an output struct.
    fn generate_fragment_code(
        types: &mut UniqueArena<Type>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            fragment_input_struct.get_field_expr_handle(
                mesh_input_field_indices
                    .color
                    .expect("No `color` passed to vertex color fragment shader"),
            ),
        );

        output_struct_builder.generate_output_code(types, fragment_function);
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

        let input_arg_idx = u32::try_from(function.arguments.len()).unwrap();

        function.arguments.push(FunctionArgument {
            name: Some(self.input_arg_name),
            ty: input_type_handle,
            binding: None,
        });

        let input_arg_ptr_expr_handle = append_to_arena(
            &mut function.expressions,
            Expression::FunctionArgument(input_arg_idx),
        );

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
}

fn new_name<S: ToString>(name_str: S) -> Option<String> {
    Some(name_str.to_string())
}

fn float32_constant(value: f64) -> Constant {
    Constant {
        name: None,
        specialization: None,
        inner: ConstantInner::Scalar {
            width: FLOAT32_WIDTH as Bytes,
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

    const VERTEX_POSITION_BINDING: u32 = 4;

    const CAMERA_INPUT: CameraShaderInput = CameraShaderInput {
        projection_matrix_binding: 0,
    };

    const MODEL_TRANSFORM_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::ModelInstanceTransform(ModelInstanceTransformShaderInput {
            model_matrix_locations: (0, 1, 2, 3),
        });

    const MINIMAL_MESH_INPUT: MeshShaderInput = MeshShaderInput {
        position_location: VERTEX_POSITION_BINDING,
        color_location: None,
        normal_vector_location: None,
        texture_coord_location: None,
    };

    const FIXED_COLOR_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
            color_location: 8,
        });

    const FIXED_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::FixedMaterial(FixedTextureShaderInput {
            color_texture_and_sampler_bindings: (0, 1),
        });

    const BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: 8,
            diffuse_color_location: Some(9),
            specular_color_location: Some(10),
            shininess_location: 11,
            alpha_location: 12,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: 8,
            diffuse_color_location: None,
            specular_color_location: Some(9),
            shininess_location: 10,
            alpha_location: 11,
        });

    const DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::BlinnPhongMaterial(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: None,
        });

    const TEXTURED_BLINN_PHONG_FEATURE_INPUT: InstanceFeatureShaderInput =
        InstanceFeatureShaderInput::BlinnPhongMaterial(BlinnPhongFeatureShaderInput {
            ambient_color_location: 8,
            diffuse_color_location: None,
            specular_color_location: None,
            shininess_location: 9,
            alpha_location: 10,
        });

    const TEXTURED_BLINN_PHONG_TEXTURE_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::BlinnPhongMaterial(BlinnPhongTextureShaderInput {
            diffuse_texture_and_sampler_bindings: (0, 1),
            specular_texture_and_sampler_bindings: Some((2, 3)),
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

            @group(0) @binding(0)
            var<uniform> camera: CameraUniform;
            
            @vertex
            fn main(vertex: VertexProperties) -> VertexOutput {
                var out: VertexOutput;
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
            }
        }
    }

    #[test]
    #[should_panic]
    fn building_shader_with_no_inputs_fails() {
        ShaderGenerator::generate_shader_module(None, None, &[], None).unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_input_fails() {
        ShaderGenerator::generate_shader_module(Some(&CAMERA_INPUT), None, &[], None).unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_and_mesh_input_fails() {
        ShaderGenerator::generate_shader_module(
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
        ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            &[&MODEL_TRANSFORM_INPUT],
            None,
        )
        .unwrap();
    }

    #[test]
    fn building_vertex_color_shader_works() {
        let module = ShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MeshShaderInput {
                position_location: VERTEX_POSITION_BINDING,
                color_location: Some(VERTEX_POSITION_BINDING + 1),
                normal_vector_location: None,
                texture_coord_location: None,
            }),
            &[&MODEL_TRANSFORM_INPUT],
            None,
        )
        .unwrap();

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
            &[&MODEL_TRANSFORM_INPUT, &FIXED_COLOR_FEATURE_INPUT],
            Some(&MaterialTextureShaderInput::None),
        )
        .unwrap();

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
                position_location: VERTEX_POSITION_BINDING,
                color_location: None,
                normal_vector_location: None,
                texture_coord_location: Some(VERTEX_POSITION_BINDING + 1),
            }),
            &[&MODEL_TRANSFORM_INPUT],
            Some(&FIXED_TEXTURE_INPUT),
        )
        .unwrap();

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
                position_location: VERTEX_POSITION_BINDING,
                color_location: None,
                normal_vector_location: Some(VERTEX_POSITION_BINDING + 1),
                texture_coord_location: None,
            }),
            &[&MODEL_TRANSFORM_INPUT, &BLINN_PHONG_FEATURE_INPUT],
            Some(&MaterialTextureShaderInput::None),
        )
        .unwrap();

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
                position_location: VERTEX_POSITION_BINDING,
                color_location: None,
                normal_vector_location: Some(VERTEX_POSITION_BINDING + 1),
                texture_coord_location: Some(VERTEX_POSITION_BINDING + 2),
            }),
            &[
                &MODEL_TRANSFORM_INPUT,
                &DIFFUSE_TEXTURED_BLINN_PHONG_FEATURE_INPUT,
            ],
            Some(&DIFFUSE_TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
        )
        .unwrap();

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
                position_location: VERTEX_POSITION_BINDING,
                color_location: None,
                normal_vector_location: Some(VERTEX_POSITION_BINDING + 1),
                texture_coord_location: Some(VERTEX_POSITION_BINDING + 2),
            }),
            &[&MODEL_TRANSFORM_INPUT, &TEXTURED_BLINN_PHONG_FEATURE_INPUT],
            Some(&TEXTURED_BLINN_PHONG_TEXTURE_INPUT),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }
}
