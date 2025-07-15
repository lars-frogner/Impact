# Hash: cdec801d35ecf8114773015ece13c07d6c5c76c2471698c53290f29a4b4c072b
# Generated: 2025-07-15T22:31:15+00:00
# Rust type: impact_voxel::setup::VoxelSphere
# Type category: Component
# Commit: 07de5ae9 (dirty)
module [
    VoxelSphere,
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

## An object made of voxels in a spherical configuration.
VoxelSphere : {
    ## The extent of a single voxel.
    voxel_extent : F64,
    ## The number of voxels along the radius of the sphere.
    radius : F64,
}

## Defines a sphere with the given voxel extent and number of voxels across
## its radius.
##
## # Panics
## - If the voxel extent is negative.
## - If the radius zero or negative.
new : F64, F64 -> VoxelSphere
new = |voxel_extent, radius|
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius >= 0.0
    {
        voxel_extent,
        radius,
    }

## Defines a sphere with the given voxel extent and number of voxels across
## its radius.
##
## # Panics
## - If the voxel extent is negative.
## - If the radius zero or negative.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, F64 -> Entity.Data
add_new = |entity_data, voxel_extent, radius|
    add(entity_data, new(voxel_extent, radius))

## Defines a sphere with the given voxel extent and number of voxels across
## its radius.
##
## # Panics
## - If the voxel extent is negative.
## - If the radius zero or negative.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, voxel_extent, radius|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            voxel_extent, radius,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [VoxelSphere] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelSphere -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelSphere] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelSphere) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelSphere.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelSphere -> List U8
write_packet = |bytes, val|
    type_id = 10802133982048297802
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelSphere -> List U8
write_multi_packet = |bytes, vals|
    type_id = 10802133982048297802
    size = 16
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

## Serializes a value of [VoxelSphere] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelSphere -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f64(value.voxel_extent)
    |> Builtin.write_bytes_f64(value.radius)

## Deserializes a value of [VoxelSphere] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelSphere _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            radius: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
