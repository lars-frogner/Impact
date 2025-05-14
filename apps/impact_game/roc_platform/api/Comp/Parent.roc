# Hash: a5d9cbb891ab374ca06bd73e4600616f7f836ded3c24d4f8d50dc50ce83ad7e0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::ParentComp
# Type category: Component
# Commit: d505d37
module [
    Parent,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a parent entity.
##
## The purpose of this component is to aid in constructing a
## [`SceneGraphParentNodeComp`] for the entity. It is therefore not kept after
## entity creation.
Parent : {
    entity : Entity.Id,
}

## Creates a new component representing a direct child of the given
## [`Entity`].
new : Entity.Id -> Parent
new = |parent|
    { entity: parent }

## Creates a new component representing a direct child of the given
## [`Entity`].
## Adds the component to the given entity's data.
add_new : Entity.Data, Entity.Id -> Entity.Data
add_new = |data, parent|
    add(data, new(parent))

## Adds a value of the [Parent] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Parent -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Parent] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Parent -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Parent -> List U8
write_packet = |bytes, value|
    type_id = 6272559603799074398
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Parent -> List U8
write_multi_packet = |bytes, values|
    type_id = 6272559603799074398
    size = 8
    alignment = 4
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

## Serializes a value of [Parent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Parent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Entity.write_bytes_id(value.entity)

## Deserializes a value of [Parent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Parent _
from_bytes = |bytes|
    Ok(
        {
            entity: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
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
