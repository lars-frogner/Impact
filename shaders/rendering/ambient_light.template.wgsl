struct PushConstants {
    inverseWindowDimensions: vec2f,
    exposure: f32,
}

struct ProjectionUniform {
    projectionMatrix: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
}

struct AmbientLights {
    numLights: u32,
    lights: array<AmbientLight, {{max_light_count}}>,
}

struct AmbientLight {
    luminance: vec3f,
}

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) frustumFarPlanePoint: vec3f,
}

struct FragmentOutput {
    @location(0) preExposedEmissiveLuminance: vec4f,
    @location(1) preExposedAmbientReflectedLuminance: vec4f,
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
var<uniform> ambientLights: AmbientLights;

@group({{specular_reflectance_lookup_texture_group}}) @binding({{specular_reflectance_lookup_texture_binding}})
var specularGGXReflectanceLookupTexture: texture_2d_array<f32>;
@group({{specular_reflectance_lookup_texture_group}}) @binding({{specular_reflectance_lookup_sampler_binding}})
var specularGGXReflectanceLookupSampler: sampler;

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2f {
    return (framebufferPosition.xy * pushConstants.inverseWindowDimensions);
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

fn computeRGBEmissiveLuminance(materialColor: vec3f, materialProperties: vec4f) -> vec3f {
    let emissiveLuminance = materialProperties.w;
    return materialColor * emissiveLuminance;
}

fn computeAmbientDiffuseReflectedLuminanceForLambertian(
    albedo: vec3f,
    ambientLuminance: vec3f,
    ) -> vec3f {
    // Same as (albedo / pi) * ambientIlluminance
    return albedo * ambientLuminance;
}

fn computeAmbientSpecularReflectedLuminanceForGGX(
    viewDirection: vec3f,
    normalVector: vec3f,
    normalIncidenceSpecularReflectance: vec3f,
    roughness: f32,
    ambientLuminance: vec3f,
) -> vec3f {
    let viewDirectionDotNormalVector = dot(viewDirection, normalVector);

    if viewDirectionDotNormalVector > 0.0 {
        // Mip level must be explicit since it can not be computed automatically
        // inside non-uniform control flow. It should always be zero anyway.
        let mipLevel = 0.0;

        let textureCoords = vec2f(viewDirectionDotNormalVector, roughness);

        let reflectanceForZeroNormalIncidenceReflectance = textureSampleLevel(
            specularGGXReflectanceLookupTexture,
            specularGGXReflectanceLookupSampler,
            textureCoords,
            0,
            mipLevel
        ).r;

        let reflectanceForUnityNormalIncidenceReflectance = textureSampleLevel(
            specularGGXReflectanceLookupTexture,
            specularGGXReflectanceLookupSampler,
            textureCoords,
            1,
            mipLevel
        ).r;

        let reflectance = (1.0 - normalIncidenceSpecularReflectance) * reflectanceForZeroNormalIncidenceReflectance + normalIncidenceSpecularReflectance * reflectanceForUnityNormalIncidenceReflectance;

        return reflectance * ambientLuminance;
    } else {
        return vec3f(0.0);
    }
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
    let position = computePositionFromLinearDepth(depth, input.frustumFarPlanePoint);
    let viewDirection = computeCameraSpaceViewDirection(position);

    let normalColor = textureSampleLevel(normalVectorTexture, normalVectorSampler, textureCoords, 0.0).rgb;
    let normalVector = convertNormalColorToNormalizedNormalVector(normalColor);

    let materialColor = textureSampleLevel(materialColorTexture, materialColorSampler, textureCoords, 0.0).rgb;
    let materialProperties = textureSampleLevel(materialPropertiesTexture, materialPropertiesSampler, textureCoords, 0.0);

    let albedo = computeRGBAlbedo(materialColor, materialProperties);
    let normalIncidenceSpecularReflectance = computeRGBSpecularReflectance(materialColor, materialProperties);
    let roughness = materialProperties.y;
    // Emissive luminance is already pre-exposed from the geometry pass
    let preExposedEmissiveLuminance = computeRGBEmissiveLuminance(materialColor, materialProperties);

    var ambientLuminance = vec3f(0.0);
    for (var lightIdx: u32 = 0u; lightIdx < ambientLights.numLights; lightIdx++) {
        ambientLuminance += ambientLights.lights[lightIdx].luminance;
    }

    let ambientDiffuseReflectedLuminance = computeAmbientDiffuseReflectedLuminanceForLambertian(
        albedo,
        ambientLuminance,
    );

    let ambientSpecularReflectedLuminance = computeAmbientSpecularReflectedLuminanceForGGX(
        viewDirection,
        normalVector,
        normalIncidenceSpecularReflectance,
        roughness,
        ambientLuminance,
    );

    let ambientReflectedLuminance = ambientDiffuseReflectedLuminance + ambientSpecularReflectedLuminance;

    let preExposedAmbientReflectedLuminance = ambientReflectedLuminance * pushConstants.exposure;

    output.preExposedEmissiveLuminance = vec4f(preExposedEmissiveLuminance, 1.0);
    output.preExposedAmbientReflectedLuminance = vec4f(preExposedAmbientReflectedLuminance, 1.0);
    return output;
}
