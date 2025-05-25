# Hash: e60225d53c264fea45aa057cef5ddfcb9149d3f9a78015a1dd6be510a05b846a
# Generated: 2025-05-23T21:48:57+00:00
# Rust type: impact::scene::components::SceneGraphParentNodeComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    SceneGraphParentNode,
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
add_new = |entity_data, parent_node_id|
    add(entity_data, new(parent_node_id))

## Creates a new component representing the parent
## [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Scene.GroupNodeID.GroupNodeID) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, parent_node_id|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            parent_node_id,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SceneGraphParentNode] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphParentNode -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SceneGraphParentNode] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SceneGraphParentNode) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SceneGraphParentNode.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SceneGraphParentNode -> List U8
write_packet = |bytes, val|
    type_id = 7003730196458964938
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SceneGraphParentNode -> List U8
write_multi_packet = |bytes, vals|
    type_id = 7003730196458964938
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
