# Hash: a325d3c1d322a944af39043d72e27fe8a4e638c8d6ebd12adf897992bdced0d0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::motion::analytical::constant_acceleration::components::ConstantAccelerationTrajectoryComp
# Type category: Component
# Commit: d505d37
module [
    ConstantAccelerationTrajectory,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Point3
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that follow a
## fixed trajectory over time governed by a constant acceleration vector.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
## [`VelocityComp`](crate::physics::VelocityComp).
ConstantAccelerationTrajectory : {
    ## When (in simulation time) the entity should be at the initial position.
    initial_time : F64,
    ## The position of the entity at the initial time.
    initial_position : Point3.Point3 Binary64,
    ## The velocity of the entity at the initial time.
    initial_velocity : Vector3.Vector3 Binary64,
    ## The constant acceleration of the entity.
    acceleration : Vector3.Vector3 Binary64,
}

## Adds a value of the [ConstantAccelerationTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ConstantAccelerationTrajectory -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [ConstantAccelerationTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List ConstantAccelerationTrajectory -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, ConstantAccelerationTrajectory -> List U8
write_packet = |bytes, value|
    type_id = 238585499527754431
    size = 80
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List ConstantAccelerationTrajectory -> List U8
write_multi_packet = |bytes, values|
    type_id = 238585499527754431
    size = 80
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

## Serializes a value of [ConstantAccelerationTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAccelerationTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Builtin.write_bytes_f64(value.initial_time)
    |> Point3.write_bytes_64(value.initial_position)
    |> Vector3.write_bytes_64(value.initial_velocity)
    |> Vector3.write_bytes_64(value.acceleration)

## Deserializes a value of [ConstantAccelerationTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationTrajectory _
from_bytes = |bytes|
    Ok(
        {
            initial_time: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            initial_position: bytes |> List.sublist({ start: 8, len: 24 }) |> Point3.from_bytes_64?,
            initial_velocity: bytes |> List.sublist({ start: 32, len: 24 }) |> Vector3.from_bytes_64?,
            acceleration: bytes |> List.sublist({ start: 56, len: 24 }) |> Vector3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 80 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
