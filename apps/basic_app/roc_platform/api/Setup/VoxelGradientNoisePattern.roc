# Hash: d91edfa31c8932c5f848e46eeeeb2927f78b2f1e13326ab5e3ec1e4c00e1e032
# Generated: 2025-09-14T20:34:17+00:00
# Rust type: impact_voxel::setup::VoxelGradientNoisePattern
# Type category: Component
# Commit: aa40a05d (dirty)
module [
    VoxelGradientNoisePattern,
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

## An object made of voxels in a gradient noise pattern.
VoxelGradientNoisePattern : {
    ## The extent of a single voxel.
    voxel_extent : F32,
    ## The maximum number of voxels in the x-direction.
    extent_x : F32,
    ## The maximum number of voxels in the y-direction.
    extent_y : F32,
    ## The maximum number of voxels in the z-direction.
    extent_z : F32,
    ## The spatial frequency of the noise pattern.
    noise_frequency : F32,
    ## The threshold noise value for generating a voxel.
    noise_threshold : F32,
    ## The seed for the noise pattern.
    seed : U32,
}

## Defines a gradient noise voxel pattern with the given maximum number of
## voxels in each direction, spatial noise frequency, noise threshold and
## seed.
new : F32, F32, F32, F32, F32, F32, U32 -> VoxelGradientNoisePattern
new = |voxel_extent, extent_x, extent_y, extent_z, noise_frequency, noise_threshold, seed|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect extent_x >= 0.0
    # expect extent_y >= 0.0
    # expect extent_z >= 0.0
    {
        voxel_extent,
        extent_x,
        extent_y,
        extent_z,
        noise_frequency,
        noise_threshold,
        seed,
    }

## Defines a gradient noise voxel pattern with the given maximum number of
## voxels in each direction, spatial noise frequency, noise threshold and
## seed.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32, F32, F32, F32, F32, U32 -> Entity.Data
add_new = |entity_data, voxel_extent, extent_x, extent_y, extent_z, noise_frequency, noise_threshold, seed|
    add(entity_data, new(voxel_extent, extent_x, extent_y, extent_z, noise_frequency, noise_threshold, seed))

## Adds a value of the [VoxelGradientNoisePattern] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelGradientNoisePattern -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelGradientNoisePattern] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelGradientNoisePattern) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelGradientNoisePattern.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelGradientNoisePattern -> List U8
write_packet = |bytes, val|
    type_id = 10360059559405253247
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelGradientNoisePattern -> List U8
write_multi_packet = |bytes, vals|
    type_id = 10360059559405253247
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

## Serializes a value of [VoxelGradientNoisePattern] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelGradientNoisePattern -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Builtin.write_bytes_f32(value.voxel_extent)
    |> Builtin.write_bytes_f32(value.extent_x)
    |> Builtin.write_bytes_f32(value.extent_y)
    |> Builtin.write_bytes_f32(value.extent_z)
    |> Builtin.write_bytes_f32(value.noise_frequency)
    |> Builtin.write_bytes_f32(value.noise_threshold)
    |> Builtin.write_bytes_u32(value.seed)

## Deserializes a value of [VoxelGradientNoisePattern] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelGradientNoisePattern _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_x: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_y: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_z: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            noise_frequency: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
            noise_threshold: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
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
