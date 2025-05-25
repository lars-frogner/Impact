# Hash: 331e27d3c0aee29cc9ab3e002558a398d3a099975296c0e2e7b2c126823d2e19
# Generated: 2025-05-24T10:01:42+00:00
# Rust type: impact::voxel::components::VoxelGradientNoisePatternComp
# Type category: Component
# Commit: 31f3514 (dirty)
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

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities comprised of voxels in a gradient noise pattern.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
VoxelGradientNoisePattern : {
    ## The extent of a single voxel.
    voxel_extent : F64,
    ## The maximum number of voxels in the x-direction.
    extent_x : F64,
    ## The maximum number of voxels in the y-direction.
    extent_y : F64,
    ## The maximum number of voxels in the z-direction.
    extent_z : F64,
    ## The spatial frequency of the noise pattern.
    noise_frequency : F64,
    ## The threshold noise value for generating a voxel.
    noise_threshold : F64,
    ## The seed for the noise pattern.
    seed : U64,
}

## Creates a new component for a gradient noise voxel pattern with the
## given maximum number of voxels in each direction, spatial noise
## frequency, noise threshold and seed.
new : F64, F64, F64, F64, F64, F64, U64 -> VoxelGradientNoisePattern
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

## Creates a new component for a gradient noise voxel pattern with the
## given maximum number of voxels in each direction, spatial noise
## frequency, noise threshold and seed.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, F64, F64, F64, F64, F64, U64 -> Entity.Data
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
    type_id = 4731168943360092776
    size = 56
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelGradientNoisePattern -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4731168943360092776
    size = 56
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

## Serializes a value of [VoxelGradientNoisePattern] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelGradientNoisePattern -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Builtin.write_bytes_f64(value.voxel_extent)
    |> Builtin.write_bytes_f64(value.extent_x)
    |> Builtin.write_bytes_f64(value.extent_y)
    |> Builtin.write_bytes_f64(value.extent_z)
    |> Builtin.write_bytes_f64(value.noise_frequency)
    |> Builtin.write_bytes_f64(value.noise_threshold)
    |> Builtin.write_bytes_u64(value.seed)

## Deserializes a value of [VoxelGradientNoisePattern] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelGradientNoisePattern _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_x: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_y: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_z: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
            noise_frequency: bytes |> List.sublist({ start: 32, len: 8 }) |> Builtin.from_bytes_f64?,
            noise_threshold: bytes |> List.sublist({ start: 40, len: 8 }) |> Builtin.from_bytes_f64?,
            seed: bytes |> List.sublist({ start: 48, len: 8 }) |> Builtin.from_bytes_u64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
