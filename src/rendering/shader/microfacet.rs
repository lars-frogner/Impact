//! Generation of shaders for microfacet materials.

use super::{
    append_to_arena, append_unity_component_to_vec3, emit_in_func, include_expr_in_func,
    insert_in_arena, new_name, push_to_block, InputStruct, InputStructBuilder,
    LightShaderGenerator, LightVertexOutputFieldIndices, MeshVertexOutputFieldIndices,
    OmnidirectionalLightShaderGenerator, OutputStructBuilder, SampledTexture, SourceCode,
    TextureType, UnidirectionalLightShaderGenerator, F32_TYPE, F32_WIDTH, VECTOR_3_SIZE,
    VECTOR_3_TYPE, VECTOR_4_SIZE, VECTOR_4_TYPE,
};
use naga::{Expression, Function, LocalVariable, MathFunction, Module, Statement};

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
/// material properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetFeatureShaderInput {
    /// Vertex attribute location for the instance feature representing diffuse
    /// color.
    pub diffuse_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing specular
    /// color.
    pub specular_color_location: Option<u32>,
    /// Vertex attribute location for the instance feature representing
    /// roughness.
    pub roughness_location: u32,
    /// Vertex attribute location for the instance feature representing the
    /// displacement scale for parallax mapping.
    pub parallax_displacement_scale_location: u32,
}

/// Input description specifying the bindings of textures for microfacet
/// properties.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MicrofacetTextureShaderInput {
    /// Bind group bindings of the diffuse color texture and its sampler.
    pub diffuse_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the specular color texture and its sampler.
    pub specular_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the roughness texture and its sampler.
    pub roughness_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the normal map texture and its sampler.
    pub normal_map_texture_and_sampler_bindings: Option<(u32, u32)>,
    /// Bind group bindings of the height map texture and its sampler.
    pub height_map_texture_and_sampler_bindings: Option<(u32, u32)>,
}

/// Shader generator for a microfacet material.
#[derive(Clone, Debug)]
pub struct MicrofacetShaderGenerator<'a> {
    model: &'a MicrofacetShadingModel,
    feature_input: &'a MicrofacetFeatureShaderInput,
    texture_input: &'a MicrofacetTextureShaderInput,
}

/// Indices of the fields holding the various microfacet properties in the
/// vertex shader output struct.
#[derive(Clone, Debug)]
pub struct MicrofacetVertexOutputFieldIndices {
    diffuse_color: Option<usize>,
    specular_color: Option<usize>,
    roughness: usize,
    parallax_displacement_scale: usize,
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
        texture_input: &'a MicrofacetTextureShaderInput,
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

        let input_parallax_displacement_scale_field_idx = input_struct_builder.add_field(
            "parallaxDisplacementScale",
            float_type,
            self.feature_input.parallax_displacement_scale_location,
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

        let output_parallax_displacement_scale_field_idx = vertex_output_struct_builder
            .add_field_with_perspective_interpolation(
                "parallaxDisplacementScale",
                float_type,
                F32_WIDTH,
                input_struct.get_field_expr(input_parallax_displacement_scale_field_idx),
            );

        let mut indices = MicrofacetVertexOutputFieldIndices {
            diffuse_color: None,
            specular_color: None,
            roughness: output_roughness_field_idx,
            parallax_displacement_scale: output_parallax_displacement_scale_field_idx,
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
        // TODO: Diffuse microfacet BRDF evaluates to zero when
        // viewDirectionDotNormalVector <= 0, which may happen for visible
        // fragments when using normal mapping, leading to dark artifacts
        let source_code = SourceCode::from_wgsl_source(
            "\
            fn convertNormalMapColorToNormalVector(color: vec3<f32>) -> vec3<f32> {
                // May require normalization depending on filtering
                return 2.0 * (color - 0.5);
            }

            fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }

            fn rotateVectorWithInverseOfQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
                let tmp = 2.0 * cross(quaternion.xyz, vector);
                return vector - quaternion.w * tmp + cross(quaternion.xyz, tmp);
            }

