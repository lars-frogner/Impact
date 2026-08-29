# Batched voxel rendering via GPU buffer pooling

Design note for eliminating per-object command encoding overhead in voxel
rendering. Written 2026-07-05 after profiling a scene with 1000 small voxel
objects (`apps/basic_app/perf.data`).

## Problem

Command encoding inside `command_encoder.finish()` in `render_before_surface`
took 7 ms (21% of frame time). wgpu 27 defers render/compute pass encoding to
`finish()`, so this is the replay cost of the whole before-surface frame:
validation, resource tracking, barriers, and Vulkan HAL calls.

The cost scales as `objects x views`. The Fracturing scene has ~11 views
(camera + 6 omnidirectional shadow cubemap faces + 4 unidirectional cascades),
and for every visible object in every view the engine records:

- **Chunk culling compute pass** (`impact_voxel/src/render_commands.rs`,
  `VoxelChunkCullingPass::record`): ~140 B of push constants (frustum planes,
  most-inside corners, apex, chunk count, instance index) split over 3
  `set_push_constants` calls + 1 `set_bind_group` + 1 `dispatch_workgroups`.
- **Geometry pass** (`VoxelGeometryPipeline::record`): 1 push-constant set +
  1 `set_bind_group` + 2 `set_vertex_buffer` + 1 `multi_draw_indirect`.
- **Shadow map passes** (`VoxelRenderCommands::record_shadow_map_update`):
  1 `set_vertex_buffer` + 1 `set_index_buffer` + 1
  `multi_draw_indexed_indirect` per object per view.

That is tens of thousands of wgpu commands per frame at ~150-250 ns each
(validation + tracking + dyn-HAL downcast + Vulkan encoding). Hot symbols:
`DynCommandEncoder::set_bind_group` (~3%, mostly `Any::downcast_ref` overhead
in the culling passes), `encode_render_pass`/`set_vertex_buffer` (~2%),
indirect-draw validation (~1%), buffer trackers/barriers (~2.5%).

The root cause is that every object owns its own GPU buffers
(`VoxelObjectGPUBuffers` in `impact_voxel/src/gpu_resource.rs`: position,
normal, index, index-material, chunk-submesh, and two indirect-argument
buffers, plus 3 bind groups), so nothing can be batched.

## Target

Per view: **1 culling dispatch + 1 multi-draw**, with O(1) bind groups and
buffer binds. Commands per frame become proportional to views, not
`objects x views`.

## Design

### 1. Buffer arenas

Replace per-object buffers with one growable arena per data type:

| Arena | Replaces (per object) | Bound as |
|---|---|---|
| positions | `position_buffer` | storage (geometry) |
| normals | `normal_vector_buffer` | storage (geometry) |
| mesh indices | `index_buffer` | vertex buffer / index buffer |
| index materials | `index_material_buffer` | vertex buffer |
| chunk submeshes | `chunk_submesh_buffer` | storage (culling) |
| draw args | `indirect_argument_buffer` | storage (culling), indirect |
| indexed draw args | `indexed_indirect_argument_buffer` | storage (culling), indirect |

- Objects hold `{offset, len}` ranges into each arena instead of buffers.
- Suballocation via free lists with size classes (power-of-two slack) so
  fracturing-driven re-meshing usually reuses the slot in place.
- Arena growth = allocate bigger buffer + `copy_buffer_to_buffer` + rebuild
  the (single, global) bind group. Rare.
- Occasional compaction pass if fragmentation grows (GPU-side copies).
- Watch `max_storage_buffer_binding_size` / `max_buffer_size` limits; if
  exceeded, page arenas (one bind group per page). Irrelevant for many small
  objects.

### 2. Global chunk table + per-object metadata

- **Chunk table** (storage buffer): flattened list of all chunks of all
  resident objects. Entry = existing `ChunkSubmesh` fields + the owning
  object's slot index. Maintained incrementally on object add/remove/remesh.
- **Object metadata** (storage buffer, indexed by object slot): base offsets
  into each arena, chunk count, `origin_offset_in_root`, `chunk_extent`.
  Replaces the per-object push constants in the geometry pass.

