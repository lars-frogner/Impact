struct PushConstants {
    // Split up inverseWindowDimensions to avoid padding
    inverseWindowWidth: f32,
    inverseWindowHeight: f32,
    frameCounter: u32,
    exposure: f32,
}

struct ProjectionUniform {
    projection: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
    inverseFarPlaneZ: vec4f,
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

struct VertexInput {
    @location({{position_location}}) modelSpacePosition: vec3f,
    @location({{normal_vector_location}}) modelSpaceNormalVector: vec3f,
}

struct FragmentInput {
    @builtin(position) projectedPosition: vec4f,
    @location(0) previousClipSpacePosition: vec4f,
    @location(1) cameraSpacePosition: vec3f,
    @location(2) cameraSpaceNormalVector: vec3f,
}

struct FragmentOutput {
    @location(0) linearDepth: f32,
    @location(1) normalVector: vec4f,
    @location(2) motionVector: vec2f,
    @location(3) materialColor: vec4f,
    @location(4) materialProperties: vec4f,
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

fn rotateVectorWithQuaternion(quaternion: vec4f, vector: vec3f) -> vec3f {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn obtainProjectionMatrix() -> mat4x4f {
    var matrix = projectionUniform.projection;
    let jitterIndex = pushConstants.frameCounter % JITTER_COUNT;
    let jitterOffsets = projectionUniform.jitterOffsets[jitterIndex];
    matrix[2][0] += jitterOffsets.x * pushConstants.inverseWindowWidth;
    matrix[2][1] += jitterOffsets.y * pushConstants.inverseWindowHeight;
    return matrix;
}

fn convertFramebufferPositionToScreenTextureCoords(framebufferPosition: vec4f) -> vec2<f32> {
    return framebufferPosition.xy * vec2f(pushConstants.inverseWindowWidth, pushConstants.inverseWindowHeight);
}

fn computeMotionVector(
    screenTextureCoords: vec2f,
    previousClipSpacePosition: vec4f,
) -> vec2f {
    if (previousClipSpacePosition.w < 1e-6) {
        // The previous position is behind the camera
        return vec2f(1.0, 1.0);
    }
    let previousNDCXYPosition = previousClipSpacePosition.xy / previousClipSpacePosition.w;
    let previousScreenCoords = vec2f(0.5 * (1.0 + previousNDCXYPosition.x), 0.5 * (1.0 - previousNDCXYPosition.y));
    return screenTextureCoords - previousScreenCoords;
}

// From [-1, 1] to [0, 1]
fn convertNormalVectorToNormalColor(normalVector: vec3f) -> vec3f {
    return 0.5 * (normalVector + 1.0);
}

@vertex
fn mainVS(
    vertex: VertexInput,
    modelViewTransform: ModelViewTransform,
    previousModelViewTransform: PreviousModelViewTransform,
) -> FragmentInput {
    var output: FragmentInput;

    let projectionMatrix = obtainProjectionMatrix();

    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translationAndScaling.xyz,
        modelViewTransform.translationAndScaling.w,
        vertex.modelSpacePosition,
    );
    output.projectedPosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);
    output.cameraSpacePosition = cameraSpacePosition;

    let previousCameraSpacePosition = transformPosition(
        previousModelViewTransform.rotationQuaternion,
        previousModelViewTransform.translationAndScaling.xyz,
        previousModelViewTransform.translationAndScaling.w,
        vertex.modelSpacePosition,
    );
    output.previousClipSpacePosition = projectionMatrix * vec4f(previousCameraSpacePosition, 1.0);

    output.cameraSpaceNormalVector = rotateVectorWithQuaternion(
        modelViewTransform.rotationQuaternion,
        vertex.modelSpaceNormalVector,
    );

    return output;
}

@fragment
fn mainFS(fragment: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    output.linearDepth = projectionUniform.inverseFarPlaneZ.x * fragment.cameraSpacePosition.z;

    let cameraSpaceNormalVector = normalize(fragment.cameraSpaceNormalVector);

    output.normalVector = vec4f(convertNormalVectorToNormalColor(cameraSpaceNormalVector), 1.0);

    let screenTextureCoords = convertFramebufferPositionToScreenTextureCoords(
        fragment.projectedPosition,
    );
    output.motionVector = computeMotionVector(screenTextureCoords, fragment.previousClipSpacePosition);

    let materialColor = vec3f(0.5);
    output.materialColor = vec4f(materialColor, 1.0);

    let specularReflectance = 0.5;
    let roughness = 0.5;
    let metalness = 0.0;
    let preExposedEmissiveLuminance = 0.0 * pushConstants.exposure;

    output.materialProperties = vec4f(specularReflectance, roughness, metalness, preExposedEmissiveLuminance);

    return output;
}
