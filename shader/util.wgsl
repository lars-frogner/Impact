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
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
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

fn tranformVectorFromTangentSpace(
    tangentToParentSpaceRotationQuaternion: vec4<f32>,
    tangentSpaceVector: vec3<f32>,
) -> vec3<f32> {
    var tangentSpaceVector = tangentSpaceVector;

    // If the real component is negative, tangent space is really left-handed
    // and we have to flip the y (bitangent) component of the tangent space
    // vector before applying the rotation
    if tangentToParentSpaceRotationQuaternion.w < 0.0 {
        tangentSpaceVector.y = -tangentSpaceVector.y;
    }

    return rotateVectorWithQuaternion(tangentToParentSpaceRotationQuaternion, tangentSpaceVector);
}

fn tranformVectorToTangentSpace(
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

fn computeCameraSpaceViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
    return normalize(-vertexPosition);
}
