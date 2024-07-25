// TODO: Linear interpolation of quaternions from vertex to fragment positions
// may lead to vanishing quaternions where they actually should change very
// little if two of the vertex quaternions are similar but of opposite sign
// (negating a quaternion does not change the rotation).

fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn rotateVectorWithInverseOfQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector - quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn transformPosition(
    rotationQuaternion: vec4<f32>,
    translation: vec3<f32>,
    scaling: f32,
    position: vec3<f32>
) -> vec3<f32> {
    return transformPositionWithoutTranslation(rotationQuaternion, scaling, position) + translation;
}

fn transformPositionWithoutTranslation(
    rotationQuaternion: vec4<f32>,
    scaling: f32,
    position: vec3<f32>
) -> vec3<f32> {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position);
}

fn normalizeVector(vector: vec3<f32>) -> vec3<f32> {
    return normalize(vector);
}

fn normalizeQuaternion(quaternion: vec4<f32>) -> vec4<f32> {
    return normalize(quaternion);
}

fn applyRotationToTangentSpaceQuaternion(
    rotationQuaternion: vec4<f32>,
    tangentToParentSpaceRotationQuaternion: vec4<f32>,
) -> vec4<f32> {
    let q1 = rotationQuaternion;
    let q2 = tangentToParentSpaceRotationQuaternion;
    var rotated = normalize(vec4<f32>(q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz), q1.w * q2.w - dot(q1.xyz, q2.xyz)));
    
    // Preserve encoding of tangent space handedness in real component of
    // tangent space quaternion
    if (rotated.w < 0.0) != (tangentToParentSpaceRotationQuaternion.w < 0.0) {
        rotated = -rotated;
    }
    
    return rotated;
}

fn transformVectorFromTangentSpace(
    tangentToParentSpaceRotationQuaternion: vec4<f32>,
    tangentSpaceVector: vec3<f32>,
) -> vec3<f32> {
    var correctedTangentSpaceVector = tangentSpaceVector;

    // If the real component is negative, tangent space is really left-handed
    // and we have to flip the y (bitangent) component of the tangent space
    // vector before applying the rotation
    if tangentToParentSpaceRotationQuaternion.w < 0.0 {
        correctedTangentSpaceVector.y = -correctedTangentSpaceVector.y;
    }

    return rotateVectorWithQuaternion(tangentToParentSpaceRotationQuaternion, correctedTangentSpaceVector);
}

fn transformVectorToTangentSpace(
    tangentToParentSpaceRotationQuaternion: vec4<f32>,
    parentSpaceVector: vec3<f32>,
) -> vec3<f32> {
    var tangentSpaceVector = rotateVectorWithInverseOfQuaternion(tangentToParentSpaceRotationQuaternion, parentSpaceVector);
    
    // If the real component is negative, tangent space is really left-handed
    // and we have to flip the y (bitangent) component of the tangent space
    // vector after applying the rotation
    if tangentToParentSpaceRotationQuaternion.w < 0.0 {
        tangentSpaceVector.y = -tangentSpaceVector.y;
    }

    return tangentSpaceVector;
}

fn clampToZero(value: f32) -> f32 {
    return max(0.0, value);
}

fn computeCameraSpaceViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
    return normalize(-vertexPosition);
}

fn convertFramebufferPositionToScreenTextureCoords(
    inverseWindowDimensions: vec2<f32>,
    framebufferPosition: vec4<f32>,
) -> vec2<f32> {
    return framebufferPosition.xy * inverseWindowDimensions;
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalVector(color: vec3<f32>) -> vec3<f32> {
    // May require normalization depending on filtering
    return 2.0 * (color - 0.5);
}

// From [0, 1] to [-1, 1]
fn convertNormalColorToNormalizedNormalVector(color: vec3<f32>) -> vec3<f32> {
    return normalize(convertNormalColorToNormalVector(color));
}

// From [-1, 1] to [0, 1]
fn convertNormalVectorToNormalColor(normalVector: vec3<f32>) -> vec3<f32> {
    return 0.5 * (normalVector + 1.0);
}

// Returns a random number between 0 and 1 based on the pixel coordinates
fn generateInterleavedGradientNoiseFactor(cameraFramebufferPosition: vec4<f32>) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(magic.xy, cameraFramebufferPosition.xy)));
}

fn generateRandomAngle(cameraFramebufferPosition: vec4<f32>) -> f32 {
    // Multiply noise factor with 2 * pi to get random angle
    return 6.283185307 * generateInterleavedGradientNoiseFactor(cameraFramebufferPosition);
}

fn createJitteredPerspectiveProjectionMatrix(
    perspectiveProjectionMatrix: mat4x4<f32>,
    jitterOffsets: vec4<f32>,
    inverseWindowDimensions: vec2<f32>
) -> mat4x4<f32> {
    var jitteredMatrix = perspectiveProjectionMatrix;
    jitteredMatrix[2][0] += jitterOffsets.x * inverseWindowDimensions.x;
    jitteredMatrix[2][1] += jitterOffsets.y * inverseWindowDimensions.y;
    return jitteredMatrix;
}

fn computeMotionVector(
    screenTextureCoords: vec2<f32>,
    currentDepth: f32,
    previousClipSpacePosition: vec4<f32>,
) -> vec3<f32> {
    if (previousClipSpacePosition.w < 1e-6) {
        // The previous position is behind the camera
        return vec3<f32>(1.0, 1.0, 0.0);
    }
    let currentScreenCoords = vec3<f32>(screenTextureCoords.xy, currentDepth);
    let previousNDCPosition = previousClipSpacePosition.xyz / previousClipSpacePosition.w;
    let previousScreenCoords = vec3<f32>(0.5 * (1.0 + previousNDCPosition.x), 0.5 * (1.0 - previousNDCPosition.y), previousNDCPosition.z);
    return currentScreenCoords - previousScreenCoords;
}

fn zeroMotionVector() -> vec3<f32> {
    return vec3<f32>(0.0, 0.0, 0.0);
}