# Hash: b88def55442d8c0d
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_voxel::setup::GradientNoiseVoxelTypes
# Type category: Component
module [
    GradientNoiseVoxelTypes,
    voxel_type_array_size,
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
import core.Hashing
import core.NativeNum

## A set of voxel types distributed according to a gradient noise pattern.
GradientNoiseVoxelTypes : {
    n_voxel_types : U32,
    voxel_type_name_hashes : List Hashing.Hash32,
    noise_frequency : F32,
    voxel_type_frequency : F32,
    seed : U32,
}

voxel_type_array_size : NativeNum.Usize
voxel_type_array_size = 256

new : List Str, F32, F32, U32 -> GradientNoiseVoxelTypes
new = |voxel_type_names, noise_frequency, voxel_type_frequency, seed|
    n_voxel_types = List.len(voxel_type_names)
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect n_voxel_types > 0
    # expect n_voxel_types <= voxel_type_array_size
    unpadded_voxel_type_name_hashes = voxel_type_names |> List.map(Hashing.hash_str_32)
    padding_len = voxel_type_array_size - n_voxel_types
    voxel_type_name_hashes = List.concat(
        unpadded_voxel_type_name_hashes,
        List.repeat(Hashing.hash_str_32(""), padding_len),
    )
    {
        n_voxel_types: Num.to_u32(n_voxel_types),
        voxel_type_name_hashes,
        noise_frequency,
        voxel_type_frequency,
        seed,
    }

add_new : Entity.ComponentData, List Str, F32, F32, U32 -> Entity.ComponentData
add_new = |entity_data, voxel_type_names, noise_frequency, voxel_type_frequency, seed|
    add(entity_data, new(voxel_type_names, noise_frequency, voxel_type_frequency, seed))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (List Str), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (U32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, voxel_type_names, noise_frequency, voxel_type_frequency, seed|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            voxel_type_names, noise_frequency, voxel_type_frequency, seed,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [GradientNoiseVoxelTypes] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, GradientNoiseVoxelTypes -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [GradientNoiseVoxelTypes] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (GradientNoiseVoxelTypes) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in GradientNoiseVoxelTypes.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, GradientNoiseVoxelTypes -> List U8
write_packet = |bytes, val|
    type_id = 14805917472837037976
    size = 1040
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List GradientNoiseVoxelTypes -> List U8
write_multi_packet = |bytes, vals|
    type_id = 14805917472837037976
    size = 1040
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

## Serializes a value of [GradientNoiseVoxelTypes] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GradientNoiseVoxelTypes -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(1040)
    |> Builtin.write_bytes_u32(value.n_voxel_types)
    |> (|bts, values| values |> List.walk(bts, |b, val| b |> Hashing.write_bytes_hash_32(val)))(value.voxel_type_name_hashes)
    |> Builtin.write_bytes_f32(value.noise_frequency)
    |> Builtin.write_bytes_f32(value.voxel_type_frequency)
    |> Builtin.write_bytes_u32(value.seed)

## Deserializes a value of [GradientNoiseVoxelTypes] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GradientNoiseVoxelTypes _
from_bytes = |bytes|
    Ok(
        {
            n_voxel_types: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
            voxel_type_name_hashes: bytes
            |> List.sublist({ start: 4, len: 1024 })
            |> List.chunks_of(4)
            |> List.map_try(|bts| Hashing.from_bytes_hash_32(bts))?,
            noise_frequency: bytes |> List.sublist({ start: 1028, len: 4 }) |> Builtin.from_bytes_f32?,
            voxel_type_frequency: bytes |> List.sublist({ start: 1032, len: 4 }) |> Builtin.from_bytes_f32?,
            seed: bytes |> List.sublist({ start: 1036, len: 4 }) |> Builtin.from_bytes_u32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 1040 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
