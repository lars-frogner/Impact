# Hash: efa8888ebf4fe61a52b5ecbb9f20e68964b548d145cd81987723f7706a94fa78
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact_mesh::setup::BoxMesh
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    BoxMesh,
    unit_cube,
    skybox,
    new,
    add_unit_cube,
    add_multiple_unit_cube,
    add_skybox,
    add_multiple_skybox,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Mesh.FrontFaceSide
import core.Builtin

## A mesh consisting of an axis-aligned box centered on the origin.
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
add_unit_cube = |entity_data|
    add(entity_data, unit_cube)

add_multiple_unit_cube : Entity.MultiData -> Entity.MultiData
add_multiple_unit_cube = |entity_data|
    res = add_multiple(
        entity_data,
        Same(unit_cube)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in BoxMesh.add_multiple_unit_cube: ${Inspect.to_str(err)}"


skybox : BoxMesh
skybox = { extent_x: 1.0, extent_y: 1.0, extent_z: 1.0, front_faces_on_outside: 0 }

add_skybox : Entity.Data -> Entity.Data
add_skybox = |entity_data|
    add(entity_data, skybox)

add_multiple_skybox : Entity.MultiData -> Entity.MultiData
add_multiple_skybox = |entity_data|
    res = add_multiple(
        entity_data,
        Same(skybox)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in BoxMesh.add_multiple_skybox: ${Inspect.to_str(err)}"


## Defines a box mesh with the given extents.
new : F32, F32, F32, Mesh.FrontFaceSide.FrontFaceSide -> BoxMesh
new = |extent_x, extent_y, extent_z, front_face_side|
    front_faces_on_outside =
        when front_face_side is
            Outside -> 1
            Inside -> 0
    {
        extent_x,
        extent_y,
        extent_z,
        front_faces_on_outside,
    }

## Defines a box mesh with the given extents.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32, F32, Mesh.FrontFaceSide.FrontFaceSide -> Entity.Data
add_new = |entity_data, extent_x, extent_y, extent_z, front_face_side|
    add(entity_data, new(extent_x, extent_y, extent_z, front_face_side))

## Defines a box mesh with the given extents.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (Mesh.FrontFaceSide.FrontFaceSide) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, extent_x, extent_y, extent_z, front_face_side|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            extent_x, extent_y, extent_z, front_face_side,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [BoxMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, BoxMesh -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [BoxMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (BoxMesh) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in BoxMesh.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, BoxMesh -> List U8
write_packet = |bytes, val|
    type_id = 6532870238707465906
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List BoxMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 6532870238707465906
    size = 16
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
