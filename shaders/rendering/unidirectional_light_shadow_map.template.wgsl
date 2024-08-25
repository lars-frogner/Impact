struct PushConstants {
    activeLightIdx: u32,
    activeCascadeIdx: u32,
}

struct ModelToLightTransform {
    @location({{model_light_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{model_light_transform_translation_location}}) translationAndScaling: vec4f,
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

struct VertexOutput {
    @builtin(position) lightClipSpacePosition: vec4f,
}

var<push_constant> pushConstants: PushConstants;

@group({{light_uniform_group}}) @binding({{light_uniform_binding}})
var<uniform> unidirectionalLights: UnidirectionalLights;

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

fn applyOrthographicProjectionToPosition(
    orthographicTranslation: vec3f,
    orthographicScaling: vec3f,
    position: vec3f
    ) -> vec3f {
    return (position + orthographicTranslation) * orthographicScaling;
}

@vertex
fn mainVS(
    @location({{position_location}}) modelSpacePosition: vec3f,
    modelToLightTransform: ModelToLightTransform,
) -> VertexOutput {
    var output: VertexOutput;

    let lightSpacePosition = transformPosition(
        modelToLightTransform.rotationQuaternion,
        modelToLightTransform.translationAndScaling.xyz,
        modelToLightTransform.translationAndScaling.w,
        modelSpacePosition,
    );

    // Note: `var` is required here instead of `let` to make the
    // `orthographicTransforms` array indexable with a dynamic index
    var unidirectionalLight = unidirectionalLights.lights[pushConstants.activeLightIdx];
    let lightOrthographicTransform = unidirectionalLight.orthographicTransforms[pushConstants.activeCascadeIdx];

    let lightClipSpacePosition = applyOrthographicProjectionToPosition(
        lightOrthographicTransform.translation,
        lightOrthographicTransform.scaling,
        lightSpacePosition,
    );

    output.lightClipSpacePosition = vec4f(lightClipSpacePosition, 1.0);

    return output;
}
