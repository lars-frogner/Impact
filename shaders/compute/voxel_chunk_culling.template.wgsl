struct PushConstants {
    frustumPlane0: array<f32, 4>,
    frustumPlane1: array<f32, 4>,
    frustumPlane2: array<f32, 4>,
    frustumPlane3: array<f32, 4>,
    frustumPlane4: array<f32, 4>,
    frustumPlane5: array<f32, 4>,
    mostInsideCorners: array<u32, 6>,
    apexPosition: array<f32, 3>,
    chunkCount: u32,
    instanceIdx: u32,
}

struct ChunkSubmesh {
    chunkI: u32,
    chunkJ: u32,
    chunkK: u32,
    baseVertexIndex: u32,
    indexOffset: u32,
    indexCount: u32,
    isObscuredFromDirection: array<array<array<u32, 2>, 2>, 2>,
}

struct IndirectDrawArgs {
    indexCount: u32,
    instanceCount: u32,
    firstIndex: u32,
    baseVertex: i32,
    firstInstance: u32,
}

var<push_constant> pcs: PushConstants;

@group({{chunk_submesh_group}}) @binding({{chunk_submesh_binding}})
var<storage, read> chunkSubmeshes: array<ChunkSubmesh>;

@group({{indirect_draw_group}}) @binding({{indirect_draw_binding}})
var<storage, read_write> indirectDrawArgs: array<IndirectDrawArgs>;

// We use a small non-zero threshold to make sure barely visible chunks are not
// culled due to imprecision
const CULLING_THRESHOLD: f32 = -0.05;

fn chunkIsOutsideFrustum(lowerCorner: vec3f) -> bool {
    // We expect the frustum planes to have been transformed to the normalized
    // voxel object space where the chunks are axis-aligned boxes with extent
    // one, and the lower corner of the chunk at chunk index [i, j, k] is at
    // coordinate (i, j, k). This makes the culling check very simple: for each
    // frustum plane, we compute the chunk corner with the largest signed
    // distance by looking up the corner index in the `mostInsideCorners`
    // array. If this is negative, the entire chunk must be in the negative
    // half-space of the plane. If the chunk is in the negative half-space of
    // any of the planes, it is outside the frustum.
    var CORNERS_OFFSETS = array<vec3f, 8>(
        vec3f(0.0, 0.0, 0.0),
        vec3f(0.0, 0.0, 1.0),
        vec3f(0.0, 1.0, 0.0),
        vec3f(0.0, 1.0, 1.0),
        vec3f(1.0, 0.0, 0.0),
        vec3f(1.0, 0.0, 1.0),
        vec3f(1.0, 1.0, 0.0),
        vec3f(1.0, 1.0, 1.0),
    );
    return (
        signedDistance(pcs.frustumPlane0, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[0]]) < CULLING_THRESHOLD ||
        signedDistance(pcs.frustumPlane1, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[1]]) < CULLING_THRESHOLD ||
        signedDistance(pcs.frustumPlane2, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[2]]) < CULLING_THRESHOLD ||
        signedDistance(pcs.frustumPlane3, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[3]]) < CULLING_THRESHOLD ||
        signedDistance(pcs.frustumPlane4, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[4]]) < CULLING_THRESHOLD ||
        signedDistance(pcs.frustumPlane5, lowerCorner + CORNERS_OFFSETS[pcs.mostInsideCorners[5]]) < CULLING_THRESHOLD
    );
}

fn signedDistance(plane: array<f32, 4>, point: vec3f) -> f32 {
    let unitNormal = vec3f(plane[0], plane[1], plane[2]);
    let displacement = plane[3];
    return dot(unitNormal, point) - displacement;
}

fn chunkIsObscured(chunkSubmesh: ChunkSubmesh, lowerCorner: vec3f) -> bool {
    var isObscuredFromDirection = chunkSubmesh.isObscuredFromDirection;

    // Compute the unnormalized direction from the frustum apex to the center
    // of the chunk
    let viewDirection = lowerCorner + vec3f(0.5) - vec3f(pcs.apexPosition[0], pcs.apexPosition[1], pcs.apexPosition[2]);

    // For each axis, the sign of the view direction determines the
    // corresponding index into the table (positive -> 0, negative -> 1)
    let indices = select(vec3u(0u), vec3u(1u), viewDirection < vec3f(0.0));

    return isObscuredFromDirection[indices.x][indices.y][indices.z] > 0u;
}

const WORKGROUP_SIZE: u32 = {{workgroup_size}};

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn main(
    @builtin(global_invocation_id) globalID: vec3u,
) {
    let globalIdx = globalID.x;

    let chunkCount = pcs.chunkCount;
    let instanceIdx = pcs.instanceIdx;

    if (globalIdx < chunkCount) {
        let chunkSubmesh = chunkSubmeshes[globalIdx];

        let chunkIndices = vec3f(f32(chunkSubmesh.chunkI), f32(chunkSubmesh.chunkJ), f32(chunkSubmesh.chunkK));

        if (chunkIsOutsideFrustum(chunkIndices) || chunkIsObscured(chunkSubmesh, chunkIndices)) {
            // Make sure the draw call is skipped
            indirectDrawArgs[globalIdx].indexCount = 0u;
            indirectDrawArgs[globalIdx].instanceCount = 0u;
        } else {
            indirectDrawArgs[globalIdx].indexCount = chunkSubmesh.indexCount;
            indirectDrawArgs[globalIdx].instanceCount = 1u;
            indirectDrawArgs[globalIdx].firstIndex = chunkSubmesh.indexOffset;
            indirectDrawArgs[globalIdx].baseVertex = i32(chunkSubmesh.baseVertexIndex);
            indirectDrawArgs[globalIdx].firstInstance = instanceIdx;
        }
    }
}
