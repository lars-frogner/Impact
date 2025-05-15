# Hash: 32b7ea9f4488bd3e8bc029b714317f39fd4a35e183af93de347cfaf2ad3307c5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::motion::analytical::circular::components::CircularTrajectoryComp
# Type category: Component
# Commit: d505d37
module [
    CircularTrajectory,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Point3
import core.UnitQuaternion

## [`Component`](impact_ecs::component::Component) for entities that follow a
## circular trajectory over time with constant speed.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
## [`VelocityComp`](crate::physics::VelocityComp).
CircularTrajectory : {
    ## When (in simulation time) the entity should be at the initial position
    ## on the circle.
    initial_time : F64,
    ## The orientation of the orbit. The first axis of the circle's reference
    ## frame will coincide with the direction from the center to the position
    ## of the entity at the initial time, the second with the direction of the
    ## velocity at the initial time and the third with the normal of the
    ## circle's plane.
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    ## The position of the center of the circle.
    center_position : Point3.Point3 Binary64,
    ## The radius of the circle.
    radius : F64,
    ## The duration of one revolution.
    period : F64,
}

## Creates a new component for a circular trajectory with the given
## properties.
new : F64, UnitQuaternion.UnitQuaternion Binary64, Point3.Point3 Binary64, F64, F64 -> CircularTrajectory
new = |initial_time, orientation, center_position, radius, period|
    {
        initial_time,
        orientation,
        center_position,
        radius,
        period,
    }

## Creates a new component for a circular trajectory with the given
## properties.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, UnitQuaternion.UnitQuaternion Binary64, Point3.Point3 Binary64, F64, F64 -> Entity.Data
add_new = |data, initial_time, orientation, center_position, radius, period|
    add(data, new(initial_time, orientation, center_position, radius, period))

## Adds a value of the [CircularTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, CircularTrajectory -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [CircularTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List CircularTrajectory -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, CircularTrajectory -> List U8
write_packet = |bytes, value|
    type_id = 16131368659324112954
    size = 80
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List CircularTrajectory -> List U8
write_multi_packet = |bytes, values|
    type_id = 16131368659324112954
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

## Serializes a value of [CircularTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CircularTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Builtin.write_bytes_f64(value.initial_time)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Point3.write_bytes_64(value.center_position)
    |> Builtin.write_bytes_f64(value.radius)
    |> Builtin.write_bytes_f64(value.period)

## Deserializes a value of [CircularTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularTrajectory _
from_bytes = |bytes|
    Ok(
        {
            initial_time: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            orientation: bytes |> List.sublist({ start: 8, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            center_position: bytes |> List.sublist({ start: 40, len: 24 }) |> Point3.from_bytes_64?,
            radius: bytes |> List.sublist({ start: 64, len: 8 }) |> Builtin.from_bytes_f64?,
            period: bytes |> List.sublist({ start: 72, len: 8 }) |> Builtin.from_bytes_f64?,
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
