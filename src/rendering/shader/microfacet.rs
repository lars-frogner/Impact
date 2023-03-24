//! Generation of shaders for microfacet materials.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
    insert_in_arena, new_name, push_to_block, InputStruct, InputStructBuilder,
    LightShaderGenerator, LightVertexOutputFieldIndices, MeshVertexOutputFieldIndices,
    OutputStructBuilder, PointLightShaderGenerator, SampledTexture, SourceCode, TextureType,
    UnidirectionalLightShaderGenerator, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE, VECTOR_3_TYPE,
    VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Expression, Function, Handle, LocalVariable, MathFunction, Module, Statement};

/// Describes the combination of models used for diffuse and specular reflection
/// as part of a microfacet based reflection model.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetShadingModel {
    pub diffuse: DiffuseMicrofacetShadingModel,
    pub specular: SpecularMicrofacetShadingModel,
}

/// Models describing diffuse reflection as part of a microfacet based
/// reflection model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiffuseMicrofacetShadingModel {
    /// No diffuse reflection.
    None,
    /// Diffuse reflection is modelled with a Lambertian BRDF.
    Lambertian,
    /// Diffuse reflection is modelled with a microfacet BRDF using the GGX
    /// distribution of normals.
    GGX,
}

/// Models describing specular reflection as part of a microfacet based
/// reflection model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpecularMicrofacetShadingModel {
    /// No specular reflection.
    None,
    /// Specular reflection is modelled with a microfacet BRDF using the GGX
    /// distribution of normals.
    GGX,
}

/// Input description specifying the vertex attribute locations of microfacet
/// material properties, reqired for generating a shader for a
/// [`MicrofacetMaterial`](crate::scene::MicrofacetMaterial), a
/// [`DiffuseTexturedMicrofacetMaterial`](crate::scene::DiffuseTexturedMicrofacetMaterial)
/// or a
/// [`TexturedMicrofacetMaterial`](crate::scene::TexturedMicrofacetMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetFeatureShaderInput {
    /// Vertex attribute location for the instance feature
    /// representing diffuse color. If [`None`], diffuse
    /// color is obtained from a texture instead.
    pub diffuse_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing specular color. If [`None`], specular
    /// color is obtained from a texture instead.
    pub specular_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature
    /// representing roughness.
    pub roughness_location: u32,
}

/// Input description specifying the bindings of textures
/// for microfacet properties, required for generating a
/// shader for a
/// [`DiffuseTexturedMicrofacetMaterial`](crate::scene::DiffuseTexturedMicrofacetMaterial)
/// or a [`TexturedMicrofacetMaterial`](crate::scene::TexturedMicrofacetMaterial).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and
    /// its sampler.
    pub diffuse_texture_and_sampler_bindings: (u32, u32),
    /// Bind group bindings of the specular color texture and
    /// its sampler. If [`None`], specular color is an instance
    /// feature instead.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a
/// [`MicrofacetMaterial`](crate::scene::MicrofacetMaterial), a
/// [`DiffuseTexturedMicrofacetMaterial`](crate::scene::DiffuseTexturedMicrofacetMaterial)
/// or a
/// [`TexturedMicrofacetMaterial`](crate::scene::TexturedMicrofacetMaterial).
#[derive(Clone, Debug)]
pub struct MicrofacetShaderGenerator<'a> {
    model: &'a MicrofacetShadingModel,
    feature_input: &'a MicrofacetFeatureShaderInput,
    texture_input: Option<&'a MicrofacetTextureShaderInput>,
}

/// Indices of the fields holding the various microfacet properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MicrofacetVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    roughness: usize,
}

impl MicrofacetShadingModel {
    pub const NO_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::None,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const LAMBERTIAN_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::Lambertian,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const GGX_DIFFUSE_GGX_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::GGX,
        specular: SpecularMicrofacetShadingModel::GGX,
    };

    pub const GGX_DIFFUSE_NO_SPECULAR: Self = Self {
        diffuse: DiffuseMicrofacetShadingModel::GGX,
        specular: SpecularMicrofacetShadingModel::None,
    };
}

impl<'a> MicrofacetShaderGenerator<'a> {
    /// Creates a new shader generator using the given input description.
    pub fn new(
        model: &'a MicrofacetShadingModel,
        feature_input: &'a MicrofacetFeatureShaderInput,
        texture_input: Option<&'a MicrofacetTextureShaderInput>,
    ) -> Self {
        Self {
            model,
            feature_input,
            texture_input,
        }
    }

