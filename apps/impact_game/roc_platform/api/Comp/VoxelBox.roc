# Hash: 4eb00193bc3cdd8f551e6145e66fb71a27aedf639cf524bbefba7806fa5e7b89
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::VoxelBoxComp
# Type category: Component
# Commit: d505d37
module [
    VoxelBox,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities comprised of voxels in a box configuration.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
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

## Creates a new component for a box with the given voxel extent
## and number of voxels in each direction.
##
## # Panics
## - If the voxel extent is negative.
## - If either of the extents is zero or negative.
new : F64, F64, F64, F64 -> VoxelBox
new = |voxel_extent, extent_x, extent_y, extent_z|
    expect voxel_extent > 0.0
    expect extent_x >= 0.0
    expect extent_y >= 0.0
    expect extent_z >= 0.0
    {
        voxel_extent,
        extent_x,
        extent_y,
        extent_z,
    }

## Creates a new component for a box with the given voxel extent
## and number of voxels in each direction.
##
## # Panics
## - If the voxel extent is negative.
## - If either of the extents is zero or negative.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, F64, F64, F64 -> Entity.Data
add_new = |data, voxel_extent, extent_x, extent_y, extent_z|
    add(data, new(voxel_extent, extent_x, extent_y, extent_z))

## Adds a value of the [VoxelBox] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelBox -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [VoxelBox] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List VoxelBox -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, VoxelBox -> List U8
write_packet = |bytes, value|
    type_id = 4357395274577469411
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List VoxelBox -> List U8
write_multi_packet = |bytes, values|
    type_id = 4357395274577469411
    size = 32
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
