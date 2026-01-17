# Hash: 9b085c686eb4450e
# Generated: 2026-01-17T13:05:19.313975987
# Rust type: impact_voxel::setup::GeneratedVoxelObject
# Type category: Component
module [
    GeneratedVoxelObject,
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
import Voxel.VoxelGeneratorID
import core.Builtin
import core.Hashing

## A generated voxel object.
GeneratedVoxelObject : {
    generator_id : Voxel.VoxelGeneratorID.VoxelGeneratorID,
    voxel_extent : F32,
    scale_factor : F32,
    seed : U64,
}

new : Str, F32, F32, U64 -> GeneratedVoxelObject
new = |generator_name, voxel_extent, scale_factor, seed|
    { generator_id: Hashing.hash_str_64(generator_name), voxel_extent, scale_factor, seed }

add_new : Entity.ComponentData, Str, F32, F32, U64 -> Entity.ComponentData
add_new = |entity_data, generator_name, voxel_extent, scale_factor, seed|
    add(entity_data, new(generator_name, voxel_extent, scale_factor, seed))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Str), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (U64) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, generator_name, voxel_extent, scale_factor, seed|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            generator_name, voxel_extent, scale_factor, seed,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [GeneratedVoxelObject] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, GeneratedVoxelObject -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [GeneratedVoxelObject] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (GeneratedVoxelObject) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in GeneratedVoxelObject.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, GeneratedVoxelObject -> List U8
write_packet = |bytes, val|
    type_id = 7102113392894755801
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List GeneratedVoxelObject -> List U8
write_multi_packet = |bytes, vals|
    type_id = 7102113392894755801
    size = 24
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

## Serializes a value of [GeneratedVoxelObject] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GeneratedVoxelObject -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Voxel.VoxelGeneratorID.write_bytes(value.generator_id)
    |> Builtin.write_bytes_f32(value.voxel_extent)
    |> Builtin.write_bytes_f32(value.scale_factor)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [GeneratedVoxelObject] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GeneratedVoxelObject _
from_bytes = |bytes|
    Ok(
        {
            generator_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Voxel.VoxelGeneratorID.from_bytes?,
            voxel_extent: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            scale_factor: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            seed: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_u64?,
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
