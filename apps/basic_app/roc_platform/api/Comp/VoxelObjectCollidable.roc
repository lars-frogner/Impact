# Hash: 3c11e3ebcc96390a1f1c82332790a714b92feb0805c0b480eb0591760a771c55
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::collision::components::VoxelObjectCollidableComp
# Type category: Component
# Commit: d505d37
module [
    VoxelObjectCollidable,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Physics.CollidableKind
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that use their voxel object as a collidable.
##
## The purpose of this component is to aid in constructing a [`CollidableComp`]
## for the entity. It is therefore not kept after entity creation.
VoxelObjectCollidable : {
    kind : U64,
}

new : Physics.CollidableKind.CollidableKind -> VoxelObjectCollidable
new = |kind|
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
    }

add_new : Entity.Data, Physics.CollidableKind.CollidableKind -> Entity.Data
add_new = |data, kind|
    add(data, new(kind))

## Adds a value of the [VoxelObjectCollidable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelObjectCollidable -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [VoxelObjectCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List VoxelObjectCollidable -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, VoxelObjectCollidable -> List U8
write_packet = |bytes, value|
    type_id = 13563106532577398459
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List VoxelObjectCollidable -> List U8
write_multi_packet = |bytes, values|
    type_id = 13563106532577398459
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

## Serializes a value of [VoxelObjectCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelObjectCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_u64(value.kind)

## Deserializes a value of [VoxelObjectCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelObjectCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
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
