# Hash: b828cc049e8604effe4c2d291f22eed21a7f88a11b7d2ee3c82860836e147453
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::components::CylinderMeshComp
# Type category: Component
# Commit: d505d37
module [
    CylinderMesh,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose mesh is a vertical cylinder with the bottom centered on
## the origin.
##
## The purpose of this component is to aid in constructing a [`MeshComp`] for
## the entity. It is therefore not kept after entity creation.
CylinderMesh : {
    ## The length of the cylinder.
    length : F32,
    ## The diameter of the cylinder.
    diameter : F32,
    ## The number of vertices used for representing a circular cross-section of
    ## the cylinder.
    n_circumference_vertices : U32,
}

## Creates a new component for a cylinder mesh with the given length,
## diameter and number of circumeference vertices.
new : F32, F32, U32 -> CylinderMesh
new = |length, diameter, n_circumference_vertices|
    { length, diameter, n_circumference_vertices }

## Creates a new component for a cylinder mesh with the given length,
## diameter and number of circumeference vertices.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32, U32 -> Entity.Data
add_new = |data, length, diameter, n_circumference_vertices|
    add(data, new(length, diameter, n_circumference_vertices))

## Adds a value of the [CylinderMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, CylinderMesh -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [CylinderMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List CylinderMesh -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, CylinderMesh -> List U8
write_packet = |bytes, value|
    type_id = 7629497407946727154
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List CylinderMesh -> List U8
write_multi_packet = |bytes, values|
    type_id = 7629497407946727154
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

## Serializes a value of [CylinderMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CylinderMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(value.length)
    |> Builtin.write_bytes_f32(value.diameter)
    |> Builtin.write_bytes_u32(value.n_circumference_vertices)

## Deserializes a value of [CylinderMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CylinderMesh _
from_bytes = |bytes|
    Ok(
        {
            length: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            diameter: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            n_circumference_vertices: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_u32?,
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
