# Hash: 4b90afe2a2a2481052c7329123182e5163a54b6aa9914b28f0f536b573673185
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::components::MeshComp
# Type category: Component
# Commit: d505d37
module [
    Mesh,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Mesh.MeshID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have a
## [`TriangleMesh`](crate::mesh::TriangleMesh).
Mesh : {
    ## The ID of the entity's [`TriangleMesh`](crate::mesh::TriangleMesh).
    id : Mesh.MeshID.MeshID,
}

## Creates a new component representing a
## [`TriangleMesh`](crate::mesh::TriangleMesh) with the given ID.
new : Mesh.MeshID.MeshID -> Mesh
new = |mesh_id|
    { id: mesh_id }

## Creates a new component representing a
## [`TriangleMesh`](crate::mesh::TriangleMesh) with the given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Mesh.MeshID.MeshID -> Entity.Data
add_new = |data, mesh_id|
    add(data, new(mesh_id))

## Adds a value of the [Mesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Mesh -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Mesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Mesh -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Mesh -> List U8
write_packet = |bytes, value|
    type_id = 10372688481577730772
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Mesh -> List U8
write_multi_packet = |bytes, values|
    type_id = 10372688481577730772
    size = 8
    alignment = 8
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

## Serializes a value of [Mesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Mesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Mesh.MeshID.write_bytes(value.id)

## Deserializes a value of [Mesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Mesh _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 8 }) |> Mesh.MeshID.from_bytes?,
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
