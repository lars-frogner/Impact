# Hash: b6dff2e8929828070ca8b1202892f5c8c65180c9eefd18d3bdea4e3bf89d7d94
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::physics::motion::analytical::constant_acceleration::components::ConstantAccelerationTrajectoryComp
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    ConstantAccelerationTrajectory,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that follow a
## fixed trajectory over time governed by a constant acceleration vector.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::motion::components::ReferenceFrameComp) and a
## [`VelocityComp`](crate::physics::motion::components::VelocityComp).
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
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConstantAccelerationTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ConstantAccelerationTrajectory) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ConstantAccelerationTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ConstantAccelerationTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 238585499527754431
    size = 80
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConstantAccelerationTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 238585499527754431
    size = 80
    alignment = 8
    count = List.len(vals)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    vals
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
