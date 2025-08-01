# Hash: a7b9bd5c82adb1f10194282bea29d6ac05d50fc9bd796520b097896fad6c5f7a
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_voxel::setup::VoxelBox
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    VoxelBox,
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

## An object made of voxels in a box configuration.
VoxelBox : {
    ## The extent of a single voxel.
    voxel_extent : F64,
    ## The number of voxels along the box in the x-direction.
    extent_x : F64,
    ## The number of voxels along the box in the y-direction.
    extent_y : F64,
    ## The number of voxels along the box in the z-direction.
    extent_z : F64,
}

## Defines a box with the given voxel extent and number of voxels in each
## direction.
##
## # Panics
## - If the voxel extent is negative.
## - If either of the extents is zero or negative.
new : F64, F64, F64, F64 -> VoxelBox
new = |voxel_extent, extent_x, extent_y, extent_z|
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
    }

## Defines a box with the given voxel extent and number of voxels in each
## direction.
##
## # Panics
## - If the voxel extent is negative.
## - If either of the extents is zero or negative.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, F64, F64, F64 -> Entity.Data
add_new = |entity_data, voxel_extent, extent_x, extent_y, extent_z|
    add(entity_data, new(voxel_extent, extent_x, extent_y, extent_z))

## Defines a box with the given voxel extent and number of voxels in each
## direction.
##
## # Panics
## - If the voxel extent is negative.
## - If either of the extents is zero or negative.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, voxel_extent, extent_x, extent_y, extent_z|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            voxel_extent, extent_x, extent_y, extent_z,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [VoxelBox] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelBox -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelBox] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelBox) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelBox.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelBox -> List U8
write_packet = |bytes, val|
    type_id = 11759487956506158112
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelBox -> List U8
write_multi_packet = |bytes, vals|
    type_id = 11759487956506158112
    size = 32
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

## Serializes a value of [VoxelBox] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelBox -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_f64(value.voxel_extent)
    |> Builtin.write_bytes_f64(value.extent_x)
    |> Builtin.write_bytes_f64(value.extent_y)
    |> Builtin.write_bytes_f64(value.extent_z)

## Deserializes a value of [VoxelBox] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelBox _
from_bytes = |bytes|
    Ok(
        {
            voxel_extent: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_x: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_y: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            extent_z: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
