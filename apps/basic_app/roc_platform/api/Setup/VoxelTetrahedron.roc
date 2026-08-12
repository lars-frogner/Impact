# Hash: 2f5915b169657153
# Generated: 2026-08-12T18:18:21.064079493
# Rust type: impact_voxel::setup::VoxelTetrahedron
# Type category: Component
module [
    VoxelTetrahedron,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## An object made of voxels in a tetrahedral configuration. The tetrahedron
## has a base edge aligned with the x-axis and a top edge aligned with the
## z-axis, both twice as long as the height.
VoxelTetrahedron : {
    ## The extent of a single voxel.
    voxel_extent : F32,
    ## The number of voxels from the base edge to the top edge.
    height : F32,
}

## Defines a tetrahedron with the given voxel extent and number of voxels
## from its base edge to its top edge. The base edge is aligned with the
## x-axis and the top edge with the z-axis, and both are twice as long as
## the height.
##
## # Panics
## - If the voxel extent is negative.
## - If the height is negative.
new : F32, F32 -> VoxelTetrahedron
new = |voxel_extent, height|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect height >= 0.0
    {
        voxel_extent,
        height,
    }

## Defines a tetrahedron with the given voxel extent and number of voxels
## from its base edge to its top edge. The base edge is aligned with the
## x-axis and the top edge with the z-axis, and both are twice as long as
## the height.
##
## # Panics
## - If the voxel extent is negative.
## - If the height is negative.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, F32 -> Entity.ComponentData
add_new = |entity_data, voxel_extent, height|
    add(entity_data, new(voxel_extent, height))

## Defines a tetrahedron with the given voxel extent and number of voxels
## from its base edge to its top edge. The base edge is aligned with the
## x-axis and the top edge with the z-axis, and both are twice as long as
## the height.
##
## # Panics
## - If the voxel extent is negative.
## - If the height is negative.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, voxel_extent, height|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            voxel_extent, height,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [VoxelTetrahedron] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, VoxelTetrahedron -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelTetrahedron] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (VoxelTetrahedron) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelTetrahedron.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelTetrahedron -> List U8
write_packet = |bytes, val|
    type_id = 14972868737804231021
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelTetrahedron -> List U8
write_multi_packet = |bytes, vals|
    type_id = 14972868737804231021
    size = 8
    alignment = 4
    count = List.len(vals)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    vals
    |> List.walk(
        bytes_with_header,
        |bts, value| bts |> write_bytes(value),
    )

## Serializes a value of [VoxelTetrahedron] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelTetrahedron -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(value.voxel_extent)
    |> Builtin.write_bytes_f32(value.height)

## Deserializes a value of [VoxelTetrahedron] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelTetrahedron _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            height: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
