# Hash: fcc208727db8cdf0c7c1471042efa21cd29dbe987e1b36f6a3fc8ca921c41f99
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::components::BoxMeshComp
# Type category: Component
# Commit: d505d37
module [
    BoxMesh,
    unit_cube,
    skybox,
    add_unit_cube,
    add_skybox,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose mesh is an axis-aligned box centered on the origin.
##
## The purpose of this component is to aid in constructing a [`MeshComp`] for
## the entity. It is therefore not kept after entity creation.
BoxMesh : {
    ## The extent of the box in the x-direction.
    extent_x : F32,
    ## The extent of the box in the y-direction.
    extent_y : F32,
    ## The extent of the box in the z-direction.
    extent_z : F32,
    front_faces_on_outside : U32,
}

unit_cube : BoxMesh
unit_cube = { extent_x: 1.0, extent_y: 1.0, extent_z: 1.0, front_faces_on_outside: 1 }

add_unit_cube : Entity.Data -> Entity.Data
add_unit_cube = |data|
    add(data, unit_cube)

skybox : BoxMesh
skybox = { extent_x: 1.0, extent_y: 1.0, extent_z: 1.0, front_faces_on_outside: 0 }

add_skybox : Entity.Data -> Entity.Data
add_skybox = |data|
    add(data, skybox)

## Adds a value of the [BoxMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, BoxMesh -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [BoxMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List BoxMesh -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, BoxMesh -> List U8
write_packet = |bytes, value|
    type_id = 11529368862797554621
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List BoxMesh -> List U8
write_multi_packet = |bytes, values|
    type_id = 11529368862797554621
    size = 16
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

## Serializes a value of [BoxMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, BoxMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(value.extent_x)
    |> Builtin.write_bytes_f32(value.extent_y)
    |> Builtin.write_bytes_f32(value.extent_z)
    |> Builtin.write_bytes_u32(value.front_faces_on_outside)

## Deserializes a value of [BoxMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result BoxMesh _
from_bytes = |bytes|
    Ok(
        {
            extent_x: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_y: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_z: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            front_faces_on_outside: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_u32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
