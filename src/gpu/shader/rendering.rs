//! Generation of rendering shaders.

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
pub use gaussian_blur::GaussianBlurShaderInput;
pub use microfacet::{
    DiffuseMicrofacetShadingModel, MicrofacetShadingModel, MicrofacetTextureShaderInput,
    SpecularMicrofacetShadingModel,
};
pub use passthrough::PassthroughShaderInput;
pub use prepass::{
    BumpMappingTextureShaderInput, NormalMappingShaderInput, ParallaxMappingShaderInput,
    PrepassTextureShaderInput,
};
pub use skybox::SkyboxTextureShaderInput;
pub use tone_mapping::ToneMappingShaderInput;

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, generate_input_argument,
    generate_location_bound_input_argument, include_expr_in_func, include_named_expr_in_func,
    insert_in_arena, new_name, swizzle_xyz_expr, EntryPointNames, InputStruct, OutputStructBuilder,
    PushConstantExpressions, SampledTexture, SourceCode, TextureType, F32_TYPE, F32_WIDTH,
    MATRIX_4X4_TYPE, U32_TYPE, VECTOR_2_SIZE, VECTOR_2_TYPE, VECTOR_3_SIZE, VECTOR_3_TYPE,
    VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantGroupStage, PushConstantVariant},
        rendering::fre,
        texture::attachment::RenderAttachmentQuantitySet,
    },
    light::MAX_SHADOW_MAP_CASCADES,
    mesh::{
        VertexAttribute, VertexAttributeSet, VertexColor, VertexNormalVector, VertexPosition,
        VertexTangentSpaceQuaternion, VertexTextureCoords, N_VERTEX_ATTRIBUTES,
    },
};
use ambient_occlusion::AmbientOcclusionShaderGenerator;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use blinn_phong::{BlinnPhongShaderGenerator, BlinnPhongVertexOutputFieldIndices};
use fixed::{
    FixedColorShaderGenerator, FixedColorVertexOutputFieldIdx, FixedTextureShaderGenerator,
};
use gaussian_blur::GaussianBlurShaderGenerator;
use lazy_static::lazy_static;
use microfacet::{MicrofacetShaderGenerator, MicrofacetVertexOutputFieldIndices};
use naga::{
    AddressSpace, ArraySize, BinaryOperator, Binding, EntryPoint, Expression, Function,
    GlobalVariable, Handle, Literal, Module, ResourceBinding, ShaderStage, StructMember,
    SwizzleComponent, Type, TypeInner, UnaryOperator, VectorSize,
};
use passthrough::PassthroughShaderGenerator;
use prepass::{PrepassShaderGenerator, PrepassVertexOutputFieldIndices};
use skybox::{SkyboxShaderGenerator, SkyboxVertexOutputFieldIndices};
use std::{borrow::Cow, num::NonZeroU32};
use tone_mapping::ToneMappingShaderGenerator;
use vertex_color::VertexColorShaderGenerator;

/// Generator for shader programs.
#[derive(Clone, Debug)]
pub(super) struct RenderShaderGenerator;

