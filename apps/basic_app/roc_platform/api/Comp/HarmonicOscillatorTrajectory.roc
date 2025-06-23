# Hash: 223046098c48ac4169ca7fe12735b65c61cce81b1bf453907e89a0000f17da5d
# Generated: 2025-06-23T21:05:32+00:00
# Rust type: impact::physics::motion::analytical::harmonic_oscillation::components::HarmonicOscillatorTrajectoryComp
# Type category: Component
# Commit: 6a2f327 (dirty)
module [
    HarmonicOscillatorTrajectory,
    new,
    add_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.UnitVector3

## [`Component`](impact_ecs::component::Component) for entities that follow a
## trajectory with harmonically oscillating position, velocity and acceleration
## over time.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::motion::components::ReferenceFrameComp) and a
## [`VelocityComp`](crate::physics::motion::components::VelocityComp).
HarmonicOscillatorTrajectory : {
    ## A simulation time when the entity should be at the center of
    ## oscillation.
    center_time : F64,
    ## The position of the center of oscillation.
    center_position : Point3.Point3 Binary64,
    ## The direction in which the entity is oscillating back and forth.
    direction : UnitVector3.UnitVector3 Binary64,
    ## The maximum distance of the entity from the center position.
    amplitude : F64,
    ## The duration of one full oscillation.
    period : F64,
}

## Creates a new component for an harmonically oscillating trajectory with
## the given properties.
new : F64, Point3.Point3 Binary64, UnitVector3.UnitVector3 Binary64, F64, F64 -> HarmonicOscillatorTrajectory
new = |center_time, center_position, direction, amplitude, period|
    {
        center_time,
        center_position,
        direction,
        amplitude,
        period,
    }

## Creates a new component for an harmonically oscillating trajectory with
## the given properties.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, Point3.Point3 Binary64, UnitVector3.UnitVector3 Binary64, F64, F64 -> Entity.Data
add_new = |entity_data, center_time, center_position, direction, amplitude, period|
    add(entity_data, new(center_time, center_position, direction, amplitude, period))

## Adds a value of the [HarmonicOscillatorTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, HarmonicOscillatorTrajectory -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [HarmonicOscillatorTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (HarmonicOscillatorTrajectory) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in HarmonicOscillatorTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, HarmonicOscillatorTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 1583612721431612208
    size = 72
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List HarmonicOscillatorTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1583612721431612208
    size = 72
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

## Serializes a value of [HarmonicOscillatorTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, HarmonicOscillatorTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(72)
    |> Builtin.write_bytes_f64(value.center_time)
    |> Point3.write_bytes_64(value.center_position)
    |> UnitVector3.write_bytes_64(value.direction)
    |> Builtin.write_bytes_f64(value.amplitude)
    |> Builtin.write_bytes_f64(value.period)

## Deserializes a value of [HarmonicOscillatorTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result HarmonicOscillatorTrajectory _
from_bytes = |bytes|
    Ok(
        {
            center_time: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            center_position: bytes |> List.sublist({ start: 8, len: 24 }) |> Point3.from_bytes_64?,
            direction: bytes |> List.sublist({ start: 32, len: 24 }) |> UnitVector3.from_bytes_64?,
            amplitude: bytes |> List.sublist({ start: 56, len: 8 }) |> Builtin.from_bytes_f64?,
            period: bytes |> List.sublist({ start: 64, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 72 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
