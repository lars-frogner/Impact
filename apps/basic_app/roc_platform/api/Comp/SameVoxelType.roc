# Hash: 9f1645f47f638539e617ec2653e85103382cfc53e3ad1597d1212a36e4a3df2b
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact::voxel::components::SameVoxelTypeComp
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    SameVoxelType,
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
add_new = |entity_data, voxel_type|
    add(entity_data, new(voxel_type))

## Creates a new component for an entity comprised of voxels of the given
## type.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Voxel.VoxelType.VoxelType) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, voxel_type|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            voxel_type,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SameVoxelType] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SameVoxelType -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SameVoxelType] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SameVoxelType) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SameVoxelType.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SameVoxelType -> List U8
write_packet = |bytes, val|
    type_id = 4426266743824765082
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SameVoxelType -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4426266743824765082
    size = 8
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
