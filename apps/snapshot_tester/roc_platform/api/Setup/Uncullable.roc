# Hash: f8966d71865a5b373d561f6f363e2d35125f516d2353aebbaf1ac5c416de04e8
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_scene::setup::Uncullable
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    Uncullable,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## The entity should never be frustum culled in the
## [`SceneGraph`](crate::graph::SceneGraph).
##
## This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
## purpose is to aid in constructing a `SceneGraphModelInstanceNodeHandle`
## component for an entity. It is therefore not kept after entity creation.
Uncullable : {}

## Adds the [Uncullable] component to an entity's data.
add : Entity.Data -> Entity.Data
add = |entity_data|
    entity_data |> Entity.append_component(write_packet, {})

## Adds the [Uncullable] component to each entity's data.
add_multiple : Entity.MultiData -> Entity.MultiData
add_multiple = |entity_data|
    res = entity_data
        |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(Same({}), Entity.multi_count(entity_data)))
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in Uncullable.add_multiple: ${Inspect.to_str(err)}"

write_packet : List U8, Uncullable -> List U8
write_packet = |bytes, val|
    type_id = 1207053739455656714
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List Uncullable -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1207053739455656714
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

## Serializes a value of [Uncullable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Uncullable -> List U8
write_bytes = |bytes, _value|
    bytes

## Deserializes a value of [Uncullable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Uncullable _
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
