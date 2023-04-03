fn rotateVectorWithQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn rotateVectorWithInverseOfQuaternion(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let tmp = 2.0 * cross(quaternion.xyz, vector);
    return vector - quaternion.w * tmp + cross(quaternion.xyz, tmp);
}

fn multiplyAndNormalizeQuaternions(q1: vec4<f32>, q2: vec4<f32>) -> vec4<f32> {
    let product = vec4<f32>(q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz), q1.w * q2.w - dot(q1.xyz, q2.xyz));
    return normalize(product);
}

fn transformPosition(
    rotationQuaternion: vec4<f32>,
    translation: vec3<f32>,
    scaling: f32,
    position: vec3<f32>
) -> vec3<f32> {
    return rotateVectorWithQuaternion(rotationQuaternion, scaling * position) + translation;
}

fn computeCameraSpaceViewDirection(vertexPosition: vec3<f32>) -> vec3<f32> {
    return normalize(-vertexPosition);
}
