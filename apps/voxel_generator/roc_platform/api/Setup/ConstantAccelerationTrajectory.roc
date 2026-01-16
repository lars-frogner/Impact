# Hash: 403e6c796fc5a183
# Generated: 2026-01-16T10:03:37.04536142
# Rust type: impact_physics::driven_motion::constant_acceleration::ConstantAccelerationTrajectory
# Type category: Component
module [
    ConstantAccelerationTrajectory,
    new,
    with_constant_velocity,
    add_new,
    add_multiple_new,
    add_with_constant_velocity,
    add_multiple_with_constant_velocity,
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

## A trajectory with constant acceleration.
ConstantAccelerationTrajectory : {
    ## When (in simulation time) the body should be at the initial position.
    initial_time : F32,
    ## The position of the body at the initial time.
    initial_position : Point3.Point3,
    ## The velocity of the body at the initial time.
    initial_velocity : Vector3.Vector3,
    ## The constant acceleration of the body.
    acceleration : Vector3.Vector3,
}

## Creates a new constant acceleration trajectory with the given properties.
new : F32, Point3.Point3, Vector3.Vector3, Vector3.Vector3 -> ConstantAccelerationTrajectory
new = |initial_time, initial_position, initial_velocity, acceleration|
    {
        initial_time,
        initial_position,
        initial_velocity,
        acceleration,
    }

## Creates a new constant acceleration trajectory with the given properties.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, Point3.Point3, Vector3.Vector3, Vector3.Vector3 -> Entity.ComponentData
add_new = |entity_data, initial_time, initial_position, initial_velocity, acceleration|
    add(entity_data, new(initial_time, initial_position, initial_velocity, acceleration))

## Creates a new constant acceleration trajectory with the given properties.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (Point3.Point3), Entity.Arg.Broadcasted (Vector3.Vector3), Entity.Arg.Broadcasted (Vector3.Vector3) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, initial_time, initial_position, initial_velocity, acceleration|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            initial_time, initial_position, initial_velocity, acceleration,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates a new constant velocity trajectory (no acceleration) with the
## given properties.
with_constant_velocity : F32, Point3.Point3, Vector3.Vector3 -> ConstantAccelerationTrajectory
with_constant_velocity = |initial_time, initial_position, velocity|
    new(
        initial_time,
        initial_position,
        velocity,
        Vector3.zeros,
    )

## Creates a new constant velocity trajectory (no acceleration) with the
## given properties.
## Adds the component to the given entity's data.
add_with_constant_velocity : Entity.ComponentData, F32, Point3.Point3, Vector3.Vector3 -> Entity.ComponentData
add_with_constant_velocity = |entity_data, initial_time, initial_position, velocity|
    add(entity_data, with_constant_velocity(initial_time, initial_position, velocity))

## Creates a new constant velocity trajectory (no acceleration) with the
## given properties.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_constant_velocity : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (Point3.Point3), Entity.Arg.Broadcasted (Vector3.Vector3) -> Result Entity.MultiComponentData Str
add_multiple_with_constant_velocity = |entity_data, initial_time, initial_position, velocity|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            initial_time, initial_position, velocity,
            Entity.multi_count(entity_data),
            with_constant_velocity
        ))
    )

## Adds a value of the [ConstantAccelerationTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ConstantAccelerationTrajectory -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConstantAccelerationTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ConstantAccelerationTrajectory) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ConstantAccelerationTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ConstantAccelerationTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 12430862769938531894
    size = 40
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConstantAccelerationTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 12430862769938531894
    size = 40
    alignment = 4
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
    |> List.reserve(40)
    |> Builtin.write_bytes_f32(value.initial_time)
    |> Point3.write_bytes(value.initial_position)
    |> Vector3.write_bytes(value.initial_velocity)
    |> Vector3.write_bytes(value.acceleration)

## Deserializes a value of [ConstantAccelerationTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationTrajectory _
from_bytes = |bytes|
    Ok(
        {
            initial_time: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            initial_position: bytes |> List.sublist({ start: 4, len: 12 }) |> Point3.from_bytes?,
            initial_velocity: bytes |> List.sublist({ start: 16, len: 12 }) |> Vector3.from_bytes?,
            acceleration: bytes |> List.sublist({ start: 28, len: 12 }) |> Vector3.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 40 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
