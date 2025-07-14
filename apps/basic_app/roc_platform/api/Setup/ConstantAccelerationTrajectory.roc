# Hash: e93a41e43129350149a0af7535456eddad0583243935ef4680df93544c9f037e
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::driven_motion::constant_acceleration::ConstantAccelerationTrajectory
# Type category: Component
# Commit: b1b4dfd8 (dirty)
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
    initial_time : F64,
    ## The position of the body at the initial time.
    initial_position : Point3.Point3 Binary64,
    ## The velocity of the body at the initial time.
    initial_velocity : Vector3.Vector3 Binary64,
    ## The constant acceleration of the body.
    acceleration : Vector3.Vector3 Binary64,
}

## Creates a new constant acceleration trajectory with the given properties.
new : F64, Point3.Point3 Binary64, Vector3.Vector3 Binary64, Vector3.Vector3 Binary64 -> ConstantAccelerationTrajectory
new = |initial_time, initial_position, initial_velocity, acceleration|
    {
        initial_time,
        initial_position,
        initial_velocity,
        acceleration,
    }

## Creates a new constant acceleration trajectory with the given properties.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, Point3.Point3 Binary64, Vector3.Vector3 Binary64, Vector3.Vector3 Binary64 -> Entity.Data
add_new = |entity_data, initial_time, initial_position, initial_velocity, acceleration|
    add(entity_data, new(initial_time, initial_position, initial_velocity, acceleration))

## Creates a new constant acceleration trajectory with the given properties.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Vector3.Vector3 Binary64) -> Result Entity.MultiData Str
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
with_constant_velocity : F64, Point3.Point3 Binary64, Vector3.Vector3 Binary64 -> ConstantAccelerationTrajectory
with_constant_velocity = |initial_time, initial_position, velocity|
    new(
        initial_time,
        initial_position,
        velocity,
        Vector3.zero,
    )

## Creates a new constant velocity trajectory (no acceleration) with the
## given properties.
## Adds the component to the given entity's data.
add_with_constant_velocity : Entity.Data, F64, Point3.Point3 Binary64, Vector3.Vector3 Binary64 -> Entity.Data
add_with_constant_velocity = |entity_data, initial_time, initial_position, velocity|
    add(entity_data, with_constant_velocity(initial_time, initial_position, velocity))

## Creates a new constant velocity trajectory (no acceleration) with the
## given properties.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_constant_velocity : Entity.MultiData, Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (Vector3.Vector3 Binary64) -> Result Entity.MultiData Str
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
    type_id = 12430862769938531894
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
    type_id = 12430862769938531894
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
