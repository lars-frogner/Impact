# Hash: 832428b0695087ab30858094db0ea8b0cfa0c783282439526657df257c13276d
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::UncullableComp
# Type category: Component
# Commit: d505d37
module [
    Uncullable,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that should never be frustum culled in the
## [`SceneGraph`](crate::scene::SceneGraph).
##
## The purpose of this component is to aid in constructing a
## [`SceneGraphModelInstanceNodeComp`] for the entity. It is therefore not kept
## after entity creation.
Uncullable : {}

## Adds a value of the [Uncullable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Uncullable -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Uncullable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Uncullable -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Uncullable -> List U8
write_packet = |bytes, value|
    type_id = 10018873855112902633
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Uncullable -> List U8
write_multi_packet = |bytes, values|
    type_id = 10018873855112902633
    size = 0
    alignment = 1
    count = List.len(values)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    values
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
