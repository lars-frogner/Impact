# Hash: ca262aaf11ef58281e435382c71adc539cd9403bfaad06aced0d5ceca276e578
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact_camera::setup::OrthographicCamera
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    OrthographicCamera,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Radians

## Properties of an [`OrthographicCamera`](crate::camera::OrthographicCamera).
OrthographicCamera : {
    vertical_field_of_view : Radians.Radians Binary32,
    near_distance : F32,
    far_distance : F32,
}

## Creates a new value representing an
## [`OrthographicCamera`](crate::camera::OrthographicCamera) with the given
## vertical field of view (in radians) and near and far distance.
##
## # Panics
## If the field of view or the near distance does not exceed zero, or if
## the far distance does not exceed the near distance.
new : Radians.Radians Binary32, F32, F32 -> OrthographicCamera
new = |vertical_field_of_view, near_distance, far_distance|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect vertical_field_of_view > 0.0
    # expect near_distance > 0.0
    # expect far_distance > near_distance
    {
        vertical_field_of_view,
        near_distance,
        far_distance
    }

## Creates a new value representing an
## [`OrthographicCamera`](crate::camera::OrthographicCamera) with the given
## vertical field of view (in radians) and near and far distance.
##
## # Panics
## If the field of view or the near distance does not exceed zero, or if
## the far distance does not exceed the near distance.
## Adds the component to the given entity's data.
add_new : Entity.Data, Radians.Radians Binary32, F32, F32 -> Entity.Data
add_new = |entity_data, vertical_field_of_view, near_distance, far_distance|
    add(entity_data, new(vertical_field_of_view, near_distance, far_distance))

## Creates a new value representing an
## [`OrthographicCamera`](crate::camera::OrthographicCamera) with the given
## vertical field of view (in radians) and near and far distance.
##
## # Panics
## If the field of view or the near distance does not exceed zero, or if
## the far distance does not exceed the near distance.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Radians.Radians Binary32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, vertical_field_of_view, near_distance, far_distance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            vertical_field_of_view, near_distance, far_distance,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [OrthographicCamera] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, OrthographicCamera -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [OrthographicCamera] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (OrthographicCamera) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in OrthographicCamera.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, OrthographicCamera -> List U8
write_packet = |bytes, val|
    type_id = 1551354808382789169
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List OrthographicCamera -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1551354808382789169
    size = 12
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

## Serializes a value of [OrthographicCamera] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, OrthographicCamera -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Radians.write_bytes_32(value.vertical_field_of_view)
    |> Builtin.write_bytes_f32(value.near_distance)
    |> Builtin.write_bytes_f32(value.far_distance)

## Deserializes a value of [OrthographicCamera] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result OrthographicCamera _
from_bytes = |bytes|
    Ok(
        {
            vertical_field_of_view: bytes |> List.sublist({ start: 0, len: 4 }) |> Radians.from_bytes_32?,
            near_distance: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            far_distance: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
