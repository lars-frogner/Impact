# Hash: e57b390a0911fe91db023e0631b6976e52c03b738f62066746c5b187a1e3c7b7
# Generated: 2025-09-20T12:42:13+00:00
# Rust type: impact_mesh::setup::ConeMesh
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    ConeMesh,
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

## A mesh consisting of an upward-pointing cone with the bottom centered on
## the origin.
ConeMesh : {
    ## The length of the cone.
    length : F32,
    ## The maximum diameter of the cone.
    max_diameter : F32,
    ## The number of vertices used for representing a circular cross-section of
    ## the cone.
    n_circumference_vertices : U32,
}

## Defines a cone mesh with the given length, maximum diameter and number
## of circumeference vertices.
new : F32, F32, U32 -> ConeMesh
new = |length, max_diameter, n_circumference_vertices|
    { length, max_diameter, n_circumference_vertices }

## Defines a cone mesh with the given length, maximum diameter and number
## of circumeference vertices.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, F32, F32, U32 -> Entity.ComponentData
add_new = |entity_data, length, max_diameter, n_circumference_vertices|
    add(entity_data, new(length, max_diameter, n_circumference_vertices))

## Defines a cone mesh with the given length, maximum diameter and number
## of circumeference vertices.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (U32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, length, max_diameter, n_circumference_vertices|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            length, max_diameter, n_circumference_vertices,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [ConeMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ConeMesh -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConeMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ConeMesh) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ConeMesh.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ConeMesh -> List U8
write_packet = |bytes, val|
    type_id = 18230576048737228968
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConeMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 18230576048737228968
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

## Serializes a value of [ConeMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConeMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Builtin.write_bytes_f32(value.length)
    |> Builtin.write_bytes_f32(value.max_diameter)
    |> Builtin.write_bytes_u32(value.n_circumference_vertices)

## Deserializes a value of [ConeMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConeMesh _
from_bytes = |bytes|
    Ok(
        {
            length: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            max_diameter: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
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
