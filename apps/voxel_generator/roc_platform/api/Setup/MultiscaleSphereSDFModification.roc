# Hash: 67001e6dbf37177ec380b3837d07d7b1ab3648ffc0b35173d808b790408785b5
# Generated: 2025-10-11T08:32:27+00:00
# Rust type: impact_voxel::setup::MultiscaleSphereSDFModification
# Type category: Component
# Commit: 8cb17139 (dirty)
module [
    MultiscaleSphereSDFModification,
    new,
    add_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## A modification to a voxel signed distance field based on unions with a
## multiscale sphere grid (<https://iquilezles.org/articles/fbmsdf>/).
MultiscaleSphereSDFModification : {
    octaves : U32,
    max_scale : F32,
    persistence : F32,
    inflation : F32,
    intersection_smoothness : F32,
    union_smoothness : F32,
    seed : U32,
}

new : U32, F32, F32, F32, F32, F32, U32 -> MultiscaleSphereSDFModification
new = |octaves, max_scale, persistence, inflation, intersection_smoothness, union_smoothness, seed|
    {
        octaves,
        max_scale,
        persistence,
        inflation,
        intersection_smoothness,
        union_smoothness,
        seed,
    }

add_new : Entity.ComponentData, U32, F32, F32, F32, F32, F32, U32 -> Entity.ComponentData
add_new = |entity_data, octaves, max_scale, persistence, inflation, intersection_smoothness, union_smoothness, seed|
    add(entity_data, new(octaves, max_scale, persistence, inflation, intersection_smoothness, union_smoothness, seed))

## Adds a value of the [MultiscaleSphereSDFModification] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, MultiscaleSphereSDFModification -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MultiscaleSphereSDFModification] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (MultiscaleSphereSDFModification) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MultiscaleSphereSDFModification.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MultiscaleSphereSDFModification -> List U8
write_packet = |bytes, val|
    type_id = 1066675320843307296
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MultiscaleSphereSDFModification -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1066675320843307296
    size = 28
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

## Serializes a value of [MultiscaleSphereSDFModification] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MultiscaleSphereSDFModification -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Builtin.write_bytes_u32(value.octaves)
    |> Builtin.write_bytes_f32(value.max_scale)
    |> Builtin.write_bytes_f32(value.persistence)
    |> Builtin.write_bytes_f32(value.inflation)
    |> Builtin.write_bytes_f32(value.intersection_smoothness)
    |> Builtin.write_bytes_f32(value.union_smoothness)
    |> Builtin.write_bytes_u32(value.seed)

## Deserializes a value of [MultiscaleSphereSDFModification] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MultiscaleSphereSDFModification _
from_bytes = |bytes|
    Ok(
        {
            octaves: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
            max_scale: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            persistence: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            inflation: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            intersection_smoothness: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
            union_smoothness: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
            seed: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_u32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 28 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
