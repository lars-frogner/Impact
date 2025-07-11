# Hash: cfdd5f5a0cb31997e41102599f8221a761a9b29b67833a19cf2c7ad77294be73
# Generated: 2025-07-07T19:02:48+00:00
# Rust type: impact::physics::rigid_body::forces::detailed_drag::components::DragLoadMapComp
# Type category: Component
# Commit: 503a2ec (dirty)
module [
    DragLoadMap,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Mesh.TriangleMeshID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have an
## associated
## [`DragLoadMap`](crate::physics::rigid_body::forces::detailed_drag::DragLoadMap)
## in the
## [`DragLoadMapRepository`](crate::physics::rigid_body::forces::detailed_drag::DragLoadMapRepository).
DragLoadMap : {
    ## The ID of the mesh from which the drag load map was computed.
    mesh_id : Mesh.TriangleMeshID.TriangleMeshID,
    ## The drag coefficient of the body.
    drag_coefficient : F64,
}

## Adds a value of the [DragLoadMap] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DragLoadMap -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DragLoadMap] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DragLoadMap) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DragLoadMap.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DragLoadMap -> List U8
write_packet = |bytes, val|
    type_id = 2715277257896951626
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DragLoadMap -> List U8
write_multi_packet = |bytes, vals|
    type_id = 2715277257896951626
    size = 16
    alignment = 8
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

## Serializes a value of [DragLoadMap] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DragLoadMap -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Mesh.TriangleMeshID.write_bytes(value.mesh_id)
    |> Builtin.write_bytes_f64(value.drag_coefficient)

## Deserializes a value of [DragLoadMap] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DragLoadMap _
from_bytes = |bytes|
    Ok(
        {
            mesh_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Mesh.TriangleMeshID.from_bytes?,
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
