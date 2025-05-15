# Hash: 0ffb72abe0cc188d009a1d1a82893a449319d46a6261f2c04e8cc33570b96b2a
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::forces::detailed_drag::components::DragLoadMapComp
# Type category: Component
# Commit: d505d37
module [
    DragLoadMap,
    add,
    add_multiple,
]

import Entity
import Mesh.MeshID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have an
## associated [`DragLoadMap`](crate::physics::DragLoadMap) in the
## [`DragLoadMapRepository`](crate::physics::rigid_body::forces::DragLoadMapRepository).
DragLoadMap : {
    ## The ID of the mesh from which the drag load map was computed.
    mesh_id : Mesh.MeshID.MeshID,
    ## The drag coefficient of the body.
    drag_coefficient : F64,
}

## Adds a value of the [DragLoadMap] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DragLoadMap -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [DragLoadMap] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List DragLoadMap -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, DragLoadMap -> List U8
write_packet = |bytes, value|
    type_id = 2715277257896951626
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List DragLoadMap -> List U8
write_multi_packet = |bytes, values|
    type_id = 2715277257896951626
    size = 16
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

## Serializes a value of [DragLoadMap] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DragLoadMap -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Mesh.MeshID.write_bytes(value.mesh_id)
    |> Builtin.write_bytes_f64(value.drag_coefficient)

## Deserializes a value of [DragLoadMap] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DragLoadMap _
from_bytes = |bytes|
    Ok(
        {
            mesh_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Mesh.MeshID.from_bytes?,
            drag_coefficient: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
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
