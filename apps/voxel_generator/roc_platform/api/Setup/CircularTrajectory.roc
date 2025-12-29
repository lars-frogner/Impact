# Hash: df122da49eb47c3c
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_physics::driven_motion::circular::CircularTrajectory
# Type category: Component
module [
    CircularTrajectory,
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
import core.UnitQuaternion

## A circular trajectory with constant speed.
CircularTrajectory : {
    ## When (in simulation time) the body should be at the initial position
    ## on the circle.
    initial_time : F32,
    ## The orientation of the orbit. The first axis of the circle's reference
    ## frame will coincide with the direction from the center to the position
    ## of the body at the initial time, the second with the direction of the
    ## velocity at the initial time and the third with the normal of the
    ## circle's plane.
    orientation : UnitQuaternion.UnitQuaternion,
    ## The position of the center of the circle.
    center_position : Point3.Point3,
    ## The radius of the circle.
    radius : F32,
    ## The duration of one revolution.
    period : F32,
}

## Creates a new circular trajectory with the given properties.
new : F32, UnitQuaternion.UnitQuaternion, Point3.Point3, F32, F32 -> CircularTrajectory
new = |initial_time, orientation, center_position, radius, period|
    {
        initial_time,
        orientation,
        center_position,
        radius,
        period,
    }

## Creates a new circular trajectory with the given properties.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, UnitQuaternion.UnitQuaternion, Point3.Point3, F32, F32 -> Entity.ComponentData
add_new = |entity_data, initial_time, orientation, center_position, radius, period|
    add(entity_data, new(initial_time, orientation, center_position, radius, period))

## Adds a value of the [CircularTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, CircularTrajectory -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [CircularTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (CircularTrajectory) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in CircularTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, CircularTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 11253132944081296891
    size = 40
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List CircularTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 11253132944081296891
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

## Serializes a value of [CircularTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CircularTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(40)
    |> Builtin.write_bytes_f32(value.initial_time)
    |> UnitQuaternion.write_bytes(value.orientation)
    |> Point3.write_bytes(value.center_position)
    |> Builtin.write_bytes_f32(value.radius)
    |> Builtin.write_bytes_f32(value.period)

## Deserializes a value of [CircularTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularTrajectory _
from_bytes = |bytes|
    Ok(
        {
            initial_time: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            orientation: bytes |> List.sublist({ start: 4, len: 16 }) |> UnitQuaternion.from_bytes?,
            center_position: bytes |> List.sublist({ start: 20, len: 12 }) |> Point3.from_bytes?,
            radius: bytes |> List.sublist({ start: 32, len: 4 }) |> Builtin.from_bytes_f32?,
            period: bytes |> List.sublist({ start: 36, len: 4 }) |> Builtin.from_bytes_f32?,
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
