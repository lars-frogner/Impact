# Hash: 789c928e4ee8fc1af434a88ec25cee2b081f49ff536d64b6bd7b0130c0904fe3
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::light::components::AmbientLightComp
# Type category: Component
# Commit: d505d37
module [
    AmbientLight,
    add,
    add_multiple,
]

import Entity
import Light.LightID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have an
## [`AmbientLight`](crate::light::AmbientLight).
AmbientLight : {
    ## The ID of the entity's [`AmbientLight`](crate::light::AmbientLight).
    id : Light.LightID.LightID,
}

## Adds a value of the [AmbientLight] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, AmbientLight -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [AmbientLight] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List AmbientLight -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, AmbientLight -> List U8
write_packet = |bytes, value|
    type_id = 1932867273800880306
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List AmbientLight -> List U8
write_multi_packet = |bytes, values|
    type_id = 1932867273800880306
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

## Serializes a value of [AmbientLight] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AmbientLight -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Light.LightID.write_bytes(value.id)

## Deserializes a value of [AmbientLight] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AmbientLight _
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
