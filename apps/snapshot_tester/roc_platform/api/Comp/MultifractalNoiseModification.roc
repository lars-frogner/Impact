# Hash: e1173b74240dc05e0d659655e1e6ade7767c89cfa362ceabd23636f3be681512
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact::voxel::components::MultifractalNoiseModificationComp
# Type category: Component
# Commit: 189570ab (dirty)
module [
    MultifractalNoiseModification,
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

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose voxel signed distance field should be perturbed by
## multifractal noise.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
MultifractalNoiseModification : {
    octaves : NativeNum.Usize,
    frequency : F64,
    lacunarity : F64,
    persistence : F64,
    amplitude : F64,
    seed : U64,
}

new : NativeNum.Usize, F64, F64, F64, F64, U64 -> MultifractalNoiseModification
new = |octaves, frequency, lacunarity, persistence, amplitude, seed|
    {
        octaves,
        frequency,
        lacunarity,
        persistence,
        amplitude,
        seed,
    }

add_new : Entity.Data, NativeNum.Usize, F64, F64, F64, F64, U64 -> Entity.Data
add_new = |entity_data, octaves, frequency, lacunarity, persistence, amplitude, seed|
    add(entity_data, new(octaves, frequency, lacunarity, persistence, amplitude, seed))

## Adds a value of the [MultifractalNoiseModification] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MultifractalNoiseModification -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MultifractalNoiseModification] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (MultifractalNoiseModification) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MultifractalNoiseModification.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MultifractalNoiseModification -> List U8
write_packet = |bytes, val|
    type_id = 16681079135556438596
    size = 48
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MultifractalNoiseModification -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16681079135556438596
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

## Serializes a value of [MultifractalNoiseModification] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MultifractalNoiseModification -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> NativeNum.write_bytes_usize(value.octaves)
    |> Builtin.write_bytes_f64(value.frequency)
    |> Builtin.write_bytes_f64(value.lacunarity)
    |> Builtin.write_bytes_f64(value.persistence)
    |> Builtin.write_bytes_f64(value.amplitude)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [MultifractalNoiseModification] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MultifractalNoiseModification _
from_bytes = |bytes|
    Ok(
        {
            octaves: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
            frequency: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            lacunarity: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            persistence: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
            amplitude: bytes |> List.sublist({ start: 32, len: 8 }) |> Builtin.from_bytes_f64?,
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
