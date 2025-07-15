# Hash: 3bd089b9225deabe85db6e90d70f7bf08338036ce4aec5ec735f09cae911af08
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact::physics::collision::setup::VoxelCollidable
# Type category: Component
# Commit: 189570ab (dirty)
module [
    VoxelCollidable,
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
import Physics.CollidableKind
import Physics.ContactResponseParameters
import core.Builtin

## A voxel object-based collidable.
##
## This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
## purpose is to aid in constructing a `CollidableID` component for an entity.
## It is therefore not kept after entity creation.
VoxelCollidable : {
    kind : U64,
    response_params : Physics.ContactResponseParameters.ContactResponseParameters,
}

new : Physics.CollidableKind.CollidableKind, Physics.ContactResponseParameters.ContactResponseParameters -> VoxelCollidable
new = |kind, response_params|
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        response_params,
    }

add_new : Entity.Data, Physics.CollidableKind.CollidableKind, Physics.ContactResponseParameters.ContactResponseParameters -> Entity.Data
add_new = |entity_data, kind, response_params|
    add(entity_data, new(kind, response_params))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Physics.CollidableKind.CollidableKind), Entity.Arg.Broadcasted (Physics.ContactResponseParameters.ContactResponseParameters) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, kind, response_params|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            kind, response_params,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [VoxelCollidable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, VoxelCollidable -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [VoxelCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (VoxelCollidable) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in VoxelCollidable.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, VoxelCollidable -> List U8
write_packet = |bytes, val|
    type_id = 11983096598995050472
    size = 32
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List VoxelCollidable -> List U8
write_multi_packet = |bytes, vals|
    type_id = 11983096598995050472
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

## Serializes a value of [VoxelCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, VoxelCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_u64(value.kind)
    |> Physics.ContactResponseParameters.write_bytes(value.response_params)

## Deserializes a value of [VoxelCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result VoxelCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
            response_params: bytes |> List.sublist({ start: 8, len: 24 }) |> Physics.ContactResponseParameters.from_bytes?,
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