### 3. Per-view culling-instance buffer

The per-object culling push constants become a per-view compact storage
buffer: for each visible object in the view, CPU writes `{frustum planes in
object space, most-inside corners, apex, instance idx, object slot}` - the
exact data computed today in `VoxelChunkCullingPass::record`, uploaded via
staging belt instead of pushed per dispatch.

(Later option: upload only object transforms and derive the planes on the
GPU; trades CPU + upload for GPU ALU. Not needed initially.)

### 4. One culling dispatch per view

Dispatch `total_visible_chunk_count` threads. Each thread:

1. reads its chunk-table entry -> object slot,
2. reads the object's culling instance for this view,
3. runs the existing frustum + obscuredness test
   (`voxel_chunk_culling.template.wgsl` logic unchanged),
4. writes draw args at its global chunk slot: `firstInstance = instanceIdx`
   (as today), `firstIndex`/`baseVertex` offset by the object's arena base.

Needs a small indirection: per view, the set of visible chunks differs, so
either dispatch over all resident chunks and let per-view visibility zero the
args (simplest; matches today's "zero the args to skip" approach), or build a
per-view chunk list on CPU alongside the culling instances.

### 5. One multi-draw per view

Bind the global bind groups + shared arena vertex/index buffers once, then a
single `multi_draw_indirect` / `multi_draw_indexed_indirect` over the view's
args range.

- **Draw count**: start with full-length draws where culled chunks have
  zeroed args (exactly today's semantics; zero-size draws are near-free on
  the GPU front end). Later: GPU compaction with an atomic counter +
  `multi_draw_indirect_count` (requires `Features::MULTI_DRAW_INDIRECT_COUNT`;
  fine on desktop Vulkan, not on WebGPU).
- **Args regions**: today one args buffer per object is reused sequentially
  across views. With pooling, give each view its own region in the args arena
  so all culling can run in one pass and draws never alias.

### 6. Vertex fetch offsets

The geometry shader fetches positions/normals from storage buffers using the
`VoxelMeshIndex` vertex attribute. Two options with pooled storage:

- (a) rewrite indices to be absolute into the pooled arrays at upload time
  (zero shader cost, indices are per-object data anyway), or
- (b) fetch the object's base offset from the metadata buffer via
  `instance_index`.

Option (a) is simplest; (b) is needed anyway if other per-object data
(`origin_offset_in_root`) moves to the metadata buffer, so both will likely
coexist.

## Migration phases

1. **Pool chunk-submesh + args buffers only.** Culling becomes one dispatch
   per view driven by the chunk table; draws still per object. Removes all
   per-object compute bind groups, dispatches, and push constants - the
   single biggest cost in the profile - without touching the draw path.
2. **Pool vertex/index/material buffers** with absolute indices and
   `baseVertex` offsets. Geometry and shadow passes become one multi-draw per
   view; per-object bind groups and vertex-buffer binds disappear.
3. **(Optional) GPU compaction + `multi_draw_indirect_count`**, and a single
   whole-frame culling pass writing every view's args region.

## Prerequisites and risks

- Features: `MULTI_DRAW_INDIRECT` and `INDIRECT_FIRST_INSTANCE` are already
  relied upon; `MULTI_DRAW_INDIRECT_COUNT` only for phase 3.
- wgpu's indirect-call validation (`InstanceFlags::VALIDATION_INDIRECT_CALL`,
  on by default even in release) batches and validates every draw arg on CPU
  + GPU; it should be disabled for production regardless of pooling
  (`engine/src/gpu.rs` instance creation), since all args are written by our
  own culling shader.
- Args memory scales with `resident chunks x views` if each view gets its own
  region; fine for small objects, revisit for huge worlds.
- Fragmentation from fracturing churn: mitigated by size-class free lists;
  compaction as fallback.

## Expected impact

- Per-view commands drop from ~4-6 per object to ~6 total; the 7 ms
  `finish()` cost should collapse to well under 1 ms, and tracker/barrier
  costs shrink with the unique-buffer count.
- Subsumes the existing todo item "Handle rendering of single-chunk voxel
  objects separately in a more lightweight manner".
