# Hash: 1f9dc6b30eb3c6dbd7f664e3361a8f04fa28fc4c6bfb84ca4aa6fa5b69140acf
# Generated: 2025-07-15T17:32:17+00:00
# Rust type: impact_scene::setup::Parent
# Type category: Component
# Commit: 1fbb6f6b (dirty)
module [
    Parent,
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

## A parent entity.
##
## This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
## purpose is to aid in constructing a `SceneGraphParentNodeHandle` component
## for an entity. It is therefore not kept after entity creation.
Parent : {
    entity_id : Entity.Id,
}

new : Entity.Id -> Parent
new = |parent|
    { entity_id: parent }

add_new : Entity.Data, Entity.Id -> Entity.Data
add_new = |entity_data, parent|
    add(entity_data, new(parent))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Entity.Id) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, parent|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            parent,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [Parent] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Parent -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [Parent] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (Parent) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in Parent.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, Parent -> List U8
write_packet = |bytes, val|
    type_id = 1586825014089495529
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List Parent -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1586825014089495529
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

## Serializes a value of [Parent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Parent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Entity.write_bytes_id(value.entity_id)

## Deserializes a value of [Parent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Parent _
from_bytes = |bytes|
    Ok(
        {
            entity_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
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
