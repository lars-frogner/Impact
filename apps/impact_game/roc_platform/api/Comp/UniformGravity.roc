# Hash: 7a6d0246632b97532b44ccbf7e8e08763ffba04ec973135e748b41a1d6cf0aa0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::forces::uniform_gravity::components::UniformGravityComp
# Type category: Component
# Commit: d505d37
module [
    UniformGravity,
    earth_downward_acceleration,
    new,
    downward,
    earth,
    add_new,
    add_downward,
    add_earth,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## uniform gravitational acceleration.
UniformGravity : {
    ## The gravitational acceleration of the entity.
    acceleration : Vector3.Vector3 Binary64,
}

## The downward acceleration at the surface of Earth [m/s^2].
earth_downward_acceleration : F64
earth_downward_acceleration = 9.81

## Creates a new component for uniform gravitational acceleration.
new : Vector3.Vector3 Binary64 -> UniformGravity
new = |acceleration|
    { acceleration }

## Creates a new component for uniform gravitational acceleration.
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary64 -> Entity.Data
add_new = |data, acceleration|
    add(data, new(acceleration))

## Creates a new component for uniform gravitational acceleration in the
## negative y-direction.
downward : F64 -> UniformGravity
downward = |acceleration|
    new((0.0, -acceleration, 0.0))

## Creates a new component for uniform gravitational acceleration in the
## negative y-direction.
## Adds the component to the given entity's data.
add_downward : Entity.Data, F64 -> Entity.Data
add_downward = |data, acceleration|
    add(data, downward(acceleration))

## Creates a new component for the gravitational acceleration at the
## surface of Earth.
earth : {} -> UniformGravity
earth = |{}|
    downward(earth_downward_acceleration)

## Creates a new component for the gravitational acceleration at the
## surface of Earth.
## Adds the component to the given entity's data.
add_earth : Entity.Data -> Entity.Data
add_earth = |data|
    add(data, earth({}))

## Adds a value of the [UniformGravity] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformGravity -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UniformGravity] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UniformGravity -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UniformGravity -> List U8
write_packet = |bytes, value|
    type_id = 5075699606031977644
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UniformGravity -> List U8
write_multi_packet = |bytes, values|
    type_id = 5075699606031977644
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

## Serializes a value of [UniformGravity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformGravity -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes_64(value.acceleration)

## Deserializes a value of [UniformGravity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformGravity _
from_bytes = |bytes|
    Ok(
        {
            acceleration: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
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
