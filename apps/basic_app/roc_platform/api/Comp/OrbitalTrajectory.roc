# Hash: 4c343a797aa05561f70c6ff21bd3b32b45763383e8e24c615b57afbdbfe4b241
# Generated: 2025-05-23T20:19:02+00:00
# Rust type: impact::physics::motion::analytical::orbit::components::OrbitalTrajectoryComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    OrbitalTrajectory,
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

## [`Component`](impact_ecs::component::Component) for entities that follow an
## closed orbital trajectory over time.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
## [`VelocityComp`](crate::physics::VelocityComp).
OrbitalTrajectory : {
    ## When (in simulation time) the entity should be at the periapsis (the
    ## closest point to the orbited body).
    periapsis_time : F64,
    ## The orientation of the orbit. The first axis of the oriented orbit frame
    ## will coincide with the direction from the orbited body to the periapsis,
    ## the second with the direction of the velocity at the periapsis and the
    ## third with the normal of the orbital plane.
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    ## The position of the focal point where the body being orbited would be
    ## located.
    focal_position : Point3.Point3 Binary64,
    ## Half the longest diameter of the orbital ellipse.
    semi_major_axis : F64,
    ## The eccentricity of the orbital ellipse (0 is circular, 1 is a line).
    eccentricity : F64,
    ## The orbital period.
    period : F64,
}

## Creates a new component for an orbital trajectory with the given
## properties.
new : F64, UnitQuaternion.UnitQuaternion Binary64, Point3.Point3 Binary64, F64, F64, F64 -> OrbitalTrajectory
new = |periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period|
    {
        periapsis_time,
        orientation,
        focal_position,
        semi_major_axis,
        eccentricity,
        period,
    }

## Creates a new component for an orbital trajectory with the given
## properties.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, UnitQuaternion.UnitQuaternion Binary64, Point3.Point3 Binary64, F64, F64, F64 -> Entity.Data
add_new = |entity_data, periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period|
    add(entity_data, new(periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period))

## Adds a value of the [OrbitalTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, OrbitalTrajectory -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [OrbitalTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (OrbitalTrajectory) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in OrbitalTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, OrbitalTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 13391131268911867523
    size = 88
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List OrbitalTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13391131268911867523
    size = 88
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

## Serializes a value of [OrbitalTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrbitalTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(88)
    |> Builtin.write_bytes_f64(value.periapsis_time)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Point3.write_bytes_64(value.focal_position)
    |> Builtin.write_bytes_f64(value.semi_major_axis)
    |> Builtin.write_bytes_f64(value.eccentricity)
    |> Builtin.write_bytes_f64(value.period)

## Deserializes a value of [OrbitalTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrbitalTrajectory _
from_bytes = |bytes|
    Ok(
        {
            periapsis_time: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            orientation: bytes |> List.sublist({ start: 8, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            focal_position: bytes |> List.sublist({ start: 40, len: 24 }) |> Point3.from_bytes_64?,
            semi_major_axis: bytes |> List.sublist({ start: 64, len: 8 }) |> Builtin.from_bytes_f64?,
            eccentricity: bytes |> List.sublist({ start: 72, len: 8 }) |> Builtin.from_bytes_f64?,
            period: bytes |> List.sublist({ start: 80, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 88 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
