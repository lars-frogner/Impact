//! Graphics shaders.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use naga::{
    AddressSpace, Arena, BinaryOperator, Binding, Block, BuiltIn, Bytes, Constant, ConstantInner,
    EntryPoint, Expression, Function, FunctionArgument, FunctionResult, GlobalVariable, Handle,
    ImageClass, ImageDimension, Interpolation, LocalVariable, MathFunction, Module,
    ResourceBinding, SampleLevel, Sampling, ScalarKind, ScalarValue, ShaderStage, Span, Statement,
    StructMember, SwizzleComponent, Type, TypeInner, UniqueArena, VectorSize,
};
use std::{borrow::Cow, hash::Hash, mem, vec};

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
    pub projection_matrix_binding: u32,
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

bitflags! {
    struct VertexPropertyRequirements: u32 {
        const POSITION = 0b00000001;
        const COLOR = 0b00000010;
        const NORMAL_VECTOR = 0b00000100;
        const TEXTURE_COORDS = 0b00001000;
    }
}

#[derive(Clone, Debug)]
enum MaterialShaderBuilder<'a> {
    VertexColor,
    FixedColor(FixedColorShaderBuilder<'a>),
    FixedTexture(FixedTextureShaderBuilder<'a>),
    BlinnPhong(BlinnPhongShaderBuilder<'a>),
}

#[derive(Copy, Clone, Debug)]
struct VertexColorShaderBuilder;

#[derive(Clone, Debug)]
struct FixedColorShaderBuilder<'a> {
    feature_input: &'a FixedColorFeatureShaderInput,
}

#[derive(Clone, Debug)]
struct FixedTextureShaderBuilder<'a> {
    texture_input: &'a FixedTextureShaderInput,
}

#[derive(Clone, Debug)]
struct BlinnPhongShaderBuilder<'a> {
    feature_input: &'a BlinnPhongFeatureShaderInput,
    texture_input: Option<&'a BlinnPhongTextureShaderInput>,
}

#[derive(Clone, Debug)]
struct MeshVertexOutputFieldIndices {
    clip_position: usize,
    position: Option<usize>,
    color: Option<usize>,
    normal_vector: Option<usize>,
    texture_coords: Option<usize>,
}

#[derive(Clone, Debug)]
enum MaterialVertexOutputFieldIndices {
    FixedColor(FixedColorVertexOutputFieldIdx),
    BlinnPhong(BlinnPhongVertexOutputFieldIndices),
    None,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
struct FixedColorVertexOutputFieldIdx(usize);

#[derive(Clone, Debug)]
struct BlinnPhongVertexOutputFieldIndices {
    ambient_color: usize,
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    shininess: usize,
    alpha: usize,
}

#[derive(Clone, Debug)]
struct InputStruct {
    input_field_expr_handles: Vec<Handle<Expression>>,
}

#[derive(Clone, Debug)]
struct InputStructBuilder {
    builder: StructBuilder,
    input_arg_name: String,
}

#[derive(Clone, Debug)]
struct OutputStructBuilder {
    builder: StructBuilder,
    input_expr_handles: Vec<Handle<Expression>>,
    location: u32,
}

#[derive(Clone, Debug)]
struct StructBuilder {
    name: String,
    fields: Vec<StructMember>,
    offset: u32,
}

struct SampledTexture {
    texture_var_handle: Handle<GlobalVariable>,
    sampler_var_handle: Handle<GlobalVariable>,
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

    fn interpret_inputs<'a>(
        instance_feature_shader_inputs: &'a [&'a InstanceFeatureShaderInput],
        material_texture_shader_input: Option<&'a MaterialTextureShaderInput>,
    ) -> Result<(
        &'a ModelInstanceTransformShaderInput,
        MaterialShaderBuilder<'a>,
    )> {
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

        let material_shader_builder = match (
            fixed_color_feature_shader_input,
            blinn_phong_feature_shader_input,
            material_texture_shader_input,
        ) {
            (Some(feature_input), None, Some(MaterialTextureShaderInput::None)) => {
                MaterialShaderBuilder::FixedColor(FixedColorShaderBuilder { feature_input })
            }
            (None, None, Some(MaterialTextureShaderInput::FixedMaterial(texture_input))) => {
                MaterialShaderBuilder::FixedTexture(FixedTextureShaderBuilder { texture_input })
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
                MaterialShaderBuilder::BlinnPhong(BlinnPhongShaderBuilder {
                    feature_input,
                    texture_input,
                })
            }
            (None, None, None) => MaterialShaderBuilder::VertexColor,
            _ => {
                return Err(anyhow!("Tried to build shader with invalid material"));
            }
        };

        Ok((
            model_instance_transform_shader_input,
            material_shader_builder,
        ))
    }

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

