struct PushConstants {
    activeLightIdx: u32,
}

struct ModelToCubemapFaceSpaceTransform {
    @location({{model_light_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{model_light_transform_translation_location}}) translationAndScaling: vec4f,
}

struct OmnidirectionalLights {
    numLights: u32,
    lights: array<OmnidirectionalLight, {{max_light_count}}>,
}

struct OmnidirectionalLight {
    cameraToLightRotationQuaternion: vec4f,
    cubemapFaceSpacePosition: vec3f,
    luminousIntensityAndEmissionRadius: vec4f,
    distanceMapping: DistanceMapping,
}

struct DistanceMapping {
    nearDistance: f32,
    inverseDistanceSpan: f32,
}

struct VertexOutput {
    @builtin(position) cubemapFaceClipSpacePosition: vec4f,
    @location(0) cubemapFaceSpacePosition: vec3f,
}

struct FragmentOutput {
    @location(0) fragmentDepth: f32,
}

var<push_constant> pushConstants: PushConstants;

@group({{light_uniform_group}}) @binding({{light_uniform_binding}})
var<uniform> omnidirectionalLights: OmnidirectionalLights;

fn transformPosition(
    rotationQuaternion: vec4f,
    translation: vec3f,
    scaling: f32,
    position: vec3f
) -> vec3f {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
}

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn applyCubemapFaceProjectionToPosition(
    position: vec3f,
) -> vec4f {
    // It is important not to perform perspective division manually
    // here, because the homogeneous vector should be interpolated
    // first.
    return vec4f(
        position.xy,
        // This component does not matter, as we compute the proper
        // depth in the fragment shader
        position.z,
        position.z,
    );
}

fn computeShadowMapFragmentDepth(
    nearDistance: f32,
    inverseDistanceSpan: f32,
    cubemapSpaceFragmentPosition: vec3f,
) -> f32 {
    // Compute distance between fragment and light and scale to [0, 1] range
    return (length(cubemapSpaceFragmentPosition) - nearDistance) * inverseDistanceSpan;
}

@vertex
fn mainVS(
    @location({{position_location}}) modelSpacePosition: vec3f,
    modelToCubemapFaceSpaceTransform: ModelToCubemapFaceSpaceTransform,
) -> VertexOutput {
    var output: VertexOutput;

    let cubemapFaceSpacePosition = transformPosition(
        modelToCubemapFaceSpaceTransform.rotationQuaternion,
        modelToCubemapFaceSpaceTransform.translationAndScaling.xyz,
        modelToCubemapFaceSpaceTransform.translationAndScaling.w,
        modelSpacePosition,
    );

    output.cubemapFaceClipSpacePosition = applyCubemapFaceProjectionToPosition(cubemapFaceSpacePosition);
    output.cubemapFaceSpacePosition = cubemapFaceSpacePosition;

    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let omnidirectionalLight = omnidirectionalLights.lights[pushConstants.activeLightIdx];

    let lightNearDistance = omnidirectionalLight.distanceMapping.nearDistance;
    let lightInverseDistanceSpan = omnidirectionalLight.distanceMapping.inverseDistanceSpan;

    output.fragmentDepth = computeShadowMapFragmentDepth(
        lightNearDistance,
        lightInverseDistanceSpan,
        input.cubemapFaceSpacePosition,
    );

    return output;
}
