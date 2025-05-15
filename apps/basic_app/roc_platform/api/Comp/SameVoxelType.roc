# Hash: bccff149e34742d11afa4bb6acb4c84e609c3f40b1eb2573b6958b304201d8bd
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::voxel::components::SameVoxelTypeComp
# Type category: Component
# Commit: d505d37
module [
    SameVoxelType,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Voxel.VoxelType
import core.Builtin
import core.NativeNum

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose voxel type is the same everywhere.
##
## The purpose of this component is to aid in constructing a
## [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
## creation.
SameVoxelType : {
    ## The index of the voxel type.
    voxel_type_idx : NativeNum.Usize,
}

## Creates a new component for an entity comprised of voxels of the given
## type.
new : Voxel.VoxelType.VoxelType -> SameVoxelType
new = |voxel_type|
    { voxel_type_idx: NativeNum.to_usize(voxel_type) }

## Creates a new component for an entity comprised of voxels of the given
## type.
## Adds the component to the given entity's data.
add_new : Entity.Data, Voxel.VoxelType.VoxelType -> Entity.Data
add_new = |data, voxel_type|
    add(data, new(voxel_type))

## Adds a value of the [SameVoxelType] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SameVoxelType -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SameVoxelType] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SameVoxelType -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SameVoxelType -> List U8
write_packet = |bytes, value|
    type_id = 4426266743824765082
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SameVoxelType -> List U8
write_multi_packet = |bytes, values|
    type_id = 4426266743824765082
    size = 8
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

## Serializes a value of [SameVoxelType] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SameVoxelType -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> NativeNum.write_bytes_usize(value.voxel_type_idx)

## Deserializes a value of [SameVoxelType] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SameVoxelType _
from_bytes = |bytes|
    Ok(
        {
            voxel_type_idx: bytes |> List.sublist({ start: 0, len: 8 }) |> NativeNum.from_bytes_usize?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
