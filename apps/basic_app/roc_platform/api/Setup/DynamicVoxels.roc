# Hash: ad23b7b7fcb26c0b99fa69b9b72cdd814b30f9a52d7c87a7e916e4f201bc22a7
# Generated: 2025-09-20T12:39:41+00:00
# Rust type: impact_voxel::setup::DynamicVoxels
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    DynamicVoxels,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## A voxel object with dynamic voxels will behave like a dynamic rigid body
## and respond to voxel absorption.
DynamicVoxels : {}

## Adds the [DynamicVoxels] component to an entity's data.
add : Entity.ComponentData -> Entity.ComponentData
add = |entity_data|
    entity_data |> Entity.append_component(write_packet, {})

## Adds the [DynamicVoxels] component to each entity's data.
add_multiple : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple = |entity_data|
    res = entity_data
        |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(Same({}), Entity.multi_count(entity_data)))
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in DynamicVoxels.add_multiple: ${Inspect.to_str(err)}"

write_packet : List U8, DynamicVoxels -> List U8
write_packet = |bytes, val|
    type_id = 12582075058657672671
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicVoxels -> List U8
write_multi_packet = |bytes, vals|
    type_id = 12582075058657672671
    size = 0
    alignment = 1
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

## Serializes a value of [DynamicVoxels] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicVoxels -> List U8
write_bytes = |bytes, _value|
    bytes

## Deserializes a value of [DynamicVoxels] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicVoxels _
from_bytes = |_bytes|
    Ok({})

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 0 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
