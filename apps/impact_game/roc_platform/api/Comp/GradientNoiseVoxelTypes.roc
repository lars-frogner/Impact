# Hash: 124fc362b83cde5845a2e4a6454ed3b57b16535d2d1f105a311cb6e02ab6af49
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::GradientNoiseVoxelTypesComp
# Type category: Component
# Commit: d505d37
module [
    GradientNoiseVoxelTypes,
    voxel_type_array_size,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Hashing
import core.NativeNum

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose voxel types are distributed according to a gradient noise
## pattern.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
GradientNoiseVoxelTypes : {
    n_voxel_types : NativeNum.Usize,
    voxel_type_name_hashes : List Hashing.Hash32,
    noise_frequency : F64,
    voxel_type_frequency : F64,
    seed : U64,
}

voxel_type_array_size : NativeNum.Usize
voxel_type_array_size = 256

new : List Str, F64, F64, U64 -> GradientNoiseVoxelTypes
new = |voxel_type_names, noise_frequency, voxel_type_frequency, seed|
    n_voxel_types = List.len(voxel_type_names)
    expect n_voxel_types > 0
    expect n_voxel_types <= voxel_type_array_size
    voxel_type_name_hashes = voxel_type_names |> List.map(Hashing.hash_str_32)
    {
        n_voxel_types,
        voxel_type_name_hashes,
        noise_frequency,
        voxel_type_frequency,
        seed,
    }

add_new : Entity.Data, List Str, F64, F64, U64 -> Entity.Data
add_new = |data, voxel_type_names, noise_frequency, voxel_type_frequency, seed|
    add(data, new(voxel_type_names, noise_frequency, voxel_type_frequency, seed))

## Adds a value of the [GradientNoiseVoxelTypes] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, GradientNoiseVoxelTypes -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [GradientNoiseVoxelTypes] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List GradientNoiseVoxelTypes -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, GradientNoiseVoxelTypes -> List U8
write_packet = |bytes, value|
    type_id = 865311061570754875
    size = 1056
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List GradientNoiseVoxelTypes -> List U8
write_multi_packet = |bytes, values|
    type_id = 865311061570754875
    size = 1056
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

## Serializes a value of [GradientNoiseVoxelTypes] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GradientNoiseVoxelTypes -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1056)
    |> NativeNum.write_bytes_usize(value.n_voxel_types)
    |> (|bts, values| values |> List.walk(bts, |b, val| b |> Hashing.write_bytes_hash_32(val)))(value.voxel_type_name_hashes)
    |> Builtin.write_bytes_f64(value.noise_frequency)
    |> Builtin.write_bytes_f64(value.voxel_type_frequency)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [GradientNoiseVoxelTypes] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GradientNoiseVoxelTypes _
from_bytes = |bytes|
    Ok(
        {
            n_voxel_types: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
            voxel_type_name_hashes: bytes
            |> List.sublist({ start: 8, len: 1024 })
            |> List.chunks_of(4)
            |> List.map_try(|bts| Hashing.from_bytes_hash_32(bts))?,
            noise_frequency: bytes |> List.sublist({ start: 1032, len: 8 }) |> Builtin.from_bytes_f64?,
            voxel_type_frequency: bytes |> List.sublist({ start: 1040, len: 8 }) |> Builtin.from_bytes_f64?,
            seed: bytes |> List.sublist({ start: 1048, len: 8 }) |> Builtin.from_bytes_u64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1056 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
