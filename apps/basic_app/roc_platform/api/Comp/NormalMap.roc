# Hash: a6294a80f197dc8439963d71e23a8f0e329b9ca0f5a5cfa73b02180bf21c2695
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::NormalMapComp
# Type category: Component
# Commit: d505d37
module [
    NormalMap,
    add,
    add_multiple,
]

import Entity
import Rendering.TextureID
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose surface details are described by a normal map.
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
NormalMap : Rendering.TextureID.TextureID

## Adds a value of the [NormalMap] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, NormalMap -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [NormalMap] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List NormalMap -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, NormalMap -> List U8
write_packet = |bytes, value|
    type_id = 13304748650252932154
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List NormalMap -> List U8
write_multi_packet = |bytes, values|
    type_id = 13304748650252932154
    size = 4
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

## Serializes a value of [NormalMap] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NormalMap -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Rendering.TextureID.write_bytes(value)

## Deserializes a value of [NormalMap] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NormalMap _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Rendering.TextureID.from_bytes?,
        ),
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
