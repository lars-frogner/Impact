# Hash: fa628e82f710902e4b2797a7e76e36f88220ff8b346f703396a73249415350d4
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::control::motion::components::MotionControlComp
# Type category: Component
# Commit: d505d37
module [
    MotionControl,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities whose motion
## that can be controlled by a user.
MotionControl : {
    control_velocity : Vector3.Vector3 Binary64,
}

## Creates a new component for motion control.
new : {} -> MotionControl
new = |{}|
    { control_velocity: Vector3.zero }

## Creates a new component for motion control.
## Adds the component to the given entity's data.
add_new : Entity.Data -> Entity.Data
add_new = |data|
    add(data, new({}))

## Adds a value of the [MotionControl] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MotionControl -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [MotionControl] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List MotionControl -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, MotionControl -> List U8
write_packet = |bytes, value|
    type_id = 15665890016094755610
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List MotionControl -> List U8
write_multi_packet = |bytes, values|
    type_id = 15665890016094755610
    size = 24
    alignment = 8
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

## Serializes a value of [MotionControl] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MotionControl -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes_64(value.control_velocity)

## Deserializes a value of [MotionControl] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MotionControl _
from_bytes = |bytes|
    Ok(
        {
            control_velocity: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
