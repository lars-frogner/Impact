# Hash: df8b9ef9f7b85007
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_voxel::setup::SameVoxelType
# Type category: Component
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
import core.Builtin
import core.Hashing

## A voxel type that is the only type present in a voxel object.
SameVoxelType : {
    voxel_type_name_hash : Hashing.Hash32,
}

new : Str -> SameVoxelType
new = |voxel_type_name|
    { voxel_type_name_hash: Hashing.hash_str_32(voxel_type_name) }

add_new : Entity.ComponentData, Str -> Entity.ComponentData
add_new = |entity_data, voxel_type_name|
    add(entity_data, new(voxel_type_name))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Str) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, voxel_type_name|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            voxel_type_name,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SameVoxelType] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, SameVoxelType -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SameVoxelType] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (SameVoxelType) -> Result Entity.MultiComponentData Str
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
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SameVoxelType -> List U8
write_multi_packet = |bytes, vals|
    type_id = 3721752180572445305
    size = 4
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

## Serializes a value of [SameVoxelType] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SameVoxelType -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Hashing.write_bytes_hash_32(value.voxel_type_name_hash)

## Deserializes a value of [SameVoxelType] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SameVoxelType _
from_bytes = |bytes|
    Ok(
        {
            voxel_type_name_hash: bytes |> List.sublist({ start: 0, len: 4 }) |> Hashing.from_bytes_hash_32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 4 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
