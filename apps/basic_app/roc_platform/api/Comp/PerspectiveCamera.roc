# Hash: d2daa52b287f2aed12721bd57d3fc12d33d3fab0dfd1e744cc0e92e638678550
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::camera::components::PerspectiveCameraComp
# Type category: Component
# Commit: d505d37
module [
    PerspectiveCamera,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Degrees

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a
## [`PerspectiveCamera`](crate::camera::PerspectiveCamera).
##
## The purpose of this component is to aid in constructing a
## [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
## entity and a [`SceneCamera`](crate::camera::SceneCamera) for the
## [`Scene`](crate::scene::Scene). It is therefore not kept after entity
## creation.
PerspectiveCamera : {
    vertical_field_of_view_rad : F32,
    near_distance : F32,
    far_distance : F32,
}

## Creates a new component representing a
## [`PerspectiveCamera`](crate::camera::PerspectiveCamera) with the given
## vertical field of view (in degrees) and near and far distance.
##
## # Panics
## If the field of view or the near distance does not exceed zero, or if
## the far distance does not exceed the near distance.
new : Degrees.Degrees Binary32, F32, F32 -> PerspectiveCamera
new = |vertical_field_of_view, near_distance, far_distance|
    expect vertical_field_of_view > 0.0
    expect near_distance > 0.0
    expect far_distance > near_distance
    vertical_field_of_view_rad = Degrees.to_radians(vertical_field_of_view)
    {
        vertical_field_of_view_rad,
        near_distance,
        far_distance,
    }

## Creates a new component representing a
## [`PerspectiveCamera`](crate::camera::PerspectiveCamera) with the given
## vertical field of view (in degrees) and near and far distance.
##
## # Panics
## If the field of view or the near distance does not exceed zero, or if
## the far distance does not exceed the near distance.
## Adds the component to the given entity's data.
add_new : Entity.Data, Degrees.Degrees Binary32, F32, F32 -> Entity.Data
add_new = |data, vertical_field_of_view, near_distance, far_distance|
    add(data, new(vertical_field_of_view, near_distance, far_distance))

## Adds a value of the [PerspectiveCamera] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, PerspectiveCamera -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [PerspectiveCamera] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List PerspectiveCamera -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, PerspectiveCamera -> List U8
write_packet = |bytes, value|
    type_id = 10477796021710556922
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List PerspectiveCamera -> List U8
write_multi_packet = |bytes, values|
    type_id = 10477796021710556922
    size = 12
    alignment = 4
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

## Serializes a value of [PerspectiveCamera] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PerspectiveCamera -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(value.vertical_field_of_view_rad)
    |> Builtin.write_bytes_f32(value.near_distance)
    |> Builtin.write_bytes_f32(value.far_distance)

## Deserializes a value of [PerspectiveCamera] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PerspectiveCamera _
from_bytes = |bytes|
    Ok(
        {
            vertical_field_of_view_rad: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
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
