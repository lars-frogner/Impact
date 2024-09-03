struct PushConstants {
    chunkCount: u32,
    instanceIdx: u32,
}

struct ChunkSubmesh {
    chunkI: u32,
    chunkJ: u32,
    chunkK: u32,
    baseVertexIndex: u32,
    indexOffset: u32,
    indexCount: u32
}

struct IndirectDrawArgs {
    indexCount: u32,
    instanceCount: u32,
    firstIndex: u32,
    baseVertex: i32,
    firstInstance: u32,
}

var<push_constant> pushConstants: PushConstants;

@group({{chunk_submesh_group}}) @binding({{chunk_submesh_binding}})
var<storage, read> chunkSubmeshes: array<ChunkSubmesh>;

@group({{indirect_draw_group}}) @binding({{indirect_draw_binding}})
var<storage, read_write> indirectDrawArgs: array<IndirectDrawArgs>;

fn chunkShouldBeCulled(chunkIndices: vec3<u32>) -> bool {
    return false;
}

const WORKGROUP_SIZE: u32 = {{workgroup_size}};

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn main(
    @builtin(global_invocation_id) globalID: vec3u,
) {
    let globalIdx = globalID.x;

    if (globalIdx < pushConstants.chunkCount) {
        let chunkSubmesh = chunkSubmeshes[globalIdx];

        let chunkIndices = vec3u(chunkSubmesh.chunkI, chunkSubmesh.chunkJ, chunkSubmesh.chunkK);
        let culled = chunkShouldBeCulled(chunkIndices);

        if (culled) {
            // Make sure the draw call is skipped
            indirectDrawArgs[globalIdx].indexCount = 0u;
            indirectDrawArgs[globalIdx].instanceCount = 0u;
        } else {
            indirectDrawArgs[globalIdx].indexCount = chunkSubmesh.indexCount;
            indirectDrawArgs[globalIdx].instanceCount = 1u;
            indirectDrawArgs[globalIdx].firstIndex = chunkSubmesh.indexOffset;
            indirectDrawArgs[globalIdx].baseVertex = i32(chunkSubmesh.baseVertexIndex);
            indirectDrawArgs[globalIdx].firstInstance = pushConstants.instanceIdx;
        }
    }
}
