struct PushConstants {
    // Split up inverseWindowDimensions to avoid padding
    inverseWindowWidth: f32,
    inverseWindowHeight: f32,
    activeLightIdx: u32,
    exposure: f32,
}

struct ProjectionUniform {
    projectionMatrix: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
}

struct UnidirectionalLights {
    numLights: u32,
    lights: array<UnidirectionalLight, {{max_light_count}}>,
}

struct UnidirectionalLight {
    cameraToLightRotationQuaternion: vec4f,
    cameraSpaceDirection: vec3f,
    perpendicularIlluminanceAndTanAngularRadius: vec4f,
    orthographicTransforms: array<OrthographicTransform, {{cascade_count}}>,
    partitionDepths: vec4f,
}

struct OrthographicTransform {
    translation: vec3f,
    scaling: vec3f,
}

struct LightQuantities {
    preExposedIncidentLuminance: vec3f,
    lightNDCSpacePosition: vec3f,
    dots: ReflectionDotProducts,
}

struct ReflectionDotProducts {
    VDotN: f32,
    LDotN: f32,
    LDotV: f32,
    NDotH: f32,
    LDotH: f32,
}

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) frustumFarPlanePoint: vec3f,
}

struct FragmentOutput {
    @location(0) preExposedReflectedLuminance: vec4f,
}

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

@group({{linear_depth_texture_group}}) @binding({{linear_depth_texture_binding}})
var linearDepthTexture: texture_2d<f32>;
@group({{linear_depth_texture_group}}) @binding({{linear_depth_sampler_binding}})
var linearDepthSampler: sampler;

@group({{normal_vector_texture_group}}) @binding({{normal_vector_texture_binding}})
var normalVectorTexture: texture_2d<f32>;
@group({{normal_vector_texture_group}}) @binding({{normal_vector_sampler_binding}})
var normalVectorSampler: sampler;

@group({{material_color_texture_group}}) @binding({{material_color_texture_binding}})
var materialColorTexture: texture_2d<f32>;
@group({{material_color_texture_group}}) @binding({{material_color_sampler_binding}})
var materialColorSampler: sampler;

@group({{material_properties_texture_group}}) @binding({{material_properties_texture_binding}})
var materialPropertiesTexture: texture_2d<f32>;
@group({{material_properties_texture_group}}) @binding({{material_properties_sampler_binding}})
var materialPropertiesSampler: sampler;

@group({{light_uniform_group}}) @binding({{light_uniform_binding}})
var<uniform> unidirectionalLights: UnidirectionalLights;

@group({{shadow_map_texture_group}}) @binding({{shadow_map_texture_binding}})
var cascadedShadowMapTexture: texture_depth_2d_array;
@group({{shadow_map_texture_group}}) @binding({{shadow_map_sampler_binding}})
var cascadedShadowMapSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return framebufferPosition.xy * vec2f(pushConstants.inverseWindowWidth, pushConstants.inverseWindowHeight);
}

fn computePositionFromLinearDepth(linearDepth: f32, frustumFarPlanePoint: vec3f) -> vec3f {
    return linearDepth * frustumFarPlanePoint;
}

