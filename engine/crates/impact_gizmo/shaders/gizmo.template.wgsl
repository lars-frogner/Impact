struct ProjectionUniform {
    projection: mat4x4f,
}

struct ModelViewTransform {
    @location({{model_view_transform_rotation_location}}) rotationQuaternion: vec4f,
    @location({{model_view_transform_translation_location}}) translation: vec3f,
    @location({{model_view_transform_scaling_location}}) scaling: vec3f,
}

struct VertexInput {
    @location({{position_location}}) modelSpacePosition: vec3f,
    @location({{color_location}}) color: vec4f,
}

struct VertexOutput {
    @builtin(position) clipSpacePosition: vec4f,
    @location(0) color: vec4f,
}

struct FragmentOutput {
    @location(0) color: vec4f
}

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

fn transformPosition(
    rotationQuaternion: vec4f,
    translation: vec3f,
    scaling: vec3f,
    position: vec3f
) -> vec3f {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
}

fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

@vertex
fn mainVS(
    vertex: VertexInput,
    modelViewTransform: ModelViewTransform,
) -> VertexOutput {
    var output: VertexOutput;

    let projectionMatrix = projectionUniform.projection;

    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translation,
        modelViewTransform.scaling,
        vertex.modelSpacePosition,
    );
    output.clipSpacePosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);

    output.color = vertex.color;

    return output;
}

@fragment
fn mainFS(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;
    output.color = input.color;
    return output;
}
