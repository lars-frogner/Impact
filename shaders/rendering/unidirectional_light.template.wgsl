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
    cameraSpaceDirection: vec3f,
    perpendicularIlluminanceAndTanAngularRadius: vec4f,
}

struct LightQuantities {
    preExposedIncidentLuminance: vec3f,
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

fn clampToZero(value: f32) -> f32 {
    return max(0.0, value);
}

// ***** Unidirectional lights *****

#if (emulate_area_light_reflection)
fn computeAreaLightQuantities(
    directionOfLight: vec3f,
    lightPerpendicularIlluminance: vec3f,
    tanAngularLightRadius: f32,
    fragmentNormal: vec3f,
    viewDirection: vec3f,
    roughness: f32,
    exposure: f32,
) -> LightQuantities {
    var output: LightQuantities;

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
    fragmentNormal: vec3f,
    viewDirection: vec3f,
    exposure: f32,
) -> LightQuantities {
    var output: LightQuantities;

    output.preExposedIncidentLuminance = exposure * lightPerpendicularIlluminance;

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

    let unidirectionalLight = unidirectionalLights.lights[pushConstants.activeLightIdx];

    let cameraSpaceLightDirection = unidirectionalLight.cameraSpaceDirection;

    let lightPerpendicularIlluminance = unidirectionalLight.perpendicularIlluminanceAndTanAngularRadius.xyz;
    let tanAngularLightRadius = unidirectionalLight.perpendicularIlluminanceAndTanAngularRadius.w;

#if (emulate_area_light_reflection)
    let lightQuantities = computeAreaLightQuantities(
        cameraSpaceLightDirection,
        lightPerpendicularIlluminance,
        tanAngularLightRadius,
        cameraSpaceNormalVector,
        cameraSpaceViewDirection,
        roughness,
        pushConstants.exposure,
    );
#else
    let lightQuantities = computeLightQuantities(
        cameraSpaceLightDirection,
        lightPerpendicularIlluminance,
        cameraSpaceNormalVector,
        cameraSpaceViewDirection,
        pushConstants.exposure,
    );
#endif

    let preExposedReflectedLuminance = computeGGXDiffuseGGXSpecularReflectedLuminance(
        lightQuantities.dots,
        albedo,
        normalIncidenceSpecularReflectance,
        roughness,
        lightQuantities.preExposedIncidentLuminance,
    );

    output.preExposedReflectedLuminance = vec4f(preExposedReflectedLuminance, 1.0);
    return output;
}