            fn computeViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
                return normalize(-vertexPosition);
            }

            fn computeParallaxMappedTextureCoordinates(
                heightTexture: texture_2d<f32>,
                heightSampler: sampler,
                displacementScale: f32,
                originalTextureCoordinates: vec2<f32>,
                tangentToCameraSpaceRotationQuaternion: vec4<f32>,
                cameraSpaceViewDirection: vec3<f32>,
            ) -> vec2<f32> {
                let tangentSpaceViewDirection = rotateVectorWithInverseOfQuaternion(tangentToCameraSpaceRotationQuaternion, cameraSpaceViewDirection);

                let layerDepth = displacementScale / mix(64.0, 8.0, max(0.0, tangentSpaceViewDirection.z));

                let textureCoordOffsetVector = tangentSpaceViewDirection.xy * (layerDepth / tangentSpaceViewDirection.z);

                var currentDepth = 0.0;
                var prevTextureCoords = originalTextureCoordinates;
                var currentTextureCoords = originalTextureCoordinates;

                let sampledHeight = textureSample(heightTexture, heightSampler, currentTextureCoords).r;
                var currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
                var prevSampledDepth = currentSampledDepth;

                while currentSampledDepth > currentDepth {
                    prevTextureCoords = currentTextureCoords;
                    prevSampledDepth = currentSampledDepth;

                    currentTextureCoords -= textureCoordOffsetVector;
                    currentDepth += layerDepth;

                    let sampledHeight = textureSample(heightTexture, heightSampler, currentTextureCoords).r;
                    currentSampledDepth = (1.0 - sampledHeight) * displacementScale;
                }

                let currentDepthDiff = currentSampledDepth - currentDepth;
                let prevDepthDiff = prevSampledDepth - (currentDepth - layerDepth);

                let interpWeightForZeroDepthDiff = currentDepthDiff / (currentDepthDiff - prevDepthDiff);

                return mix(currentTextureCoords, prevTextureCoords, interpWeightForZeroDepthDiff);
            }

            fn obtainNormalFromHeightMap(
                heightTexture: texture_2d<f32>,
                heightSampler: sampler,
                textureCoords: vec2<f32>,
            ) -> vec3<f32> {
                let textureDims = textureDimensions(heightTexture);
                
                let offsetU = vec2<f32>(1.0 / f32(textureDims.x), 0.0);
                let offsetV = vec2<f32>(0.0, 1.0 / f32(textureDims.y));

                let heightDownU = textureSample(heightTexture, heightSampler, textureCoords - offsetU).r;
                let heightUpU = textureSample(heightTexture, heightSampler, textureCoords + offsetU).r;
                let heightDownV = textureSample(heightTexture, heightSampler, textureCoords - offsetV).r;
                let heightUpV = textureSample(heightTexture, heightSampler, textureCoords + offsetV).r;
                
                return normalize(vec3<f32>(heightDownU - heightUpU, heightDownV - heightUpV, 2.0));
            }

            fn computeNoDiffuseGGXSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                specularColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
                let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
                let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector,
                    clampedLightDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(vec3<f32>(0.0, 0.0, 0.0), specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
            }

            fn computeGGXRoughnessFromSampledRoughness(sampledRoughness: f32, roughnessScale: f32) -> f32 {
                // Square sampled roughness (assumed perceptually linear) to get
                // GGX roughness, then apply scaling
                return sampledRoughness * sampledRoughness * roughnessScale;
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

                let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
                let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
                let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                // The Lambertian BRDF (diffuseColor / pi) must be scaled to
                // account for some of the available light being specularly
                // reflected rather than subsurface scattered (Shirley et al.
                // 1997)
                let diffuseBRDFTimesPi = diffuseColor
                    * computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                          specularColor,
                          clampedLightDirectionDotNormalVector,
                          clampedViewDirectionDotNormalVector
                    );

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector,
                    clampedLightDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
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

                let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
                let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
                let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
                let clampedLightDirectionDotHalfVector = max(0.0, dot(lightDirection, halfVector));
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
                    diffuseColor,
                    specularColor,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector,
                    lightDirectionDotViewDirection,
                    normalVectorDotHalfVector,
                    roughness,
                );

                let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
                    specularColor,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector,
                    clampedLightDirectionDotHalfVector,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLightDirectionDotNormalVector, lightRadiance);
            }

            fn computeGGXDiffuseNoSpecularColor(
                viewDirection: vec3<f32>,
                normalVector: vec3<f32>,
                diffuseColor: vec3<f32>,
                roughness: f32,
                lightDirection: vec3<f32>,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                let halfVector = normalize((lightDirection + viewDirection));

                let clampedLightDirectionDotNormalVector = max(0.0, dot(lightDirection, normalVector));
                let clampedViewDirectionDotNormalVector = max(0.0, dot(viewDirection, normalVector));
                let lightDirectionDotViewDirection = dot(lightDirection, viewDirection);
                let normalVectorDotHalfVector = dot(normalVector, halfVector);

                let zero = vec3<f32>(0.0, 0.0, 0.0);

                let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
                    diffuseColor,
                    zero,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector,
                    lightDirectionDotViewDirection,
                    normalVectorDotHalfVector,
                    roughness,
                );

                return computeColor(diffuseBRDFTimesPi, zero, clampedLightDirectionDotNormalVector, lightRadiance);
            }

            fn computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector: f32) -> f32 {
                let oneMinusLDotN = 1.0 - max(0.0, clampedLightDirectionDotNormalVector);
                return oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN;
            }

            // Computes Fresnel reflectance using the Schlick approximation.
            fn computeFresnelReflectance(
                specularColor: vec3<f32>,
                clampedLightDirectionDotNormalVector: f32,
            ) -> vec3<f32> {
                return specularColor + (1.0 - specularColor) * computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector);
            }

            // Evaluates (approximately) the Smith height-correlated
            // masking-shadowing function divided by (4 *
            // abs(lightDirectionDotNormalVector) *
            // abs(viewDirectionDotNormalVector)) (Hammon 2017).
            fn computeScaledGGXMaskingShadowingFactor(
                clampedLightDirectionDotNormalVector: f32,
                clampedViewDirectionDotNormalVector: f32,
                roughness: f32,
            ) -> f32 {
                return 0.5 / (mix(
                        2.0 * clampedLightDirectionDotNormalVector * clampedViewDirectionDotNormalVector,
                        clampedLightDirectionDotNormalVector + clampedViewDirectionDotNormalVector,
                        roughness
                    ) + 1e-6);
            }

            // Evaluates the GGX distribution multiplied by pi.
            fn evaluateGGXDistributionTimesPi(normalVectorDotHalfVector: f32, roughness: f32) -> f32 {
                let roughnessSquared = roughness * roughness;
                let denom = 1.0 + normalVectorDotHalfVector * normalVectorDotHalfVector * (roughnessSquared - 1.0);
                return f32(normalVectorDotHalfVector > 0.0) * roughnessSquared / (denom * denom + 1e-6);
            }

            fn computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                specularColor: vec3<f32>,
                clampedLightDirectionDotNormalVector: f32,
                clampedViewDirectionDotNormalVector: f32,
            ) -> vec3<f32> {
                return 1.05 * (1.0 - specularColor) 
                * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedLightDirectionDotNormalVector)) 
                * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedViewDirectionDotNormalVector));
            }

            // Evaluates a fit to the diffuse BRDF derived from microfacet
            // theory using the GGX normal distribution and the Smith
            // masking-shadowing function (Hammon 2017).
            fn computeDiffuseGGXBRDFTimesPi(
                diffuseColor: vec3<f32>,
                specularColor: vec3<f32>,
                clampedLightDirectionDotNormalVector: f32,
                clampedViewDirectionDotNormalVector: f32,
                lightDirectionDotViewDirection: f32,
                normalVectorDotHalfVector: f32,
                roughness: f32,
            ) -> vec3<f32> {
                let diffuseBRDFSmoothComponent = computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
                    specularColor,
                    clampedLightDirectionDotNormalVector,
                    clampedViewDirectionDotNormalVector
               );

                let halfOnePlusLightDirectionDotViewDirection = 0.5 * (1.0 + lightDirectionDotViewDirection);
                let diffuseBRDFRoughComponent = halfOnePlusLightDirectionDotViewDirection * (0.9 - 0.4 * halfOnePlusLightDirectionDotViewDirection)
                    * (1.0 + 0.5 / (normalVectorDotHalfVector + 1e-6));

                let diffuseBRDFMultiComponent = 0.3641 * roughness;

                return f32(clampedViewDirectionDotNormalVector > 0.0)
                    * diffuseColor
                    * (
                          (1.0 - roughness) * diffuseBRDFSmoothComponent
                          + roughness * diffuseBRDFRoughComponent
                          + diffuseColor * diffuseBRDFMultiComponent
                      );
            }

            fn computeSpecularGGXBRDFTimesPi(
                specularColor: vec3<f32>,
                clampedLightDirectionDotNormalVector: f32,
                clampedViewDirectionDotNormalVector: f32,
                clampedLightDirectionDotHalfVector: f32,
                normalVectorDotHalfVector: f32,
                roughness: f32,
            ) -> vec3<f32> {
                return computeFresnelReflectance(specularColor, clampedLightDirectionDotHalfVector)
                    * computeScaledGGXMaskingShadowingFactor(
                          clampedLightDirectionDotNormalVector,
                          clampedViewDirectionDotNormalVector,
                          roughness
                    )
                    * evaluateGGXDistributionTimesPi(normalVectorDotHalfVector, roughness);
            }

            fn computeColor(
                diffuseBRDFTimesPi: vec3<f32>,
                specularBRDFTimesPi: vec3<f32>,
                clampedLightDirectionDotNormalVector: f32,
                lightRadiance: vec3<f32>,
            ) -> vec3<f32> {
                return (diffuseBRDFTimesPi + specularBRDFTimesPi) * clampedLightDirectionDotNormalVector * lightRadiance;
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

        let fixed_roughness_value_expr =
            fragment_input_struct.get_field_expr(material_input_field_indices.roughness);

        let view_dir_expr = SourceCode::generate_call_named(
            fragment_function,
            "viewDirection",
            source_code.functions["computeViewDirection"],
            vec![position_expr],
        );

        let (bind_group, mut texture_coord_expr) = if !self.texture_input.is_empty() {
            let texture_coord_expr = fragment_input_struct.get_field_expr(
                mesh_input_field_indices
                    .texture_coords
                    .expect("No `texture_coords` passed to textured microfacet fragment shader"),
            );

            let bind_group = *bind_group_idx;
            *bind_group_idx += 1;

            (bind_group, Some(texture_coord_expr))
        } else {
            (*bind_group_idx, None)
        };

        let normal_vector_expr =
            if let Some((height_map_texture_binding, height_map_sampler_binding)) =
                self.texture_input.height_map_texture_and_sampler_bindings
            {
                let height_map_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image,
                    "heightMap",
                    bind_group,
                    height_map_texture_binding,
                    Some(height_map_sampler_binding),
                    None,
                );

                let (height_map_texture_expr, height_map_sampler_expr) = height_map_texture
                    .generate_texture_and_sampler_expressions(fragment_function, false);

                let parallax_displacement_scale_expr = fragment_input_struct
                    .get_field_expr(material_input_field_indices.parallax_displacement_scale);

                let tangent_space_quaternion_expr = fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .tangent_space_quaternion
                        .expect("Missing tangent space quaternion for microfacet parallax mapping"),
                );

                texture_coord_expr = Some(SourceCode::generate_call_named(
                    fragment_function,
                    "parallaxMappedTextureCoords",
                    source_code.functions["computeParallaxMappedTextureCoordinates"],
                    vec![
                        height_map_texture_expr,
                        height_map_sampler_expr,
                        parallax_displacement_scale_expr,
                        texture_coord_expr.unwrap(),
                        tangent_space_quaternion_expr,
                        view_dir_expr,
                    ],
                ));

                let tangent_space_normal_vector_expr = SourceCode::generate_call_named(
                    fragment_function,
                    "tangentSpaceNormalVector",
                    source_code.functions["obtainNormalFromHeightMap"],
                    vec![
                        height_map_texture_expr,
                        height_map_sampler_expr,
                        texture_coord_expr.unwrap(),
                    ],
                );

                SourceCode::generate_call_named(
                    fragment_function,
                    "normalVector",
                    source_code.functions["rotateVectorWithQuaternion"],
                    vec![
                        tangent_space_quaternion_expr,
                        tangent_space_normal_vector_expr,
                    ],
                )
            } else if let Some((normal_map_texture_binding, normal_map_sampler_binding)) =
                self.texture_input.normal_map_texture_and_sampler_bindings
            {
                let normal_map_texture = SampledTexture::declare(
                    &mut module.types,
                    &mut module.global_variables,
                    TextureType::Image,
                    "normalMap",
                    bind_group,
                    normal_map_texture_binding,
                    Some(normal_map_sampler_binding),
                    None,
                );

                let normal_map_color_expr = normal_map_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap());

                let tangent_space_normal_expr = SourceCode::generate_call_named(
                    fragment_function,
                    "tangentSpaceNormalVector",
                    source_code.functions["convertNormalMapColorToNormalVector"],
                    vec![normal_map_color_expr],
                );

                let tangent_space_quaternion_expr = fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .tangent_space_quaternion
                        .expect("Missing tangent space quaternion for microfacet normal mapping"),
                );

                SourceCode::generate_call_named(
                    fragment_function,
                    "normalVector",
                    source_code.functions["rotateVectorWithQuaternion"],
                    vec![tangent_space_quaternion_expr, tangent_space_normal_expr],
                )
            } else {
                fragment_input_struct.get_field_expr(
                    mesh_input_field_indices
                        .normal_vector
                        .expect("Missing normal vector for microfacet shading"),
                )
            };

        let diffuse_color_expr = self
            .texture_input
            .diffuse_texture_and_sampler_bindings
            .map(|(diffuse_texture_binding, diffuse_sampler_binding)| {
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

                diffuse_color_texture
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap())
            })
            .or_else(|| {
                material_input_field_indices
                    .diffuse_color
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let specular_color_expr = self
            .texture_input
            .specular_texture_and_sampler_bindings
            .map(|(specular_texture_binding, specular_sampler_binding)| {
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
                    .generate_rgb_sampling_expr(fragment_function, texture_coord_expr.unwrap())
            })
            .or_else(|| {
                material_input_field_indices
                    .specular_color
                    .map(|idx| fragment_input_struct.get_field_expr(idx))
            });

        let roughness_expr = self
            .texture_input
            .roughness_texture_and_sampler_bindings
            .map_or(
                fixed_roughness_value_expr,
                |(roughness_texture_binding, roughness_sampler_binding)| {
                    let roughness_texture = SampledTexture::declare(
                        &mut module.types,
                        &mut module.global_variables,
                        TextureType::Image,
                        "roughness",
                        bind_group,
                        roughness_texture_binding,
                        Some(roughness_sampler_binding),
                        None,
                    );

                    let roughness_texture_value_expr = roughness_texture
                        .generate_single_channel_sampling_expr(
                            fragment_function,
                            texture_coord_expr.unwrap(),
                            0,
                        );

                    SourceCode::generate_call_named(
                        fragment_function,
                        "ggxRoughness",
                        source_code.functions["computeGGXRoughnessFromSampledRoughness"],
                        // Use fixed roughness as scale for roughness sampled from texture
                        vec![roughness_texture_value_expr, fixed_roughness_value_expr],
                    )
                },
            );

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

        let compute_color = |fragment_function, light_dir_expr, light_radiance_expr| match (
            self.model,
            diffuse_color_expr,
            specular_color_expr,
        ) {
            (&MicrofacetShadingModel::GGX_DIFFUSE_NO_SPECULAR, Some(diffuse_color_expr), None) => {
                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    source_code.functions["computeGGXDiffuseNoSpecularColor"],
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        diffuse_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (&MicrofacetShadingModel::NO_DIFFUSE_GGX_SPECULAR, None, Some(specular_color_expr)) => {
                SourceCode::generate_call_named(
                    fragment_function,
                    "lightColor",
                    source_code.functions["computeNoDiffuseGGXSpecularColor"],
                    vec![
                        view_dir_expr,
                        normal_vector_expr,
                        specular_color_expr,
                        roughness_expr,
                        light_dir_expr,
                        light_radiance_expr,
                    ],
                )
            }
            (
                &MicrofacetShadingModel::LAMBERTIAN_DIFFUSE_GGX_SPECULAR,
                Some(diffuse_color_expr),
                Some(specular_color_expr),
            ) => SourceCode::generate_call_named(
                fragment_function,
                "lightColor",
                source_code.functions["computeLambertianDiffuseGGXSpecularColor"],
                vec![
                    view_dir_expr,
                    normal_vector_expr,
                    diffuse_color_expr,
                    specular_color_expr,
                    roughness_expr,
                    light_dir_expr,
                    light_radiance_expr,
                ],
            ),
            (
                &MicrofacetShadingModel::GGX_DIFFUSE_GGX_SPECULAR,
                Some(diffuse_color_expr),
                Some(specular_color_expr),
            ) => SourceCode::generate_call_named(
                fragment_function,
                "lightColor",
                source_code.functions["computeGGXDiffuseGGXSpecularColor"],
                vec![
                    view_dir_expr,
                    normal_vector_expr,
                    diffuse_color_expr,
                    specular_color_expr,
                    roughness_expr,
                    light_dir_expr,
                    light_radiance_expr,
                ],
            ),
            (_, None, None) => panic!("No diffuse or specular color for microfacet shader"),
            _ => panic!("Invalid combinations of microfacet shading models"),
        };

        let light_color_expr = match (light_shader_generator, light_input_field_indices) {
            (
                LightShaderGenerator::OmnidirectionalLight(
                    OmnidirectionalLightShaderGenerator::ForShading(
                        omnidirectional_light_shader_generator,
                    ),
                ),
                None,
            ) => {
                let camera_clip_position_expr =
                    fragment_input_struct.get_field_expr(mesh_input_field_indices.clip_position);

                let (light_dir_expr, light_radiance_expr) = omnidirectional_light_shader_generator
                    .generate_fragment_shading_code(
                        module,
                        fragment_function,
                        camera_clip_position_expr,
                        position_expr,
                        normal_vector_expr,
                    );

                compute_color(fragment_function, light_dir_expr, light_radiance_expr)
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

                compute_color(fragment_function, light_dir_expr, light_radiance_expr)
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
}

impl MicrofacetTextureShaderInput {
    fn is_empty(&self) -> bool {
        self.diffuse_texture_and_sampler_bindings.is_none()
            && self.specular_texture_and_sampler_bindings.is_none()
            && self.roughness_texture_and_sampler_bindings.is_none()
            && self.normal_map_texture_and_sampler_bindings.is_none()
            && self.height_map_texture_and_sampler_bindings.is_none()
    }
}