fn computeCameraSpaceViewDirection(cameraSpacePosition: vec3f) -> vec3f {
    return normalize(-cameraSpacePosition);
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalizedNormalVector(color: vec3f) -> vec3f {
    return normalize(convertNormalColorToNormalVector(color));
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalVector(color: vec3f) -> vec3f {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

fn computeRGBAlbedo(materialColor: vec3f, materialProperties: vec4f) -> vec3f {
    let metalness = materialProperties.z;
    return materialColor * (1.0 - metalness);
}

fn computeRGBSpecularReflectance(materialColor: vec3f, materialProperties: vec4f) -> vec3f {
    let specularReflectance = materialProperties.x;
    let metalness = materialProperties.z;
    return mix(vec3f(specularReflectance), materialColor * specularReflectance, metalness);
}

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn computeScreenTextureCoordsFromNDCXYPosition(ndcXYPosition: vec2f) -> vec2f {
    return vec2f(0.5 * (1.0 + ndcXYPosition.x), 0.5 * (1.0 - ndcXYPosition.y));
}

fn clampToZero(value: f32) -> f32 {
    return max(0.0, value);
}

fn generateRandomAngle(cameraFramebufferXYPosition: vec2f) -> f32 {
    // Multiply noise factor with 2 * pi to get random angle
    return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraFramebufferXYPosition);
}

// Returns a random number between 0 and 1 based on the pixel coordinates
fn generateInterleavedGradientNoiseFactor(cameraFramebufferXYPosition: vec2f) -> f32 {
    let magic = vec3f(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(magic.xy, cameraFramebufferXYPosition)));
}

// ***** Unidirectional lights *****

#if (emulate_area_light_reflection)
fn computeAreaLightQuantities(
    directionOfLight: vec3f,
    lightPerpendicularIlluminance: vec3f,
    tanAngularLightRadius: f32,
    orthographicTranslation: vec3f,
    orthographicScaling: vec3f,
    lightSpacePosition: vec3f,
    lightSpaceNormalVector: vec3f,
    fragmentNormal: vec3f,
    viewDirection: vec3f,
    roughness: f32,
    exposure: f32,
) -> LightQuantities {
    var output: LightQuantities;

    let biasedLightSpacePosition = applyNormalBias(lightSpacePosition, lightSpaceNormalVector);
    output.lightNDCSpacePosition = applyOrthographicProjectionToPosition(
        orthographicTranslation,
        orthographicScaling,
        biasedLightSpacePosition,
    );

    let lightCenterDirection = -directionOfLight;

    let VDotN = dot(viewDirection, fragmentNormal);
    let LDotN = dot(lightCenterDirection, fragmentNormal);
    let LDotV = dot(lightCenterDirection, viewDirection);

    output.dots = determineRepresentativeDirectionForSphericalAreaLight(
        tanAngularLightRadius,
        VDotN,
        LDotN,
        LDotV,
    );

    output.preExposedIncidentLuminance = lightPerpendicularIlluminance
        * (computeLuminanceScalingFactorForSphericalAreaLight(tanAngularLightRadius, roughness)
        * exposure);

    return output;
}
#else
fn computeLightQuantities(
    directionOfLight: vec3f,
    lightPerpendicularIlluminance: vec3f,
    orthographicTranslation: vec3f,
    orthographicScaling: vec3f,
    lightSpacePosition: vec3f,
    lightSpaceNormalVector: vec3f,
    fragmentNormal: vec3f,
    viewDirection: vec3f,
    exposure: f32,
) -> LightQuantities {
    var output: LightQuantities;

    output.preExposedIncidentLuminance = exposure * lightPerpendicularIlluminance;

    let biasedLightSpacePosition = applyNormalBias(lightSpacePosition, lightSpaceNormalVector);
    output.lightNDCSpacePosition = applyOrthographicProjectionToPosition(
        orthographicTranslation,
        orthographicScaling,
        biasedLightSpacePosition,
    );

    let lightCenterDirection = -directionOfLight;

    let VDotN = dot(viewDirection, fragmentNormal);
    let LDotN = dot(lightCenterDirection, fragmentNormal);
    let LDotV = dot(lightCenterDirection, viewDirection);

    let onePlusLDotV = max(1.0 + LDotV, 1e-6);
    let inverseHLength = inverseSqrt(2.0 * onePlusLDotV);
    let NDotH = (LDotN + VDotN) * inverseHLength;
    let LDotH = onePlusLDotV * inverseHLength;

    output.dots.VDotN = VDotN;
    output.dots.LDotN = LDotN;
    output.dots.LDotV = LDotV;
    output.dots.NDotH = NDotH;
    output.dots.LDotH = LDotN;

    return output;
}
#endif // emulate_area_light_reflection

fn applyOrthographicProjectionToPosition(
    orthographicTranslation: vec3f,
    orthographicScaling: vec3f,
    position: vec3f
    ) -> vec3f {
    return (position + orthographicTranslation) * orthographicScaling;
}

fn applyNormalBias(
    lightSpacePosition: vec3f,
    lightSpaceNormalVector: vec3f
    ) -> vec3f {
    let lightDirectionDotNormalVector = -lightSpaceNormalVector.z;
    return lightSpacePosition + lightSpaceNormalVector * clamp(1.0 - lightDirectionDotNormalVector, 0.0, 1.0) * 1e-1;
}

fn determineCascadeIdxMax1(partitionDepths: vec4f, depth: f32) -> i32 {
    return 0;
}

fn determineCascadeIdxMax2(partitionDepths: vec4f, depth: f32) -> i32 {
    var cascadeIdx: i32;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else {
        cascadeIdx = 1;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax3(partitionDepths: vec4f, depth: f32) -> i32 {
    var cascadeIdx: i32;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else {
        cascadeIdx = 2;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax4(partitionDepths: vec4f, depth: f32) -> i32 {
    var cascadeIdx: i32;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else if depth < partitionDepths.z {
        cascadeIdx = 2;
    } else {
        cascadeIdx = 3;
    }
    return cascadeIdx;
}

fn determineCascadeIdxMax5(partitionDepths: vec4f, depth: f32) -> i32 {
    var cascadeIdx: i32;
    if depth < partitionDepths.x {
        cascadeIdx = 0;
    } else if depth < partitionDepths.y {
        cascadeIdx = 1;
    } else if depth < partitionDepths.z {
        cascadeIdx = 2;
    } else if depth < partitionDepths.w {
        cascadeIdx = 3;
    } else {
        cascadeIdx = 4;
    }
    return cascadeIdx;
}

fn computePCSSLightAccessFactor(
    array_index: i32,
    tanAngularRadius: f32,
    worldSpaceToLightNDCSpaceXYScale: f32,
    worldSpaceToLightNDCSpaceZScale: f32,
    cameraFramebufferXYPosition: vec2f,
    centerTextureCoords: vec2f,
    referenceDepth: f32,
) -> f32 {
    let vogelDiskBaseAngle = generateRandomAngle(cameraFramebufferXYPosition);

    let shadowPenumbraExtent = computeShadowPenumbraExtent(
        array_index,
        tanAngularRadius,
        vogelDiskBaseAngle,
        worldSpaceToLightNDCSpaceXYScale,
        worldSpaceToLightNDCSpaceZScale,
        centerTextureCoords,
        referenceDepth,
    );

    if shadowPenumbraExtent < 0.0 {
        return 1.0;
    }

    return computeVogelDiskComparisonSampleAverage(
        array_index,
        vogelDiskBaseAngle,
        worldSpaceToLightNDCSpaceXYScale,
        shadowPenumbraExtent,
        centerTextureCoords,
        referenceDepth,
    );
}

fn computeShadowPenumbraExtent(
    array_index: i32,
    tanAngularRadius: f32,
    vogelDiskBaseAngle: f32,
    worldSpaceToLightNDCSpaceXYScale: f32,
    worldSpaceToLightNDCSpaceZScale: f32,
    centerTextureCoords: vec2f,
    referenceDepth: f32,
) -> f32 {
    let diskRadius: f32 = 0.4 * worldSpaceToLightNDCSpaceXYScale;
    let sampleCount: u32 = 8u;

    let inverseSqrtSampleCount = inverseSqrt(f32(sampleCount));

    var averageOccludingDepth: f32 = 0.0;
    var occludingDepthCount: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleTextureCoords = generateVogelDiskSampleCoords(
            vogelDiskBaseAngle,
            inverseSqrtSampleCount,
            sampleIdx,
        ) * diskRadius + centerTextureCoords;

        let sampledDepth = textureSample(
            cascadedShadowMapTexture,
            cascadedShadowMapSampler,
            sampleTextureCoords,
            array_index,
        );

        if sampledDepth < referenceDepth {
            averageOccludingDepth += sampledDepth;
            occludingDepthCount += 1.0;
        }
    }

    let minPenumbraExtent = 0.01;

    if occludingDepthCount > 0.0 {
        averageOccludingDepth /= occludingDepthCount;
        return max(minPenumbraExtent, tanAngularRadius * (referenceDepth - averageOccludingDepth) / worldSpaceToLightNDCSpaceZScale);
    } else {
        return -1.0;
    }
}

fn computeVogelDiskComparisonSampleAverage(
    array_index: i32,
    vogelDiskBaseAngle: f32,
    worldSpaceToLightNDCSpaceXYScale: f32,
    worldSpaceDiskRadius: f32,
    centerTextureCoords: vec2f,
    referenceDepth: f32,
) -> f32 {
    let sample_density = 800.0;

    let sampleCount = u32(clamp(worldSpaceDiskRadius * sample_density, 3.0, 64.0));

    let diskRadius = worldSpaceDiskRadius * worldSpaceToLightNDCSpaceXYScale;

    let invSampleCount = 1.0 / f32(sampleCount);
    let inverseSqrtSampleCount = sqrt(invSampleCount);

    var sampleAverage: f32 = 0.0;

    for (var sampleIdx: u32 = 0u; sampleIdx < sampleCount; sampleIdx++) {
        let sampleTextureCoords = generateVogelDiskSampleCoords(
            vogelDiskBaseAngle,
            inverseSqrtSampleCount,
            sampleIdx,
        ) * diskRadius + centerTextureCoords;

        let sampledDepth = textureSample(
            cascadedShadowMapTexture,
            cascadedShadowMapSampler,
            sampleTextureCoords,
            array_index,
        );
        if (sampledDepth >= referenceDepth) {
            sampleAverage += invSampleCount;
        }
    }

    return sampleAverage;
}

fn generateVogelDiskSampleCoords(baseAngle: f32, inverseSqrtSampleCount: f32, sampleIdx: u32) -> vec2f {
    let goldenAngle: f32 = 2.4;
    let radius = sqrt(f32(sampleIdx) + 0.5) * inverseSqrtSampleCount;
    let angle = baseAngle + goldenAngle * f32(sampleIdx);
    return vec2f(radius * cos(angle), radius * sin(angle));
}

// ***** Representative point area lighting *****

fn determineRepresentativeDirectionForSphericalAreaLight(
    tanAngularLightRadius: f32,
    VDotN: f32,
    LDotN: f32,
    LDotV: f32,
) -> ReflectionDotProducts {
    var dots: ReflectionDotProducts;
    dots.VDotN = VDotN;

    let cosAngularLightRadius = inverseSqrt(1.0 + tanAngularLightRadius * tanAngularLightRadius);

    // R is the reflection direction
    let LDotR = 2.0 * VDotN * LDotN - LDotV;

    // Check if the reflection vector points to inside the sphere
    if LDotR >= cosAngularLightRadius {
        // If so, tweak light direction to give maximal intensity (NDotH = 1)
        dots.NDotH = 1.0;
        dots.LDotN = VDotN;
        dots.LDotH = LDotN;
        dots.LDotV = 2.0 * VDotN * VDotN - 1.0;
        return dots;
    }

    let sinAngularLightRadius = tanAngularLightRadius * cosAngularLightRadius;

    // T is the direction perpendicular to L pointing towards R:
    // T = (R - LDotR * L) / |R - LDotR * L|

    let sinAngularLightRadiusOverTLength = sinAngularLightRadius * inverseSqrt(max(1.0 - LDotR * LDotR, 1e-6));

    let newLDotNAlongT = (VDotN - LDotR * LDotN) * sinAngularLightRadiusOverTLength;
    let newLDotVAlongT = (2.0 * VDotN * VDotN - 1.0 - LDotR * LDotV) * sinAngularLightRadiusOverTLength;

    let newLDotN = cosAngularLightRadius * LDotN + newLDotNAlongT;
    let newLDotV = cosAngularLightRadius * LDotV + newLDotVAlongT;

    let inverseHLength = inverseSqrt(2.0 * max(1.0 + newLDotV, 1e-6));
    let NDotH = (newLDotN + VDotN) * inverseHLength;
    let LDotH = (1.0 + newLDotV) * inverseHLength;

    dots.LDotN = newLDotN;
    dots.LDotV = newLDotV;
    dots.NDotH = NDotH;
    dots.LDotH = LDotN;

    return dots;
}

fn computeLuminanceScalingFactorForSphericalAreaLight(
    tanAngularLightRadius: f32,
    roughness: f32,
) -> f32 {
    let modifiedRoughness = saturate(roughness + 0.333333333 * tanAngularLightRadius);
    return roughness * roughness / (modifiedRoughness * modifiedRoughness + 1e-4);
}

// ***** Microfacet BRDF *****

fn computeGGXDiffuseGGXSpecularReflectedLuminance(
    dots: ReflectionDotProducts,
    albedo: vec3f,
    normalIncidenceSpecularReflectance: vec3f,
    roughness: f32,
    incidentLuminance: vec3f,
) -> vec3f {
    let clampedVDotN = clampToZero(dots.VDotN);
    let clampedLDotN = clampToZero(dots.LDotN);

    let diffuseBRDFTimesPi = computeDiffuseGGXBRDFTimesPi(
        albedo,
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotV,
        dots.NDotH,
        roughness,
    );

    let specularBRDFTimesPi = computeSpecularGGXBRDFTimesPi(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN,
        dots.LDotH,
        dots.NDotH,
        roughness,
    );

    return computeReflectedLuminanceFromBRDFs(diffuseBRDFTimesPi, specularBRDFTimesPi, clampedLDotN, incidentLuminance);
}

// Evaluates a fit to the diffuse BRDF derived from microfacet
// theory using the GGX normal distribution and the Smith
// masking-shadowing function (Hammon 2017).
fn computeDiffuseGGXBRDFTimesPi(
    albedo: vec3f,
    normalIncidenceSpecularReflectance: vec3f,
    clampedLDotN: f32,
    clampedVDotN: f32,
    LDotV: f32,
    NDotH: f32,
    roughness: f32,
) -> vec3f {
    let diffuseBRDFSmoothComponent = computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
        normalIncidenceSpecularReflectance,
        clampedLDotN,
        clampedVDotN
    );

    var diffuseBRDFRoughComponent = 0.0;
    if abs(NDotH) > 1e-6 {
        let halfOnePlusLDotV = 0.5 * (1.0 + LDotV);
        diffuseBRDFRoughComponent = halfOnePlusLDotV * (0.9 - 0.4 * halfOnePlusLDotV) * (1.0 + 0.5 / NDotH);
    }

    let diffuseBRDFMultiComponent = 0.3641 * roughness;

    return f32(clampedVDotN > 0.0) * albedo * ((1.0 - roughness) * diffuseBRDFSmoothComponent + roughness * diffuseBRDFRoughComponent + albedo * diffuseBRDFMultiComponent);
}

fn computeDiffuseBRDFCorrectionFactorForGGXSpecularReflection(
    normalIncidenceSpecularReflectance: vec3f,
    clampedLDotN: f32,
    clampedVDotN: f32,
) -> vec3f {
    return 1.05 * (1.0 - normalIncidenceSpecularReflectance) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedLDotN)) * (1.0 - computeFresnelReflectanceIncidenceFactor(clampedVDotN));
}

fn computeSpecularGGXBRDFTimesPi(
    normalIncidenceSpecularReflectance: vec3f,
    clampedLDotN: f32,
    clampedVDotN: f32,
    LDotH: f32,
    NDotH: f32,
    roughness: f32,
) -> vec3f {
    return computeFresnelReflectance(normalIncidenceSpecularReflectance, clampToZero(LDotH)) * computeScaledGGXMaskingShadowingFactor(
        clampedLDotN,
        clampedVDotN,
        roughness
    ) * evaluateGGXDistributionTimesPi(NDotH, roughness);
}

// Evaluates (approximately) the Smith height-correlated masking-shadowing
// function divided by (4 * abs(LDotN) * abs(VDotN)) (Hammon 2017).
fn computeScaledGGXMaskingShadowingFactor(
    clampedLDotN: f32,
    clampedVDotN: f32,
    roughness: f32,
) -> f32 {
    return 0.5 / (mix(
        2.0 * clampedLDotN * clampedVDotN,
        clampedLDotN + clampedVDotN,
        roughness
    ) + 1e-6);
}

// Evaluates the GGX distribution multiplied by pi.
fn evaluateGGXDistributionTimesPi(NDotH: f32, roughness: f32) -> f32 {
    let roughnessSquared = roughness * roughness;
    let denom = 1.0 + NDotH * NDotH * (roughnessSquared - 1.0);
    return f32(NDotH > 0.0) * roughnessSquared / (denom * denom + 1e-6);
}

// Computes Fresnel reflectance using the Schlick approximation.
fn computeFresnelReflectance(
    normalIncidenceSpecularReflectance: vec3f,
    clampedLDotN: f32,
) -> vec3f {
    return normalIncidenceSpecularReflectance + (1.0 - normalIncidenceSpecularReflectance) * computeFresnelReflectanceIncidenceFactor(clampedLDotN);
}

fn computeFresnelReflectanceIncidenceFactor(clampedLDotN: f32) -> f32 {
    let oneMinusLDotN = 1.0 - clampedLDotN;
    return oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN * oneMinusLDotN;
}

fn computeReflectedLuminanceFromBRDFs(
    diffuseBRDFTimesPi: vec3f,
    specularBRDFTimesPi: vec3f,
    clampedLDotN: f32,
    incidentLuminance: vec3f,
) -> vec3f {
    // The factor 0.318309886 is 1 / pi
    return (diffuseBRDFTimesPi + specularBRDFTimesPi) * (clampedLDotN * 0.318309886) * incidentLuminance;
}

@vertex
fn mainVS(
    @builtin(vertex_index) vertexIndex: u32,
    @location({{position_location}}) modelSpacePosition: vec3f
) -> VertexOutput {
    var output: VertexOutput;
    output.projectedPosition = vec4f(modelSpacePosition, 1.0);
    output.frustumFarPlanePoint = projectionUniform.frustumFarPlaneCorners[vertexIndex].xyz;
    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let textureCoords = convertFramebufferPositionToScreenTextureCoords(input.projectedPosition);

    let depth = textureSampleLevel(linearDepthTexture, linearDepthSampler, textureCoords, 0.0).r;
    let cameraSpacePosition = computePositionFromLinearDepth(depth, input.frustumFarPlanePoint);
    let cameraSpaceViewDirection = computeCameraSpaceViewDirection(cameraSpacePosition);

    let normalColor = textureSampleLevel(normalVectorTexture, normalVectorSampler, textureCoords, 0.0).rgb;
    let cameraSpaceNormalVector = convertNormalColorToNormalizedNormalVector(normalColor);

    let materialColor = textureSampleLevel(materialColorTexture, materialColorSampler, textureCoords, 0.0).rgb;
    let materialProperties = textureSampleLevel(materialPropertiesTexture, materialPropertiesSampler, textureCoords, 0.0);

    let albedo = computeRGBAlbedo(materialColor, materialProperties);
    let normalIncidenceSpecularReflectance = computeRGBSpecularReflectance(materialColor, materialProperties);
    let roughness = materialProperties.y;

    // Note: `var` is required here instead of `let` to make the
    // `orthographicTransforms` array indexable with a dynamic index
    var unidirectionalLight = unidirectionalLights.lights[pushConstants.activeLightIdx];

    let cameraSpaceLightDirection = unidirectionalLight.cameraSpaceDirection;

    let lightPerpendicularIlluminance = unidirectionalLight.perpendicularIlluminanceAndTanAngularRadius.xyz;
    let tanAngularLightRadius = unidirectionalLight.perpendicularIlluminanceAndTanAngularRadius.w;

    let cascadeIdx = determineCascadeIdxMax{{cascade_count}}(unidirectionalLight.partitionDepths, depth);

    let lightOrthographicTransform = unidirectionalLight.orthographicTransforms[cascadeIdx];
    let lightOrthographicTranslation = lightOrthographicTransform.translation;
    let lightOrthographicScaling = lightOrthographicTransform.scaling;

    let cameraToLightSpaceRotationQuaternion = unidirectionalLight.cameraToLightRotationQuaternion;
    let lightSpacePosition = rotateVectorWithQuaternion(cameraToLightSpaceRotationQuaternion, cameraSpacePosition);
    let lightSpaceNormalVector = rotateVectorWithQuaternion(cameraToLightSpaceRotationQuaternion, cameraSpaceNormalVector);

#if (emulate_area_light_reflection)
    let lightQuantities = computeAreaLightQuantities(
        cameraSpaceLightDirection,
        lightPerpendicularIlluminance,
        tanAngularLightRadius,
        lightOrthographicTranslation,
        lightOrthographicScaling,
        lightSpacePosition,
        lightSpaceNormalVector,
        cameraSpaceNormalVector,
        cameraSpaceViewDirection,
        roughness,
        pushConstants.exposure,
    );
#else
    let lightQuantities = computeLightQuantities(
        cameraSpaceLightDirection,
        lightPerpendicularIlluminance,
        lightOrthographicTranslation,
        lightOrthographicScaling,
        lightSpacePosition,
        lightSpaceNormalVector,
        cameraSpaceNormalVector,
        cameraSpaceViewDirection,
        pushConstants.exposure,
    );
#endif

    let shadowMapTextureCoords = computeScreenTextureCoordsFromNDCXYPosition(lightQuantities.lightNDCSpacePosition.xy);

    let lightAccessFactor = computePCSSLightAccessFactor(
        cascadeIdx,
        tanAngularLightRadius,
        lightOrthographicScaling.x,
        -lightOrthographicScaling.z,
        input.projectedPosition.xy,
        shadowMapTextureCoords,
        lightQuantities.lightNDCSpacePosition.z,
    );

    let preExposedReflectedLuminance = computeGGXDiffuseGGXSpecularReflectedLuminance(
        lightQuantities.dots,
        albedo,
        normalIncidenceSpecularReflectance,
        roughness,
        lightAccessFactor * lightQuantities.preExposedIncidentLuminance,
    );

    output.preExposedReflectedLuminance = vec4f(preExposedReflectedLuminance, 1.0);
    return output;
}
