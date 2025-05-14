# Hash: 7ac117df62e1dcc4168ecbcfc90c53017bde03e9a6853c3c9469e3121c653589
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::MultiscaleSphereModificationComp
# Type category: Component
# Commit: d505d37
module [
    MultiscaleSphereModification,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.NativeNum

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose voxel signed distance field should be modified by unions
## with multiscale sphere grid (<https://iquilezles.org/articles/fbmsdf>/).
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
MultiscaleSphereModification : {
    octaves : NativeNum.Usize,
    max_scale : F64,
    persistence : F64,
    inflation : F64,
    smoothness : F64,
    seed : U64,
}

new : NativeNum.Usize, F64, F64, F64, F64, U64 -> MultiscaleSphereModification
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
add_new = |data, octaves, max_scale, persistence, inflation, smoothness, seed|
    add(data, new(octaves, max_scale, persistence, inflation, smoothness, seed))

## Adds a value of the [MultiscaleSphereModification] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MultiscaleSphereModification -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [MultiscaleSphereModification] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List MultiscaleSphereModification -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, MultiscaleSphereModification -> List U8
write_packet = |bytes, value|
    type_id = 12613102059108968942
    size = 48
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List MultiscaleSphereModification -> List U8
write_multi_packet = |bytes, values|
    type_id = 12613102059108968942
    size = 48
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

## Serializes a value of [MultiscaleSphereModification] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MultiscaleSphereModification -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> NativeNum.write_bytes_usize(value.octaves)
    |> Builtin.write_bytes_f64(value.max_scale)
    |> Builtin.write_bytes_f64(value.persistence)
    |> Builtin.write_bytes_f64(value.inflation)
    |> Builtin.write_bytes_f64(value.smoothness)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [MultiscaleSphereModification] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MultiscaleSphereModification _
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
