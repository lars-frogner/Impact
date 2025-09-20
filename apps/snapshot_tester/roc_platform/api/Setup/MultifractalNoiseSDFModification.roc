# Hash: 50c89ef03dbbeaac76496a066f8535a907f5aac0b40e6116f375cd45c7a8fab2
# Generated: 2025-09-20T12:42:00+00:00
# Rust type: impact_voxel::setup::MultifractalNoiseSDFModification
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    MultifractalNoiseSDFModification,
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

## A modification of a voxel signed distance field based on multifractal
## noise.
MultifractalNoiseSDFModification : {
    octaves : U32,
    frequency : F32,
    lacunarity : F32,
    persistence : F32,
    amplitude : F32,
    seed : U32,
}

new : U32, F32, F32, F32, F32, U32 -> MultifractalNoiseSDFModification
new = |octaves, frequency, lacunarity, persistence, amplitude, seed|
    {
        octaves,
        frequency,
        lacunarity,
        persistence,
        amplitude,
        seed,
    }

add_new : Entity.ComponentData, U32, F32, F32, F32, F32, U32 -> Entity.ComponentData
add_new = |entity_data, octaves, frequency, lacunarity, persistence, amplitude, seed|
    add(entity_data, new(octaves, frequency, lacunarity, persistence, amplitude, seed))

## Adds a value of the [MultifractalNoiseSDFModification] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, MultifractalNoiseSDFModification -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MultifractalNoiseSDFModification] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (MultifractalNoiseSDFModification) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MultifractalNoiseSDFModification.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MultifractalNoiseSDFModification -> List U8
write_packet = |bytes, val|
    type_id = 5074781755057218298
    size = 24
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MultifractalNoiseSDFModification -> List U8
write_multi_packet = |bytes, vals|
    type_id = 5074781755057218298
    size = 24
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

## Serializes a value of [MultifractalNoiseSDFModification] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MultifractalNoiseSDFModification -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Builtin.write_bytes_u32(value.octaves)
    |> Builtin.write_bytes_f32(value.frequency)
    |> Builtin.write_bytes_f32(value.lacunarity)
    |> Builtin.write_bytes_f32(value.persistence)
    |> Builtin.write_bytes_f32(value.amplitude)
    |> Builtin.write_bytes_u32(value.seed)

## Deserializes a value of [MultifractalNoiseSDFModification] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MultifractalNoiseSDFModification _
from_bytes = |bytes|
    Ok(
        {
            octaves: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
            frequency: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            lacunarity: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            persistence: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            amplitude: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
            seed: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_u32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
