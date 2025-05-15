# Hash: b3f14b3a95f110e6690ef1dc76827f26022720712df682eec69d955e633d4f6b
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::components::RectangleMeshComp
# Type category: Component
# Commit: d505d37
module [
    RectangleMesh,
    unit_square,
    new,
    add_unit_square,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose mesh is an axis-aligned horizontal rectangle centered on the
## origin, whose front face is on the positive y side.
##
## The purpose of this component is to aid in constructing a [`MeshComp`] for
## the entity. It is therefore not kept after entity creation.
RectangleMesh : {
    ## The extent of the rectangle in the x-direction.
    extent_x : F32,
    ## The extent of the rectangle in the z-direction.
    extent_z : F32,
}

unit_square : RectangleMesh
unit_square = { extent_x: 1.0, extent_z: 1.0 }

add_unit_square : Entity.Data -> Entity.Data
add_unit_square = |data|
    add(data, unit_square)

## Creates a new component for a rectangle mesh with the given horizontal
## extents.
new : F32, F32 -> RectangleMesh
new = |extent_x, extent_z|
    { extent_x, extent_z }

## Creates a new component for a rectangle mesh with the given horizontal
## extents.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32 -> Entity.Data
add_new = |data, extent_x, extent_z|
    add(data, new(extent_x, extent_z))

## Adds a value of the [RectangleMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, RectangleMesh -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [RectangleMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List RectangleMesh -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, RectangleMesh -> List U8
write_packet = |bytes, value|
    type_id = 6379808845987299625
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List RectangleMesh -> List U8
write_multi_packet = |bytes, values|
    type_id = 6379808845987299625
    size = 8
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

## Serializes a value of [RectangleMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RectangleMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(value.extent_x)
    |> Builtin.write_bytes_f32(value.extent_z)

## Deserializes a value of [RectangleMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RectangleMesh _
from_bytes = |bytes|
    Ok(
        {
            extent_x: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_z: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
