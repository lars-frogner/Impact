# Hash: 3862378afb76f61ea9042afd06fc94dfd88fcff1ed8f730fb13812ea2d1ef7e5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::light::components::UnidirectionalLightComp
# Type category: Component
# Commit: d505d37
module [
    UnidirectionalLight,
    add,
    add_multiple,
]

import Entity
import Light.LightID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that
## have a [`UnidirectionalLight`](crate::light::UnidirectionalLight).
UnidirectionalLight : {
    ## The ID of the entity's
    ## [`UnidirectionalLight`](crate::light::UnidirectionalLight).
    id : Light.LightID.LightID,
}

## Adds a value of the [UnidirectionalLight] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UnidirectionalLight -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UnidirectionalLight] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UnidirectionalLight -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UnidirectionalLight -> List U8
write_packet = |bytes, value|
    type_id = 15652993950230557946
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UnidirectionalLight -> List U8
write_multi_packet = |bytes, values|
    type_id = 15652993950230557946
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

## Serializes a value of [UnidirectionalLight] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UnidirectionalLight -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Light.LightID.write_bytes(value.id)

## Deserializes a value of [UnidirectionalLight] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UnidirectionalLight _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 4 }) |> Light.LightID.from_bytes?,
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
