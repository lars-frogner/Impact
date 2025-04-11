struct PushConstants {
    inverseWindowDimensions: vec2f,
    frameCounter: u32,
}

struct ProjectionUniform {
    projection: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
    inverseFarPlaneDistance: vec4f,
    jitterOffsets: array<vec4f, {{jitter_count}}>,
}

struct ModelViewTransform {
    @location({{model_view_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{model_view_transform_translation_location}}) translationAndScaling: vec4f,
}

struct PreviousModelViewTransform {
    @location({{previous_model_view_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{previous_model_view_transform_translation_location}}) translationAndScaling: vec4f,
}

struct VertexOutput {
    @builtin(position) clipSpacePosition: vec4f,
}

const JITTER_COUNT: u32 = {{jitter_count}};

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

fn transformPosition(
    rotationQuaternion: vec4f,
    translation: vec3f,
    scaling: f32,
    position: vec3f
) -> vec3f {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
}

fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn obtainProjectionMatrix() -> mat4x4f {
    var matrix = projectionUniform.projection;
    let jitterIndex = pushConstants.frameCounter % JITTER_COUNT;
    let jitterOffsets = projectionUniform.jitterOffsets[jitterIndex];
    matrix[2][0] += jitterOffsets.x * pushConstants.inverseWindowDimensions.x;
    matrix[2][1] += jitterOffsets.y * pushConstants.inverseWindowDimensions.y;
    return matrix;
}

@vertex
fn mainVS(
    @location({{position_location}}) modelSpacePosition: vec3f,
    modelViewTransform: ModelViewTransform,
    previousModelViewTransform: PreviousModelViewTransform,
) -> VertexOutput {
    var output: VertexOutput;

    let projectionMatrix = obtainProjectionMatrix();

    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translationAndScaling.xyz,
        modelViewTransform.translationAndScaling.w,
        modelSpacePosition,
    );
    output.clipSpacePosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);

    return output;
}