/// Input description specifying the uniform binding of the projection matrix of
/// the camera to use in the shader.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraShaderInput {
    /// Bind group binding of the uniform buffer holding the camera projection
    /// matrix.
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
enum MaterialShaderGenerator<'a> {
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
    /// effects in rendering shaders.
    struct RenderShaderTricks: u8 {
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
struct ModelViewTransformExpressions {
    pub rotation_quaternion: Handle<Expression>,
    pub translation_vector: Handle<Expression>,
    pub scaling_factor: Handle<Expression>,
}

/// Handle to expressions for a projection.
#[derive(Clone, Debug)]
enum ProjectionExpressions {
    Camera(CameraProjectionVariable),
    OmnidirectionalLight(OmnidirectionalLightProjectionExpressions),
    UnidirectionalLight(UnidirectionalLightProjectionExpressions),
}

/// Handle to the global variable for the camera projection matrix.
#[derive(Clone, Debug)]
struct CameraProjectionVariable {
    projection_matrix_var: Handle<GlobalVariable>,
}

/// Marker type with method for projecting points onto a face of a shadow
/// cubemap.
#[derive(Clone, Debug)]
struct OmnidirectionalLightProjectionExpressions;

/// Handle to expressions for the orthographic transform components associated
/// with a unidirectional light.
#[derive(Clone, Debug)]
struct UnidirectionalLightProjectionExpressions {
    pub translation: Handle<Expression>,
    pub scaling: Handle<Expression>,
}

#[allow(clippy::enum_variant_names)]
/// Generator for shader code associated with a light source.
#[derive(Clone, Debug)]
enum LightShaderGenerator {
    AmbientLight(AmbientLightShaderGenerator),
    OmnidirectionalLight(OmnidirectionalLightShaderGenerator),
    UnidirectionalLight(UnidirectionalLightShaderGenerator),
}

/// Generator for shader code for shading a fragment with the light from an
/// ambient light.
#[derive(Clone, Debug)]
struct AmbientLightShaderGenerator {
    pub luminance: Handle<Expression>,
}

/// Generator for shader code associated with an omnidirectional light source.
#[derive(Clone, Debug)]
enum OmnidirectionalLightShaderGenerator {
    ForShadowMapUpdate(OmnidirectionalLightShadowMapUpdateShaderGenerator),
    ForShading(OmnidirectionalLightShadingShaderGenerator),
}

/// Generator for shader code associated with a unidirectional light source.
#[derive(Clone, Debug)]
enum UnidirectionalLightShaderGenerator {
    ForShadowMapUpdate(UnidirectionalLightShadowMapUpdateShaderGenerator),
    ForShading(UnidirectionalLightShadingShaderGenerator),
}

/// Generator for shader code for updating the shadow cubemap of an
/// omnidirectional light.
#[derive(Clone, Debug)]
struct OmnidirectionalLightShadowMapUpdateShaderGenerator {
    pub near_distance: Handle<Expression>,
    pub inverse_distance_span: Handle<Expression>,
}

/// Generator for shader code for shading a fragment with the light from an
/// omnidirectional light.
#[derive(Clone, Debug)]
struct OmnidirectionalLightShadingShaderGenerator {
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
struct UnidirectionalLightShadowMapUpdateShaderGenerator {
    pub orthographic_projection: UnidirectionalLightProjectionExpressions,
}

/// Generator for shading a fragment with the light from a unidirectional light
/// source.
#[derive(Clone, Debug)]
struct UnidirectionalLightShadingShaderGenerator {
    pub active_light_ptr_expr_in_vertex_function: Handle<Expression>,
    pub active_light_ptr_expr_in_fragment_function: Handle<Expression>,
    pub shadow_map: SampledTexture,
}

/// Expressions for vertex attributes passed as input to the vertex entry point
/// function.
struct MeshVertexInputExpressions {
    pub position: Handle<Expression>,
    pub color: Option<Handle<Expression>>,
    pub normal_vector: Option<Handle<Expression>>,
    pub texture_coords: Option<Handle<Expression>>,
    pub tangent_space_quaternion: Option<Handle<Expression>>,
}

/// Indices of the fields holding the various mesh vertex attributes and related
/// quantities in the vertex shader output struct.
#[derive(Clone, Debug)]
struct MeshVertexOutputFieldIndices {
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
enum LightVertexOutputFieldIndices {
    UnidirectionalLight(UnidirectionalLightVertexOutputFieldIndices),
}

/// Indices of the fields holding the various unidirectional light related
/// properties in the vertex shader output struct.
#[derive(Clone, Debug)]
struct UnidirectionalLightVertexOutputFieldIndices {
    pub light_space_position: usize,
    pub light_space_normal_vector: Option<usize>,
}

/// Indices of any fields holding the properties of a specific material in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
enum MaterialVertexOutputFieldIndices {
    FixedColor(FixedColorVertexOutputFieldIdx),
    BlinnPhong(BlinnPhongVertexOutputFieldIndices),
    Microfacet(MicrofacetVertexOutputFieldIndices),
    Prepass(PrepassVertexOutputFieldIndices),
    Skybox(SkyboxVertexOutputFieldIndices),
    None,
}

lazy_static! {
    static ref SHADER_SOURCE_LIB: SourceCode = SourceCode::from_wgsl_source(concat!(
        include_str!("../../../shader/rendering/util.wgsl"),
        include_str!("../../../shader/rendering/light.wgsl"),
        include_str!("../../../shader/rendering/normal_map.wgsl"),
        include_str!("../../../shader/rendering/blinn_phong.wgsl"),
        include_str!("../../../shader/rendering/microfacet.wgsl"),
        include_str!("../../../shader/rendering/ambient_occlusion.wgsl"),
        include_str!("../../../shader/rendering/gaussian_blur.wgsl"),
        include_str!("../../../shader/rendering/tone_mapping.wgsl")
    ))
    .unwrap_or_else(|err| panic!(
        "Error when including rendering shader source library: {}",
        err
    ));
}

impl RenderShaderGenerator {
    /// Uses the given camera, mesh, light, model and material input
    /// descriptions to generate an appropriate shader [`Module`], containing a
    /// vertex and (optionally) a fragment entry point.
    ///
    /// # Returns
    /// The generated shader [`Module`].
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
        push_constants: PushConstantGroup,
    ) -> Result<Module> {
        let mesh_shader_input =
            mesh_shader_input.ok_or_else(|| anyhow!("Tried to build shader with no mesh input"))?;

        let (model_view_transform_shader_input, material_shader_generator) =
            Self::interpret_rendering_inputs(
                instance_feature_shader_inputs,
                material_shader_input,
            )?;

        let mut module = Module::default();
        let mut vertex_function = Function::default();
        let mut fragment_function = Function::default();

        let mut source_code_lib = SHADER_SOURCE_LIB.clone();

        let push_constant_vertex_expressions = PushConstantExpressions::generate(
            &mut module,
            &mut vertex_function,
            push_constants.clone(),
            PushConstantGroupStage::Vertex,
        );

        let push_constant_fragment_expressions = PushConstantExpressions::generate(
            &mut module,
            &mut fragment_function,
            push_constants,
            PushConstantGroupStage::Fragment,
        );

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
            .map_or_else(RenderShaderTricks::empty, |generator| generator.tricks());

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
                vertex: Some(Cow::Borrowed("mainVS")),
                fragment: Some(Cow::Borrowed("mainFS")),
                compute: None,
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
                vertex: Some(Cow::Borrowed("mainVS")),
                fragment: fragment_entry_point_name,
                compute: None,
            }
        };

        module.entry_points.push(EntryPoint {
            name: entry_point_names.vertex.as_deref().unwrap().to_string(),
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

        Ok(module)
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
    fn interpret_rendering_inputs<'a>(
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
        tricks: RenderShaderTricks,
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
                if tricks.contains(RenderShaderTricks::FOLLOW_CAMERA) {
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
            Some(projection) if !tricks.contains(RenderShaderTricks::NO_VERTEX_PROJECTION) => {
                projection.generate_projected_position_expr(
                    module,
                    source_code_lib,
                    vertex_function,
                    tricks,
                    position_expr,
                )
            }
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

    /// Creates a generator of shader code for the light type in the given
    /// shader input.
    fn create_light_shader_generator(
        light_shader_input: &LightShaderInput,
        module: &mut Module,
        vertex_function: &mut Function,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        vertex_attribute_requirements: &mut VertexAttributeSet,
        push_constant_vertex_expressions: &PushConstantExpressions,
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_vertex_expressions: &PushConstantExpressions,
        push_constant_fragment_expressions: &PushConstantExpressions,
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
    pub fn tricks(&self) -> RenderShaderTricks {
        match self {
            Self::Skybox(_) => SkyboxShaderGenerator::TRICKS,
            Self::Passthrough(_) => PassthroughShaderGenerator::TRICKS,
            Self::AmbientOcclusion(_) => AmbientOcclusionShaderGenerator::TRICKS,
            Self::GaussianBlur(_) => GaussianBlurShaderGenerator::TRICKS,
            Self::ToneMapping(_) => ToneMappingShaderGenerator::TRICKS,
            _ => RenderShaderTricks::empty(),
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
                    push_constant_fragment_expressions,
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
        tricks: RenderShaderTricks,
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
        tricks: RenderShaderTricks,
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

            if tricks.contains(RenderShaderTricks::DRAW_AT_MAX_DEPTH) {
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_vertex_expressions: &PushConstantExpressions,
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
        push_constant_vertex_expressions: &PushConstantExpressions,
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_expressions: &PushConstantExpressions,
    ) -> Handle<Expression> {
        let lights_struct_ptr_expr =
            include_expr_in_func(function, Expression::GlobalVariable(lights_struct_var));

        Self::generate_single_light_ptr_expr(
            function,
            lights_struct_ptr_expr,
            push_constant_expressions
                .get(PushConstantVariant::LightIdx)
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
        framebuffer_position_expr: Handle<Expression>,
        position_expr: Handle<Expression>,
        normal_vector_expr: Handle<Expression>,
        view_dir_expr: Handle<Expression>,
        roughness_expr: Option<Handle<Expression>>,
        emulate_area_light_reflection: bool,
    ) -> (Handle<Expression>, Handle<Expression>) {
        source_code_lib.use_type(module, "OmniLightQuantities");

        let exposure = push_constant_fragment_expressions
            .get(PushConstantVariant::Exposure)
            .expect(
                "Missing exposure push constant for computing omnidirectional light quantities",
            );

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
                    exposure,
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
                    exposure,
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

            let pre_exposed_incident_luminance_expr = include_expr_in_func(
                function,
                Expression::AccessIndex {
                    base: light_quantities,
                    index: 0,
                },
            );

            let shadow_masked_pre_exposed_incident_luminance_expr = include_expr_in_func(
                function,
                Expression::Binary {
                    op: BinaryOperator::Multiply,
                    left: light_access_factor_expr,
                    right: pre_exposed_incident_luminance_expr,
                },
            );

            (
                reflection_dot_products_expr,
                shadow_masked_pre_exposed_incident_luminance_expr,
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
        push_constant_vertex_expressions: &PushConstantExpressions,
    ) -> Self {
        let lights_struct_ptr_expr = include_expr_in_func(
            vertex_function,
            Expression::GlobalVariable(lights_struct_var),
        );

        let active_light_ptr_expr = LightShaderGenerator::generate_single_light_ptr_expr(
            vertex_function,
            lights_struct_ptr_expr,
            push_constant_vertex_expressions
                .get(PushConstantVariant::LightIdx)
                .expect("Missing light index push constant"),
        );

        let orthographic_transform_ptr_expr =
            UnidirectionalLightShaderGenerator::generate_single_orthographic_transform_ptr_expr(
                vertex_function,
                active_light_ptr_expr,
                push_constant_vertex_expressions
                    .get(PushConstantVariant::CascadeIdx)
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
        push_constant_vertex_expressions: &PushConstantExpressions,
        push_constant_fragment_expressions: &PushConstantExpressions,
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
        push_constant_fragment_expressions: &PushConstantExpressions,
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

        let exposure = push_constant_fragment_expressions
            .get(PushConstantVariant::Exposure)
            .expect("Missing exposure push constant for computing unidirectional light quantities");

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
                    exposure,
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
                    exposure,
                ],
            )
        };

        let (pre_exposed_incident_luminance_expr, light_clip_position_expr) =
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

        let (reflection_dot_products_expr, shadow_masked_pre_exposed_incident_luminance_expr) =
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
                            right: pre_exposed_incident_luminance_expr,
                        },
                    ),
                )
            });

        (
            reflection_dot_products_expr,
            shadow_masked_pre_exposed_incident_luminance_expr,
        )
    }
}

impl SampledTexture {
    /// Generates and returns an expression for the fraction of light reaching
    /// the fragment based on sampling of the specified shadow map cascade
    /// around the texture coordinates converted from the x- and y-component of
    /// the given light clip space position, using the z-component as the
    /// reference depth.
    fn generate_light_access_factor_expr_for_shadow_cubemap(
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
    fn generate_light_access_factor_expr_for_cascaded_shadow_map(
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

// Ignore tests if running with Miri, since `naga::front::wgsl::parse_str`
// becomes extremely slow
#[cfg(test)]
// #[cfg(not(miri))]
#[allow(clippy::dbg_macro)]
mod test {
    use super::*;
    use crate::{
        gpu::push_constant::PushConstant,
        material::special::{gaussian_blur::GaussianBlurDirection, tone_mapping::ToneMapping},
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
        RenderShaderGenerator::generate_shader_module(
            None,
            None,
            None,
            &[],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn building_shader_with_only_camera_input_fails() {
        RenderShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            None,
            None,
            &[],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
        )
        .unwrap();
    }

    #[test]
    fn building_depth_prepass_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_omnidirectional_light_shadow_map_update_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&OMNIDIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT).into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_unidirectional_light_shadow_map_update_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
            None,
            Some(&MINIMAL_MESH_INPUT),
            Some(&UNIDIRECTIONAL_LIGHT_INPUT),
            &[&MODEL_VIEW_TRANSFORM_INPUT],
            None,
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::VERTEX),
                PushConstant::new(PushConstantVariant::CascadeIdx, wgpu::ShaderStages::VERTEX),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_vertex_color_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            VertexAttributeSet::COLOR,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
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
        let module = RenderShaderGenerator::generate_shader_module(
            Some(&CAMERA_INPUT),
            Some(&MINIMAL_MESH_INPUT),
            None,
            &[&MODEL_VIEW_TRANSFORM_INPUT, &FIXED_COLOR_FEATURE_INPUT],
            Some(&MaterialShaderInput::Fixed(None)),
            VertexAttributeSet::empty(),
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
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
        let module = RenderShaderGenerator::generate_shader_module(
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
            VertexAttributeSet::TEXTURE_COORDS,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::empty(),
            PushConstantGroup::new(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_diffuse_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_diffuse_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_diffuse_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_diffuse_blinn_phong_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_diffuse_uniform_specular_blinn_phong_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_diffuse_uniform_specular_blinn_phong_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_diffuse_specular_blinn_phong_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_diffuse_specular_blinn_phong_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_blinn_phong_shader_with_input_position_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::POSITION,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_blinn_phong_shader_with_input_normal_vector_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_blinn_phong_shader_with_input_position_and_normal_vector_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::POSITION | RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_lambertian_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_lambertian_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works()
    {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_diffuse_microfacet_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_diffuse_microfacet_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_ggx_diffuse_uniform_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_ggx_diffuse_uniform_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_ggx_diffuse_ggx_specular_microfacet_shader_with_omnidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_ggx_diffuse_ggx_specular_microfacet_shader_with_unidirectional_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(
                    PushConstantVariant::LightIdx,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                ),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_microfacet_shader_with_input_position_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::POSITION,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_microfacet_shader_with_input_normal_vector_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_microfacet_shader_with_input_position_and_normal_vector_attachment_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            RenderAttachmentQuantitySet::POSITION | RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::empty(),
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(
                    PushConstantVariant::InverseWindowDimensions,
                    wgpu::ShaderStages::FRAGMENT,
                ),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_minimal_prepass_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstantGroup::new(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_prepass_shader_with_emissive_luminance_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT).into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_prepass_shader_with_normal_mapping_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstantGroup::new(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_prepass_shader_with_parallax_mapping_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstantGroup::new(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_lambertian_diffuse_prepass_shader_with_ambient_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_microfacet_specular_prepass_shader_with_ambient_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_and_normal_mapping_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_textured_lambertian_diffuse_microfacet_specular_prepass_shader_with_ambient_light_and_parallax_mapping_works(
    ) {
        let module = RenderShaderGenerator::generate_shader_module(
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
            [
                PushConstant::new(PushConstantVariant::LightIdx, wgpu::ShaderStages::FRAGMENT),
                PushConstant::new(PushConstantVariant::Exposure, wgpu::ShaderStages::FRAGMENT),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_passthrough_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_ambient_occlusion_computation_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_ambient_occlusion_application_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_horizontal_gaussian_blur_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_vertical_gaussian_blur_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_no_tone_mapping_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_aces_tone_mapping_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }

    #[test]
    fn building_khronos_pbr_neutral_tone_mapping_shader_works() {
        let module = RenderShaderGenerator::generate_shader_module(
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
            PushConstant::new(
                PushConstantVariant::InverseWindowDimensions,
                wgpu::ShaderStages::FRAGMENT,
            )
            .into(),
        )
        .unwrap();

        let module_info = validate_module(&module);

        println!(
            "{}",
            wgsl_out::write_string(&module, &module_info, WriterFlags::all()).unwrap()
        );
    }
}
