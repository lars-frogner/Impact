# Hash: a046e12144715ad1c249597a92b395f9e79d91f20f03ba28f11c9c473d0319f2
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::SceneEntityFlagsComp
# Type category: Component
# Commit: d505d37
module [
    SceneEntityFlags,
    add,
    add_multiple,
]

import Entity
import SceneEntityFlags
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that
## participate in a scene and have associated [`SceneEntityFlags`].
##
## If not specified, this component is automatically added to any new entity
## that has a model, light or rigid body.
SceneEntityFlags : SceneEntityFlags.SceneEntityFlags

## Adds a value of the [SceneEntityFlags] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneEntityFlags -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SceneEntityFlags] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SceneEntityFlags -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SceneEntityFlags -> List U8
write_packet = |bytes, value|
    type_id = 930069565650709728
    size = 1
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SceneEntityFlags -> List U8
write_multi_packet = |bytes, values|
    type_id = 930069565650709728
    size = 1
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

## Serializes a value of [SceneEntityFlags] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneEntityFlags -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1)
    |> SceneEntityFlags.write_bytes(value)

## Deserializes a value of [SceneEntityFlags] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneEntityFlags _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 1 }) |> SceneEntityFlags.from_bytes?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
