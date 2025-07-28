# Hash: bd42669a0cdd0c52411235cecf30e02e2cad45083b571b90cf34307ed9c0fcff
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_voxel::setup::SameVoxelType
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    SameVoxelType,
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
import Voxel.VoxelType
import core.Builtin
import core.NativeNum

## A voxel type that is the only type present in a voxel object.
SameVoxelType : {
    ## The index of the voxel type.
    voxel_type_idx : NativeNum.Usize,
}

new : Voxel.VoxelType.VoxelType -> SameVoxelType
new = |voxel_type|
    { voxel_type_idx: NativeNum.to_usize(voxel_type) }

add_new : Entity.Data, Voxel.VoxelType.VoxelType -> Entity.Data
add_new = |entity_data, voxel_type|
    add(entity_data, new(voxel_type))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Voxel.VoxelType.VoxelType) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, voxel_type|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            voxel_type,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SameVoxelType] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SameVoxelType -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SameVoxelType] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SameVoxelType) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SameVoxelType.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SameVoxelType -> List U8
write_packet = |bytes, val|
    type_id = 3721752180572445305
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SameVoxelType -> List U8
write_multi_packet = |bytes, vals|
    type_id = 3721752180572445305
    size = 8
    alignment = 8
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

## Serializes a value of [SameVoxelType] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SameVoxelType -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> NativeNum.write_bytes_usize(value.voxel_type_idx)

## Deserializes a value of [SameVoxelType] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SameVoxelType _
from_bytes = |bytes|
    Ok(
        {
            voxel_type_idx: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
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
