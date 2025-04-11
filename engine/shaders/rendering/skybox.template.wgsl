struct PushConstants {
    cameraRotationQuaternion: vec4f,
    exposure: f32,
}

struct ProjectionUniform {
    projection: mat4x4f,
}

struct SkyboxProperties {
    maxLuminance: f32,
}

struct VertexOutput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) modelSpacePosition: vec3f,
}

struct FragmentOutput {
    @location(0) preExposedLuminance: vec4f
}

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

@group({{skybox_properties_group}}) @binding({{skybox_properties_binding}})
var<uniform> skyboxProperties: SkyboxProperties;

@group({{skybox_texture_group}}) @binding({{skybox_texture_binding}})
var skyboxTexture: texture_cube<f32>;
@group({{skybox_texture_group}}) @binding({{skybox_sampler_binding}})
var skyboxSampler: sampler;

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

@vertex
fn mainVS(
    @location({{position_location}}) modelSpacePosition: vec3f,
) -> VertexOutput {
    var output: VertexOutput;

    let untranslatedCameraSpacePosition = rotateVectorWithQuaternion(
        pushConstants.cameraRotationQuaternion,
        modelSpacePosition,
    );
    output.projectedPosition = (projectionUniform.projection * vec4f(untranslatedCameraSpacePosition, 1.0)).xyww;
    output.modelSpacePosition = modelSpacePosition;

    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let sample = textureSample(skyboxTexture, skyboxSampler, input.modelSpacePosition);
    output.preExposedLuminance = (pushConstants.exposure * skyboxProperties.maxLuminance) * sample;

    return output;
}
