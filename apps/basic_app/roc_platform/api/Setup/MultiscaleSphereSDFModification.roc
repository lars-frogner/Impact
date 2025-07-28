# Hash: 8394c9081c4b6b1534ee935b53380bdda4274ac36f11bc007a0d66471f11a04c
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_voxel::setup::MultiscaleSphereSDFModification
# Type category: Component
# Commit: 397d36d3 (dirty)
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
import core.NativeNum

## A modification to a voxel signed distance field based on unions with a
## multiscale sphere grid (<https://iquilezles.org/articles/fbmsdf>/).
MultiscaleSphereSDFModification : {
    octaves : NativeNum.Usize,
    max_scale : F64,
    persistence : F64,
    inflation : F64,
    smoothness : F64,
    seed : U64,
}

new : NativeNum.Usize, F64, F64, F64, F64, U64 -> MultiscaleSphereSDFModification
new = |octaves, max_scale, persistence, inflation, smoothness, seed|
    {
        octaves,
        max_scale,
        persistence,
        inflation,
        smoothness,
        seed,
    }

add_new : Entity.Data, NativeNum.Usize, F64, F64, F64, F64, U64 -> Entity.Data
add_new = |entity_data, octaves, max_scale, persistence, inflation, smoothness, seed|
    add(entity_data, new(octaves, max_scale, persistence, inflation, smoothness, seed))

## Adds a value of the [MultiscaleSphereSDFModification] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MultiscaleSphereSDFModification -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MultiscaleSphereSDFModification] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (MultiscaleSphereSDFModification) -> Result Entity.MultiData Str
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
    size = 48
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MultiscaleSphereSDFModification -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1066675320843307296
    size = 48
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

## Serializes a value of [MultiscaleSphereSDFModification] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MultiscaleSphereSDFModification -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> NativeNum.write_bytes_usize(value.octaves)
    |> Builtin.write_bytes_f64(value.max_scale)
    |> Builtin.write_bytes_f64(value.persistence)
    |> Builtin.write_bytes_f64(value.inflation)
    |> Builtin.write_bytes_f64(value.smoothness)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [MultiscaleSphereSDFModification] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MultiscaleSphereSDFModification _
from_bytes = |bytes|
    Ok(
        {
            octaves: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
            max_scale: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            persistence: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            inflation: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
            smoothness: bytes |> List.sublist({ start: 32, len: 8 }) |> Builtin.from_bytes_f64?,
            seed: bytes |> List.sublist({ start: 40, len: 8 }) |> Builtin.from_bytes_u64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 48 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
