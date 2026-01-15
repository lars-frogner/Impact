# Hash: 7cf424af91e1486b
# Generated: 2026-01-15T22:29:16.219696086
# Rust type: impact_physics::force::alignment_torque::FixedDirectionAlignmentTorque
# Type category: Component
module [
    FixedDirectionAlignmentTorque,
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
import core.UnitVector3

## A torque working to align an axis of the body with a fixed external
## direction.
FixedDirectionAlignmentTorque : {
    ## The local axis of the body to align.
    axis_to_align : UnitVector3.UnitVector3,
    ## The external direction to align with.
    alignment_direction : UnitVector3.UnitVector3,
    ## The approximate time the torque should take to achieve the alignment.
    settling_time : F32,
    ## The strength with which to damp the component of angular velocity
    ## around the axis to align.
    spin_damping : F32,
    ## The strength with which to damp the component of angular velocity
    ## causing precession around the alignement direction.
    precession_damping : F32,
}

new : UnitVector3.UnitVector3, UnitVector3.UnitVector3, F32, F32, F32 -> FixedDirectionAlignmentTorque
new = |axis_to_align, alignment_direction, settling_time, spin_damping, precession_damping|
    { axis_to_align, alignment_direction, settling_time, spin_damping, precession_damping }

add_new : Entity.ComponentData, UnitVector3.UnitVector3, UnitVector3.UnitVector3, F32, F32, F32 -> Entity.ComponentData
add_new = |entity_data, axis_to_align, alignment_direction, settling_time, spin_damping, precession_damping|
    add(entity_data, new(axis_to_align, alignment_direction, settling_time, spin_damping, precession_damping))

## Adds a value of the [FixedDirectionAlignmentTorque] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, FixedDirectionAlignmentTorque -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [FixedDirectionAlignmentTorque] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (FixedDirectionAlignmentTorque) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in FixedDirectionAlignmentTorque.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, FixedDirectionAlignmentTorque -> List U8
write_packet = |bytes, val|
    type_id = 16185048670009595002
    size = 36
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List FixedDirectionAlignmentTorque -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16185048670009595002
    size = 36
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

## Serializes a value of [FixedDirectionAlignmentTorque] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FixedDirectionAlignmentTorque -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(36)
    |> UnitVector3.write_bytes(value.axis_to_align)
    |> UnitVector3.write_bytes(value.alignment_direction)
    |> Builtin.write_bytes_f32(value.settling_time)
    |> Builtin.write_bytes_f32(value.spin_damping)
    |> Builtin.write_bytes_f32(value.precession_damping)

## Deserializes a value of [FixedDirectionAlignmentTorque] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FixedDirectionAlignmentTorque _
from_bytes = |bytes|
    Ok(
        {
            axis_to_align: bytes |> List.sublist({ start: 0, len: 12 }) |> UnitVector3.from_bytes?,
            alignment_direction: bytes |> List.sublist({ start: 12, len: 12 }) |> UnitVector3.from_bytes?,
            settling_time: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_f32?,
            spin_damping: bytes |> List.sublist({ start: 28, len: 4 }) |> Builtin.from_bytes_f32?,
            precession_damping: bytes |> List.sublist({ start: 32, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 36 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