impl<'a> MaterialShaderBuilder<'a> {
    fn vertex_property_requirements(&self) -> VertexPropertyRequirements {
        match self {
            Self::VertexColor => VertexColorShaderBuilder::vertex_property_requirements(),
            Self::FixedColor(_) => FixedColorShaderBuilder::vertex_property_requirements(),
            Self::FixedTexture(_) => FixedTextureShaderBuilder::vertex_property_requirements(),
            Self::BlinnPhong(builder) => builder.vertex_property_requirements(),
        }
    }

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
                VertexColorShaderBuilder::generate_fragment_code(
                    types,
                    fragment_function,
                    fragment_input_struct,
                    mesh_input_field_indices,
                );
            }
            (
                Self::FixedColor(_),
                MaterialVertexOutputFieldIndices::FixedColor(color_input_field_idx),
            ) => FixedColorShaderBuilder::generate_fragment_code(
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

impl VertexColorShaderBuilder {
    const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::COLOR
    }

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

impl<'a> FixedColorShaderBuilder<'a> {
    const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::empty()
    }

    fn generate_vertex_code(
        &self,
        types: &mut UniqueArena<Type>,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> FixedColorVertexOutputFieldIdx {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let color_arg_idx = u32::try_from(vertex_function.arguments.len()).unwrap();

        vertex_function.arguments.push(FunctionArgument {
            name: new_name("color"),
            ty: vec4_type_handle,
            binding: Some(Binding::Location {
                location: self.feature_input.color_location,
                interpolation: None,
                sampling: None,
            }),
        });

        let vertex_color_arg_ptr_expr_handle = append_to_arena(
            &mut vertex_function.expressions,
            Expression::FunctionArgument(color_arg_idx),
        );

        // let vertex_color_arg_expr_handle = emit(
        //     &mut vertex_function.body,
        //     &mut vertex_function.expressions,
        //     |expressions| {
        //         append_to_arena(
        //             expressions,
        //             Expression::Load {
        //                 pointer: vertex_color_arg_ptr_expr_handle,
        //             },
        //         )
        //     },
        // );

        let output_color_field_idx = vertex_output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            Some(Interpolation::Flat),
            Some(Sampling::Center),
            VECTOR_4_SIZE,
            vertex_color_arg_ptr_expr_handle,
        );

        FixedColorVertexOutputFieldIdx(output_color_field_idx)
    }

    fn generate_fragment_code(
        types: &mut UniqueArena<Type>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        color_input_field_idx: &FixedColorVertexOutputFieldIdx,
    ) {
        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            fragment_input_struct.get_field_expr_handle(color_input_field_idx.0),
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }
}

impl<'a> FixedTextureShaderBuilder<'a> {
    const fn vertex_property_requirements() -> VertexPropertyRequirements {
        VertexPropertyRequirements::TEXTURE_COORDS
    }

    fn generate_fragment_code(
        &self,
        types: &mut UniqueArena<Type>,
        global_variables: &mut Arena<GlobalVariable>,
        fragment_function: &mut Function,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) {
        let (color_texture_binding, color_sampler_binding) =
            self.texture_input.color_texture_and_sampler_bindings;

        let vec4_type_handle = insert_in_arena(types, VECTOR_4_TYPE);

        let color_texture = SampledTexture::declare(
            types,
            global_variables,
            "color",
            1,
            color_texture_binding,
            color_sampler_binding,
        );

        let color_sampling_expr_handle = color_texture.generate_sampling_expr(
            fragment_function,
            fragment_input_struct.get_field_expr_handle(
                mesh_input_field_indices
                    .texture_coords
                    .expect("No `texture_coords` passed to fixed texture fragment shader"),
            ),
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type_handle,
            None,
            None,
            VECTOR_4_SIZE,
            color_sampling_expr_handle,
        );

        output_struct_builder.generate_output_code(types, fragment_function);
    }
}

impl<'a> BlinnPhongShaderBuilder<'a> {
    fn vertex_property_requirements(&self) -> VertexPropertyRequirements {
        if self.texture_input.is_some() {
            VertexPropertyRequirements::POSITION
                | VertexPropertyRequirements::NORMAL_VECTOR
                | VertexPropertyRequirements::TEXTURE_COORDS
        } else {
            VertexPropertyRequirements::POSITION | VertexPropertyRequirements::NORMAL_VECTOR
        }
    }

    fn generate_vertex_code(
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

    fn generate_fragment_code(
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

impl InputStruct {
    fn get_field_expr_handle(&self, idx: usize) -> Handle<Expression> {
        self.input_field_expr_handles[idx]
    }
}

impl InputStructBuilder {
    fn new<S: ToString, T: ToString>(type_name: S, input_arg_name: T) -> Self {
        Self {
            builder: StructBuilder::new(type_name),
            input_arg_name: input_arg_name.to_string(),
        }
    }

    fn n_fields(&self) -> usize {
        self.builder.n_fields()
    }

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
    fn new<S: ToString>(name: S) -> Self {
        Self {
            builder: StructBuilder::new(name),
            input_expr_handles: Vec::new(),
            location: 0,
        }
    }

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
}

impl StructBuilder {
    fn new<S: ToString>(name: S) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            offset: 0,
        }
    }

    fn n_fields(&self) -> usize {
        self.fields.len()
    }

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

    fn into_type(self) -> Type {
        Type {
            name: Some(self.name),
            inner: TypeInner::Struct {
                members: self.fields,
                span: self.offset,
            },
        }
    }
}

impl SampledTexture {
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

fn insert_in_arena<T>(arena: &mut UniqueArena<T>, value: T) -> Handle<T>
where
    T: Eq + Hash,
{
    arena.insert(value, Span::UNDEFINED)
}

fn append_to_arena<T>(arena: &mut Arena<T>, value: T) -> Handle<T> {
    arena.append(value, Span::UNDEFINED)
}

fn push_to_block(block: &mut Block, statement: Statement) {
    block.push(statement, Span::UNDEFINED);
}

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
    fn building_vertex_color_shader_works() {
        let module = ShaderBuilder::build_shader_source(
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
        let module = ShaderBuilder::build_shader_source(
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
        let module = ShaderBuilder::build_shader_source(
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
        let module = ShaderBuilder::build_shader_source(
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
        let module = ShaderBuilder::build_shader_source(
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
        let module = ShaderBuilder::build_shader_source(
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
