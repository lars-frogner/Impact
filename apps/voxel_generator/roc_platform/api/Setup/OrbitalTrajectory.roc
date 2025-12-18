# Hash: 62599acf09cbf1451940a7ace372154cacb9a33e0798730fbd9d83aacadeec5b
# Generated: 2025-12-17T23:54:08+00:00
# Rust type: impact_physics::driven_motion::orbit::OrbitalTrajectory
# Type category: Component
# Commit: 7d41822d (dirty)
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

## An orbital trajectory.
OrbitalTrajectory : {
    ## When (in simulation time) the orbiting body should be at the periapsis
    ## (the closest point to the orbited body).
    periapsis_time : F32,
    ## The orientation of the orbit. The first axis of the oriented orbit frame
    ## will coincide with the direction from the orbited body to the periapsis,
    ## the second with the direction of the velocity at the periapsis and the
    ## third with the normal of the orbital plane.
    orientation : UnitQuaternion.UnitQuaternion Binary32,
    ## The position of the focal point where the body being orbited would be
    ## located.
    focal_position : Point3.Point3 Binary32,
    ## Half the longest diameter of the orbital ellipse.
    semi_major_axis : F32,
    ## The eccentricity of the orbital ellipse (0 is circular, 1 is a line).
    eccentricity : F32,
    ## The orbital period.
    period : F32,
}

## Creates a new orbital trajectory with the given properties.
new : F32, UnitQuaternion.UnitQuaternion Binary32, Point3.Point3 Binary32, F32, F32, F32 -> OrbitalTrajectory
new = |periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period|
    {
        periapsis_time,
        orientation,
        focal_position,
        semi_major_axis,
        eccentricity,
        period,
    }

## Creates a new orbital trajectory with the given properties.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, UnitQuaternion.UnitQuaternion Binary32, Point3.Point3 Binary32, F32, F32, F32 -> Entity.ComponentData
add_new = |entity_data, periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period|
    add(entity_data, new(periapsis_time, orientation, focal_position, semi_major_axis, eccentricity, period))

## Adds a value of the [OrbitalTrajectory] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, OrbitalTrajectory -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [OrbitalTrajectory] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (OrbitalTrajectory) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in OrbitalTrajectory.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, OrbitalTrajectory -> List U8
write_packet = |bytes, val|
    type_id = 6364739483624798862
    size = 44
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List OrbitalTrajectory -> List U8
write_multi_packet = |bytes, vals|
    type_id = 6364739483624798862
    size = 44
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

## Serializes a value of [OrbitalTrajectory] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrbitalTrajectory -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(44)
    |> Builtin.write_bytes_f32(value.periapsis_time)
    |> UnitQuaternion.write_bytes_32(value.orientation)
    |> Point3.write_bytes_32(value.focal_position)
    |> Builtin.write_bytes_f32(value.semi_major_axis)
    |> Builtin.write_bytes_f32(value.eccentricity)
    |> Builtin.write_bytes_f32(value.period)

## Deserializes a value of [OrbitalTrajectory] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrbitalTrajectory _
from_bytes = |bytes|
    Ok(
        {
            periapsis_time: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            orientation: bytes |> List.sublist({ start: 4, len: 16 }) |> UnitQuaternion.from_bytes_32?,
            focal_position: bytes |> List.sublist({ start: 20, len: 12 }) |> Point3.from_bytes_32?,
            semi_major_axis: bytes |> List.sublist({ start: 32, len: 4 }) |> Builtin.from_bytes_f32?,
            eccentricity: bytes |> List.sublist({ start: 36, len: 4 }) |> Builtin.from_bytes_f32?,
            period: bytes |> List.sublist({ start: 40, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 44 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
