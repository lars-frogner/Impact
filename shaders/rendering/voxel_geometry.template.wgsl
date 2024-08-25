struct PushConstants {
    modelViewTransformWithPrevious: ModelViewTransformWithPrevious,
    inverseWindowDimensions: vec2f,
    frameCounter: u32,
}

struct ModelViewTransformWithPrevious {
    current: ModelViewTransform,
    previous: ModelViewTransform,
}

struct ModelViewTransform {
    rotationQuaternion: vec4f,
    translationAndScaling: vec4f, // Voxel extent is baked into scaling
}

struct ProjectionUniform {
    projection: mat4x4f,
    frustumFarPlaneCorners: array<vec4f, 4>,
    inverseFarPlaneZ: vec4f,
    jitterOffsets: array<vec4f, {{jitter_count}}>,
}

// This can be reduced to 8 bytes total by including the grid dimensions in the
// push constant, passing the linear grid index as a u32, and including another
// u32 where 16 bits store the scale, 4 bits store the primitive index, 4 bits
// store the orientation index and 8 bits store the material index, all stored
// in a Uint32x2.
struct VoxelInstance {
    @location({{instance_grid_indices_location}}) gridIndicesAndScale: vec4u, // Uint16x4
    @location({{instance_primitive_offset_location}}) primitiveAndOrientationOffsets: vec2u, // Uint8x2
    @location({{instance_material_index_location}}) materialIndex: vec2u, // Uint8x2 (only first used)
}

struct VoxelPrimitiveVertexUniform {
    voxelSpacePosition: vec4f, // Position in voxel space ([0.0, 1.0]^3)
    voxelSpaceNormalVector: vec4f,
    textureCoords: vec4f,
    tangentToVoxelSpaceRotationQuaternion: vec4f,
}

struct VertexOutput {
    @builtin(position) clipSpacePosition: vec4f,
    @location(0) previousClipSpacePosition: vec4f,
    @location(1) cameraSpacePosition: vec3f,
    @location(2) cameraSpaceNormalVector: vec3f,
    @location(3) textureCoords: vec2f,
    @location(4) tangentToCameraSpaceRotationQuaternion: vec4f,
}

const JITTER_COUNT: u32 = {{jitter_count}};

const PRIMITIVE_VERTEX_COUNT: u32 = {{primitive_vertex_count}};
const PRIMITIVE_ORIENTATION_COUNT: u32 = {{primitive_orientation_count}};

var<push_constant> pushConstants: PushConstants;

@group({{projection_uniform_group}}) @binding({{projection_uniform_binding}})
var<uniform> projectionUniform: ProjectionUniform;

@group({{voxel_primitive_vertex_uniform_group}}) @binding({{voxel_primitive_vertex_uniform_binding}})
var<uniform> voxelPrimitiveVertexUniforms: array<VoxelPrimitiveVertexUniform, {{n_voxel_primitive_vertex_uniforms}}>;

fn obtainPrimitiveVertexIndex(voxelInstance: VoxelInstance) -> u32 {
    // Combine the primitive and orientation offsets to get the total offset to
    // the section in the uniform array containing the vertices for this voxel
    // instance
    let primitiveOffset = PRIMITIVE_ORIENTATION_COUNT * voxelInstance.primitiveAndOrientationOffsets.x;
    let orientationOffset = voxelInstance.primitiveAndOrientationOffsets.y;
    let vertexOffset = PRIMITIVE_VERTEX_COUNT * (primitiveOffset + orientationOffset);

    // Add the current vertex index to the offset to get the current vertex
    return vertexOffset + vertexIndex.x;
}

fn transformVoxelSpacePositionToGrid(
    voxelInstance: VoxelInstance,
    voxelSpacePosition: vec3f,
) -> vec3f {
    // Transform the vertex position from the space of an individual voxel to
    // the space of the voxel grid by translating by the indices of the voxel
    // instance in the grid after scaling by the number of voxels the instance
    // covers (spatial voxel extent is applied later)
    let translation = vec3f(voxelInstance.gridIndicesAndScale.xyz);
    let scaling = f32(voxelInstance.gridIndicesAndScale.w);
    return translation + scaling * voxelSpacePosition;
}

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

fn applyRotationToTangentSpaceQuaternion(
    rotationQuaternion: vec4f,
    tangentToParentSpaceRotationQuaternion: vec4f,
) -> vec4f {
    let q1 = rotationQuaternion;
    let q2 = tangentToParentSpaceRotationQuaternion;
    var rotated = normalize(vec4f(q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz), q1.w * q2.w - dot(q1.xyz, q2.xyz)));

    // Preserve encoding of tangent space handedness in real component of
    // tangent space quaternion
    if (rotated.w < 0.0) != (tangentToParentSpaceRotationQuaternion.w < 0.0) {
        rotated = -rotated;
    }

    return rotated;
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
    @location({{vertex_index_location}}) vertexIndex: vec2u, // Uint8x2 (only first used)
    voxelInstance: VoxelInstance,
) -> VertexOutput {
    var output: VertexOutput;

    let primitiveVertex = voxelPrimitiveVertexUniforms[obtainPrimitiveVertexIndex(voxelInstance)];
    let positionInGrid = transformVoxelSpacePositionToGrid(voxelInstance, primitiveVertex.voxelSpacePosition);

    let projectionMatrix = obtainProjectionMatrix();

    let modelViewTransform = pushConstants.modelViewTransformWithPrevious.current;
    let cameraSpacePosition = transformPosition(
        modelViewTransform.rotationQuaternion,
        modelViewTransform.translationAndScaling.xyz,
        modelViewTransform.translationAndScaling.w,
        positionInGrid,
    );
    output.clipSpacePosition = projectionMatrix * vec4f(cameraSpacePosition, 1.0);
    output.cameraSpacePosition = cameraSpacePosition;

    let previousModelViewTransform = pushConstants.modelViewTransformWithPrevious.previous;
    let previousCameraSpacePosition = transformPosition(
        previousModelViewTransform.rotationQuaternion,
        previousModelViewTransform.translationAndScaling.xyz,
        previousModelViewTransform.translationAndScaling.w,
        positionInGrid,
    );
    output.previousClipSpacePosition = projectionMatrix * vec4f(previousCameraSpacePosition, 1.0);

    output.cameraSpaceNormalVector = rotateVectorWithQuaternion(
        modelViewTransform.rotationQuaternion,
        primitiveVertex.voxelSpaceNormalVector,
    );

    output.textureCoords = primitiveVertex.textureCoords;

    output.tangentToCameraSpaceRotationQuaternion = applyRotationToTangentSpaceQuaternion(
        modelViewTransform.rotationQuaternion,
        primitiveVertex.tangentToVoxelSpaceRotationQuaternion,
    );

    return output;
}
