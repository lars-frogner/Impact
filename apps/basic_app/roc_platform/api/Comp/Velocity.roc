# Hash: 9e44e88be2b5442e00b00495271105f95b4e815fdf3bb2f6e513b4d545752e2e
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::motion::components::VelocityComp
# Type category: Component
# Commit: d505d37
module [
    Velocity,
    new,
    linear,
    angular,
    stationary,
    add_new,
    add_linear,
    add_angular,
    add_stationary,
    add,
    add_multiple,
]

import Entity
import Physics.AngularVelocity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that have a
## linear and/or angular velocity.
Velocity : {
    ## The linear velocity of the entity's reference frame in the parent space.
    linear : Vector3.Vector3 Binary64,
    ## The angular velocity of the entity's reference frame about its origin in
    ## the parent space.
    angular : Physics.AngularVelocity.AngularVelocity,
}

## Creates a new velocity component for an entity with the given linear and
## angular velocity.
new : Vector3.Vector3 Binary64, Physics.AngularVelocity.AngularVelocity -> Velocity
new = |linear_velocity, angular_velocity|
    {
        linear: linear_velocity,
        angular: angular_velocity,
    }

## Creates a new velocity component for an entity with the given linear and
## angular velocity.
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary64, Physics.AngularVelocity.AngularVelocity -> Entity.Data
add_new = |data, linear_velocity, angular_velocity|
    add(data, new(linear_velocity, angular_velocity))

## Creates a new velocity component for an entity with the given linear
## velocity and zero angular velocity.
linear : Vector3.Vector3 Binary64 -> Velocity
linear = |velocity|
    new(velocity, Physics.AngularVelocity.zero({}))

## Creates a new velocity component for an entity with the given linear
## velocity and zero angular velocity.
## Adds the component to the given entity's data.
add_linear : Entity.Data, Vector3.Vector3 Binary64 -> Entity.Data
add_linear = |data, velocity|
    add(data, linear(velocity))

## Creates a new velocity component for an entity with the given angular
## velocity and zero linear velocity.
angular : Physics.AngularVelocity.AngularVelocity -> Velocity
angular = |velocity|
    new(Vector3.zero, velocity)

## Creates a new velocity component for an entity with the given angular
## velocity and zero linear velocity.
## Adds the component to the given entity's data.
add_angular : Entity.Data, Physics.AngularVelocity.AngularVelocity -> Entity.Data
add_angular = |data, velocity|
    add(data, angular(velocity))

## Creates a new velocity component for an entity with the zero linear and
## angular velocity.
stationary : {} -> Velocity
stationary = |{}|
    linear(Vector3.zero)

## Creates a new velocity component for an entity with the zero linear and
## angular velocity.
## Adds the component to the given entity's data.
add_stationary : Entity.Data -> Entity.Data
add_stationary = |data|
    add(data, stationary({}))

## Adds a value of the [Velocity] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Velocity -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Velocity] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Velocity -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Velocity -> List U8
write_packet = |bytes, value|
    type_id = 17258100226553216954
    size = 56
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Velocity -> List U8
write_multi_packet = |bytes, values|
    type_id = 17258100226553216954
    size = 56
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

## Serializes a value of [Velocity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Velocity -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Vector3.write_bytes_64(value.linear)
    |> Physics.AngularVelocity.write_bytes(value.angular)

## Deserializes a value of [Velocity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Velocity _
from_bytes = |bytes|
    Ok(
        {
            linear: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            angular: bytes |> List.sublist({ start: 24, len: 32 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