    /// Generates the vertex shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The struct of vertex buffered microfacet properties is added as an
    /// input argument to the main vertex shader function and its fields are
    /// assigned to the output struct returned from the function.
    ///
    /// # Returns
    /// The indices of the microfacet property fields in the output struct,
    /// required for accessing the properties in [`generate_fragment_code`].
    pub fn generate_vertex_code(
        &self,
        module: &mut Module,
        vertex_function: &mut Function,
        vertex_output_struct_builder: &mut OutputStructBuilder,
    ) -> MicrofacetVertexOutputFieldIndices {
        let float_type = insert_in_arena(&mut module.types, F32_TYPE);
        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);

        let mut input_struct_builder = InputStructBuilder::new("MaterialProperties", "material");

        let input_diffuse_color_field_idx =
            self.feature_input.diffuse_color_location.map(|location| {
                input_struct_builder.add_field("diffuseColor", vec3_type, location, VECTOR_3_SIZE)
            });

        let input_specular_color_field_idx =
            self.feature_input.specular_color_location.map(|location| {
                input_struct_builder.add_field("specularColor", vec3_type, location, VECTOR_3_SIZE)
            });

        let input_roughness_field_idx = input_struct_builder.add_field(
            "roughness",
            float_type,
            self.feature_input.roughness_location,
            F32_WIDTH,
        );

        let input_struct =
            input_struct_builder.generate_input_code(&mut module.types, vertex_function);

