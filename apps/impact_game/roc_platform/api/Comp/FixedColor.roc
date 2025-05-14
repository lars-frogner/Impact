# Hash: 7529fbeee03b24726b4ebc123c023c8825f1126c8a767409439bed36e8ecc7b6
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::FixedColorComp
# Type category: Component
# Commit: d505d37
module [
    FixedColor,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## Setup [`SetupComponent`](impact_ecs::component::SetupComponent) for
## initializing entities that have a fixed, uniform color that is independent
## of lighting.
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
FixedColor : Vector3.Vector3 Binary32

## Adds a value of the [FixedColor] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, FixedColor -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [FixedColor] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List FixedColor -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, FixedColor -> List U8
write_packet = |bytes, value|
    type_id = 14806733351734441480
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List FixedColor -> List U8
write_multi_packet = |bytes, values|
    type_id = 14806733351734441480
    size = 12
    alignment = 4
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

## Serializes a value of [FixedColor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FixedColor -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Vector3.write_bytes_32(value)

## Deserializes a value of [FixedColor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FixedColor _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
