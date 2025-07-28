# Hash: 57519c3bfe84ec3feaa682832e0deb14808632891ff7b3f29481aa956b12bd35
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_mesh::setup::CircularFrustumMesh
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    CircularFrustumMesh,
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

## A mesh consisting of a vertical circular frustum with the bottom
## centered on the origin.
CircularFrustumMesh : {
    ## The length of the frustum.
    length : F32,
    ## The bottom diameter of the frustum.
    bottom_diameter : F32,
    ## The top diameter of the frustum.
    top_diameter : F32,
    ## The number of vertices used for representing a circular cross-section of
    ## the frustum.
    n_circumference_vertices : U32,
}

## Defines a circular frustum mesh with the given length, bottom and top
## diameter and number of circumeference vertices.
new : F32, F32, F32, U32 -> CircularFrustumMesh
new = |length, bottom_diameter, top_diameter, n_circumference_vertices|
    {
        length,
        bottom_diameter,
        top_diameter,
        n_circumference_vertices,
    }

## Defines a circular frustum mesh with the given length, bottom and top
## diameter and number of circumeference vertices.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32, F32, U32 -> Entity.Data
add_new = |entity_data, length, bottom_diameter, top_diameter, n_circumference_vertices|
    add(entity_data, new(length, bottom_diameter, top_diameter, n_circumference_vertices))

## Defines a circular frustum mesh with the given length, bottom and top
## diameter and number of circumeference vertices.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (U32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, length, bottom_diameter, top_diameter, n_circumference_vertices|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            length, bottom_diameter, top_diameter, n_circumference_vertices,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [CircularFrustumMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, CircularFrustumMesh -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [CircularFrustumMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (CircularFrustumMesh) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in CircularFrustumMesh.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, CircularFrustumMesh -> List U8
write_packet = |bytes, val|
    type_id = 12335576792637945962
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List CircularFrustumMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 12335576792637945962
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

## Serializes a value of [CircularFrustumMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CircularFrustumMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(value.length)
    |> Builtin.write_bytes_f32(value.bottom_diameter)
    |> Builtin.write_bytes_f32(value.top_diameter)
    |> Builtin.write_bytes_u32(value.n_circumference_vertices)

## Deserializes a value of [CircularFrustumMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularFrustumMesh _
from_bytes = |bytes|
    Ok(
        {
            length: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            bottom_diameter: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            top_diameter: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            n_circumference_vertices: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_u32?,
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