        let output_roughness_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "roughness",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_roughness_field_idx),
            );

        let mut indices = MicrofacetVertexOutputFieldIndices {
            diffuse_color: None,
            specular_color: None,
            roughness: output_roughness_field_idx,
        };

        if let Some(idx) = input_diffuse_color_field_idx {
            indices.diffuse_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "diffuseColor",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr(idx),
                ),
            );
        }

        if let Some(idx) = input_specular_color_field_idx {
            indices.specular_color = Some(
                vertex_output_struct_builder.add_field_with_perspective_interpolation(
                    "specularColor",
                    vec3_type,
                    VECTOR_3_SIZE,
                    input_struct.get_field_expr(idx),
                ),
            );
        }

        indices
    }

    /// Generates the fragment shader code specific to this material by adding
    /// code representation to the given [`naga`] objects.
    ///
    /// The texture and sampler for any material properties sampled from
    /// textured are declared as global variables, and sampling expressions are
    /// generated in the main fragment shader function. These are used together
    /// with material properties passed from the main vertex shader to compute
    /// the contribution to the fragment color for the active light with the
    /// appropriate microfacet BRDF, and the output color is returned from the
    /// main fragment shader function in an output struct.
    pub fn generate_fragment_code(
        &self,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
        light_input_field_indices: Option<&LightVertexOutputFieldIndices>,
        material_input_field_indices: &MicrofacetVertexOutputFieldIndices,
        light_shader_generator: Option<&LightShaderGenerator>,
    ) {
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn computeViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
                return normalize(-vertexPosition);
            }

            fn computeNoDiffuseGGXSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let lightDirectionDotNormalVector = dot(lightDirection, normalVector);
                let viewDirectionDotNormalVector = dot(viewDirection, normalVector);
                let lightDirectionDotHalfVector = dot(lightDirection, halfVector);
                let viewDirectionDotHalfVector = dot(viewDirection, halfVector);
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector,
                    lightDirectionDotHalfVector,
                    viewDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(vec3<f32>(0.0, 0.0, 0.0), specularBRDFTimesPi, lightDirectionDotNormalVector, lightRadiance);
            }

            fn computeLambertianDiffuseGGXSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let lightDirectionDotNormalVector = dot(lightDirection, normalVector);
                let viewDirectionDotNormalVector = dot(viewDirection, normalVector);
                let lightDirectionDotHalfVector = dot(lightDirection, halfVector);
                let viewDirectionDotHalfVector = dot(viewDirection, halfVector);
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                // The Lambertian BRDF (diffuseColor / pi) must be scaled to
                // account for some of the available light being specularly
                // reflected rather than subsurface scattered (Shirley et al.
                // 1997)
                let diffuseBRDFTimesPi = diffuseColor
                    * computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                          specularColor,
                          lightDirectionDotNormalVector,
                          viewDirectionDotNormalVector
                    );

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector,
                    lightDirectionDotHalfVector,
                    viewDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, specularBRDFTimesPi, lightDirectionDotNormalVector, lightRadiance);
            }

            fn computeGGXDiffuseGGXSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let lightDirectionDotNormalVector = dot(lightDirection, normalVector);
                let viewDirectionDotNormalVector = dot(viewDirection, normalVector);
                let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
                let lightDirectionDotHalfVector = dot(lightDirection, halfVector);
                let viewDirectionDotHalfVector = dot(viewDirection, halfVector);
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
                    diffuseColor,
                    specularColor,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector,
                    lightDirectionDotViewDirection,
                    normalVectorDotHalfVector,
                    roughness,
                );

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector,
                    lightDirectionDotHalfVector,
                    viewDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, specularBRDFTimesPi, lightDirectionDotNormalVector, lightRadiance);
            }

            fn computeGGXDiffuseNoSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let lightDirectionDotNormalVector = dot(lightDirection, normalVector);
                let viewDirectionDotNormalVector = dot(viewDirection, normalVector);
                let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let zero = vec3<f32>(0.0, 0.0, 0.0);

                let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
                    diffuseColor,
                    zero,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector,
                    lightDirectionDotViewDirection,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, zero, lightDirectionDotNormalVector, lightRadiance);
            }

            fn computeFresnelReflectanceIncidenceFactor(lightDirectionDotNormalVector: f32) -> f32 {
                let oneMinusLDotN = 1.0 - max(0.0, lightDirectionDotNormalVector);
                return oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN;
            }

            // Computes Fresnel reflectance using the Schlick approximation.
            fn computeFresnelReflectance(
                specularColor: vec3<f32>,
                lightDirectionDotNormalVector: f32,
            ) -> vec3<f32> {
                return specularColor + (1.0 - specularColor) * computeFresnelReflectanceIncidenceFactor(lightDirectionDotNormalVector);
            }

            // Evaluates (approximately) the Smith height-correlated
            // masking-shadowing function divided by (4 *
            // lightDirectionDotNormalVector * viewDirectionDotNormalVector)
            // (Hammon 2017).
            fn computeScaledGGXMaskingShadowingFactor(
                lightDirectionDotHalfVector: f32,
                viewDirectionDotHalfVector: f32,
                lightDirectionDotNormalVector: f32,
                viewDirectionDotNormalVector: f32,
                roughness: f32,
            ) -> f32 {
                return f32(lightDirectionDotHalfVector > 0.0) * f32(viewDirectionDotHalfVector > 0.0) 
                    * 0.5 / mix(
                        2.0 * lightDirectionDotNormalVector * viewDirectionDotNormalVector,
                        lightDirectionDotNormalVector + viewDirectionDotNormalVector,
                        roughness
                    );
            }

            // Evaluates the GGX distribution multiplied by pi.
            fn evaluateGGXDistributionTimesPi(normalVectorDotHalfVector: f32, roughness: f32) -> f32 {
                let roughnessSquared = roughness * roughness;
                let denom = 1.0 + normalVectorDotHalfVector * normalVectorDotHalfVector * (roughnessSquared - 1.0);
                return f32(normalVectorDotHalfVector > 0.0) * roughnessSquared / (denom * denom + 1e-10);
            }

            fn computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                specularColor: vec3<f32>,
                lightDirectionDotNormalVector: f32,
                viewDirectionDotNormalVector: f32,
            ) -> vec3<f32> {
                return 1.05 * (1.0 - specularColor) 
                * (1.0 - computeFresnelReflectanceIncidenceFactor(lightDirectionDotNormalVector)) 
                * (1.0 - computeFresnelReflectanceIncidenceFactor(viewDirectionDotNormalVector));
            }

            // Evaluates a fit to the diffuse BRDF derived from microfacet
            // theory using the GGX normal distribution and the Smith
            // masking-shadowing function (Hammon 2017).
            fn computeDiffuseGGXBRDFTimesPi(
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                lightDirectionDotNormalVector: f32,
                viewDirectionDotNormalVector: f32,
                lightDirectionDotViewDirection: f32,
                normalVectorDotHalfVector: f32,
                roughness: f32,
            ) -> vec3<f32> {
                let diffuseBRDFSmoothComponent = computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                    specularColor,
                    lightDirectionDotNormalVector,
                    viewDirectionDotNormalVector
               );

                let halfOnePlusLightDirectionDotViewDirection = 0.5 * (1.0 + lightDirectionDotViewDirection);
                let diffuseBRDFRoughComponent = halfOnePlusLightDirectionDotViewDirection * (0.9 - 0.4 * halfOnePlusLightDirectionDotViewDirection)
                    * (0.5 + normalVectorDotHalfVector) / (normalVectorDotHalfVector + 1e-10);

                let diffuseBRDFMultiComponent = 0.3641 * roughness;

                return f32(lightDirectionDotNormalVector > 0.0) * f32(viewDirectionDotNormalVector > 0.0)
                    * diffuseColor
                    * (
                          (1.0 - roughness) * diffuseBRDFSmoothComponent
                          + roughness * diffuseBRDFRoughComponent
                          + diffuseColor * diffuseBRDFMultiComponent
                      );
            }

            fn computeSpecularGGXBRDFTimesPi(
                specularColor: vec3<f32>,
                lightDirectionDotNormalVector: f32,
                viewDirectionDotNormalVector: f32,
                lightDirectionDotHalfVector: f32,
                viewDirectionDotHalfVector: f32,
                normalVectorDotHalfVector: f32,
                roughness: f32,
            ) -> vec3<f32> {
                return computeFresnelReflectance(specularColor, lightDirectionDotHalfVector)
                    * computeScaledGGXMaskingShadowingFactor(
                          lightDirectionDotHalfVector,
                          viewDirectionDotHalfVector,
                          lightDirectionDotNormalVector,
                          viewDirectionDotNormalVector,
                          roughness
                    )
                    * evaluateGGXDistributionTimesPi(normalVectorDotHalfVector, roughness);
            }

            fn computeColor(
                diffuseBRDFTimesPi: vec3<f32>,
                specularBRDFTimesPi: vec3<f32>,
                lightDirectionDotNormalVector: f32,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                return (diffuseBRDFTimesPi + specularBRDFTimesPi) * max(0.0, lightDirectionDotNormalVector) * lightRadiance;
            }
        ",
        )
        .unwrap()
        .import_to_module(module);

        let light_shader_generator =
            light_shader_generator.expect("Missing light for microfacet shading");

        let vec3_type = insert_in_arena(&mut module.types, VECTOR_3_TYPE);
        let vec4_type = insert_in_arena(&mut module.types, VECTOR_4_TYPE);

        let position_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .position
                .expect("Missing position for microfacet shading"),
        );

        let normal_vector_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .normal_vector
                .expect("Missing normal vector for microfacet shading"),
        );

        let roughness_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.roughness);

        let (diffuse_color_expr, specular_color_expr) = if let Some(texture_input) =
            self.texture_input
        {
            let (diffuse_color_expr, specular_color_expr) = Self::generate_texture_fragment_code(
                texture_input,
                module,
                fragment_function,
                bind_group_idx,
                fragment_input_struct,
                mesh_input_field_indices,
            );
            (
                diffuse_color_expr,
                specular_color_expr.unwrap_or_else(|| {
                    fragment_input_struct.get_field_expr(
                        material_input_field_indices
                            .specular_color
                            .expect("Missing `specular_color` feature for microfacet material"),
                    )
                }),
            )
        } else {
            (
                fragment_input_struct.get_field_expr(
                    material_input_field_indices
                        .diffuse_color
                        .expect("Missing `diffuse_color` feature for microfacet material"),
                ),
                fragment_input_struct.get_field_expr(
                    material_input_field_indices
                        .specular_color
                        .expect("Missing `specular_color` feature for microfacet material"),
                ),
            )
        };

        let color_ptr_expr = append_to_arena(
            &mut fragment_function.expressions,
            Expression::LocalVariable(append_to_arena(
                &mut fragment_function.local_variables,
                LocalVariable {
                    name: new_name("color"),
                    ty: vec3_type,
                    init: None,
                },
            )),
        );

        let view_dir_expr = SourceCode::generate_call_named(
            fragment_function,
            "viewDirection",
            source_code.functions["computeViewDirection"],
            vec![position_expr],
        );

        let color_computation_function = match self.model {
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::None,
                specular: SpecularMicrofacetShadingModel::GGX,
            } => source_code.functions["computeNoDiffuseGGXSpecularColor"],
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::Lambertian,
                specular: SpecularMicrofacetShadingModel::GGX,
            } => source_code.functions["computeLambertianDiffuseGGXSpecularColor"],
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::GGX,
                specular: SpecularMicrofacetShadingModel::GGX,
            } => source_code.functions["computeGGXDiffuseGGXSpecularColor"],
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::GGX,
                specular: SpecularMicrofacetShadingModel::None,
            } => source_code.functions["computeGGXDiffuseNoSpecularColor"],
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::None,
                specular: SpecularMicrofacetShadingModel::None,
            } => panic!("Microfacet shading with no diffuse and no specular BRDF not supported"),
            MicrofacetShadingModel {
                diffuse: DiffuseMicrofacetShadingModel::Lambertian,
                specular: SpecularMicrofacetShadingModel::None,
            } => panic!(
                "Microfacet shading with Lambertian diffuse and no specular BRDF not supported"
            ),
        };

        let light_color_expr = match (light_shader_generator, light_input_field_indices) {
            (
                LightShaderGenerator::PointLight(PointLightShaderGenerator::ForShading(
                    point_light_shader_generator,
                )),
                None,
            ) => {
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                let (light_dir_expr, light_radiance_expr) = point_light_shader_generator
                    .generate_fragment_shading_code(
                        module,
                        fragment_function,
                        camera_clip_position_expr,
                        position_expr,
                        normal_vector_expr,
                    );

                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    color_computation_function,
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        diffuse_color_expr,
                        specular_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (
                LightShaderGenerator::UnidirectionalLight(
                    UnidirectionalLightShaderGenerator::ForShading(
                        unidirectional_light_shader_generator,
                    ),
                ),
                Some(LightVertexOutputFieldIndices::UnidirectionalLight(
                    unidirectional_light_input_field_indices,
                )),
            ) => {
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                let light_space_position_expr = fragment_input_struct
                    .get_field_expr(unidirectional_light_input_field_indices.light_space_position);

                let light_space_normal_vector_expr = fragment_input_struct.get_field_expr(
                    unidirectional_light_input_field_indices.light_space_normal_vector,
                );

                let (light_dir_expr, light_radiance_expr) = unidirectional_light_shader_generator
                    .generate_fragment_shading_code(
                        module,
                        fragment_function,
                        camera_clip_position_expr,
                        light_space_position_expr,
                        light_space_normal_vector_expr,
                    );

                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    color_computation_function,
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        diffuse_color_expr,
                        specular_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            _ => {
                panic!("Invalid variant of light shader generator and/or light vertex output field indices for microfacet shading");
            }
        };

        push_to_block(
            &mut fragment_function.body,
            Statement::Store {
                pointer: color_ptr_expr,
                value: light_color_expr,
            },
        );

        let output_color_expr = emit_in_func(fragment_function, |function| {
            let color_expr = include_expr_in_func(
                function,
                Expression::Load {
                    pointer: color_ptr_expr,
                },
            );

            include_expr_in_func(
                function,
                Expression::Math {
                    fun: MathFunction::Saturate,
                    arg: color_expr,
                    arg1: None,
                    arg2: None,
                    arg3: None,
                },
            )
        });

        let output_rgba_color_expr = append_unity_component_to_vec3(
            &mut module.types,
            &mut module.constants,
            fragment_function,
            output_color_expr,
        );

        let mut output_struct_builder = OutputStructBuilder::new("FragmentOutput");

        output_struct_builder.add_field(
            "color",
            vec4_type,
            None,
            None,
            VECTOR_4_SIZE,
            output_rgba_color_expr,
        );

        output_struct_builder.generate_output_code(&mut module.types, fragment_function);
    }

    fn generate_texture_fragment_code(
        texture_input: &MicrofacetTextureShaderInput,
        module: &mut Module,
        fragment_function: &mut Function,
        bind_group_idx: &mut u32,
        fragment_input_struct: &InputStruct,
        mesh_input_field_indices: &MeshVertexOutputFieldIndices,
    ) -> (Handle<Expression>, Option<Handle<Expression>>) {
        let bind_group = *bind_group_idx;
        *bind_group_idx += 1;

        let (diffuse_texture_binding, diffuse_sampler_binding) =
            texture_input.diffuse_texture_and_sampler_bindings;

        let diffuse_color_texture = SampledTexture::declare(
            &mut module.types,
            &mut module.global_variables,
            TextureType::Image,
            "diffuseColor",
            bind_group,
            diffuse_texture_binding,
            Some(diffuse_sampler_binding),
            None,
        );

        let texture_coord_expr = fragment_input_struct.get_field_expr(
            mesh_input_field_indices
                .texture_coords
                .expect("No `texture_coords` passed to fixed texture fragment shader"),
        );

        let diffuse_color_sampling_expr =
            diffuse_color_texture.generate_rgb_sampling_expr(fragment_function, texture_coord_expr);

        let specular_color_sampling_expr = texture_input.specular_texture_and_sampler_bindings.map(
            |(specular_texture_binding, specular_sampler_binding)| {
                let specular_color_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image,
                    "specularColor",
                    bind_group,
                    specular_texture_binding,
                    Some(specular_sampler_binding),
                    None,
                );

                specular_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr)
            },
        );

        (diffuse_color_sampling_expr, specular_color_sampling_expr)
    }
}
