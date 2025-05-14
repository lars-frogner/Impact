# Hash: 3c6bb95e4d227060db3ee2880365c9c214eb11698753911e2519dff9cedd7654
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::SceneGraphParentNodeComp
# Type category: Component
# Commit: d505d37
module [
    SceneGraphParentNode,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Scene.GroupNodeID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have a
## parent group node in the [`SceneGraph`](crate::scene::SceneGraph).
SceneGraphParentNode : {
    id : Scene.GroupNodeID.GroupNodeID,
}

## Creates a new component representing the parent
## [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
new : Scene.GroupNodeID.GroupNodeID -> SceneGraphParentNode
new = |parent_node_id|
    { id: parent_node_id }

## Creates a new component representing the parent
## [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Scene.GroupNodeID.GroupNodeID -> Entity.Data
add_new = |data, parent_node_id|
    add(data, new(parent_node_id))

## Adds a value of the [SceneGraphParentNode] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphParentNode -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SceneGraphParentNode] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SceneGraphParentNode -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SceneGraphParentNode -> List U8
write_packet = |bytes, value|
    type_id = 7003730196458964938
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SceneGraphParentNode -> List U8
write_multi_packet = |bytes, values|
    type_id = 7003730196458964938
    size = 16
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

## Serializes a value of [SceneGraphParentNode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneGraphParentNode -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Scene.GroupNodeID.write_bytes(value.id)

## Deserializes a value of [SceneGraphParentNode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneGraphParentNode _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 16 }) |> Scene.GroupNodeID.from_bytes?,
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
