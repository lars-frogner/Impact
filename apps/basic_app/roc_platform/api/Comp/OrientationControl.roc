# Hash: e2de5e3a049055fe1de60ffeb6a1f65737e9a8a70d30e177b22458b335eb52d8
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::control::orientation::components::OrientationControlComp
# Type category: Component
# Commit: d505d37
module [
    OrientationControl,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Physics.AngularVelocity
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities whose
## orientation that can be controlled by a user.
OrientationControl : {
    control_angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

## Creates a new component for orientation control.
new : {} -> OrientationControl
new = |{}|
    { control_angular_velocity: Physics.AngularVelocity.zero({}) }

## Creates a new component for orientation control.
## Adds the component to the given entity's data.
add_new : Entity.Data -> Entity.Data
add_new = |data|
    add(data, new({}))

## Adds a value of the [OrientationControl] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, OrientationControl -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [OrientationControl] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List OrientationControl -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, OrientationControl -> List U8
write_packet = |bytes, value|
    type_id = 13759247365815440278
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List OrientationControl -> List U8
write_multi_packet = |bytes, values|
    type_id = 13759247365815440278
    size = 32
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

## Serializes a value of [OrientationControl] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrientationControl -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Physics.AngularVelocity.write_bytes(value.control_angular_velocity)

## Deserializes a value of [OrientationControl] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrientationControl _
from_bytes = |bytes|
    Ok(
        {
            control_angular_velocity: bytes |> List.sublist({ start: 0, len: 32 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
