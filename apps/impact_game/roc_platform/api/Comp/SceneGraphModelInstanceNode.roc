# Hash: c0d40bc4fc46c51de153266034972cd79033bce3297141449c1238a1aeb01bd5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::SceneGraphModelInstanceNodeComp
# Type category: Component
# Commit: d505d37
module [
    SceneGraphModelInstanceNode,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Scene.ModelInstanceNodeID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have a
## model instance node in the [`SceneGraph`](crate::scene::SceneGraph).
SceneGraphModelInstanceNode : {
    ## The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    ## representing the entity.
    id : Scene.ModelInstanceNodeID.ModelInstanceNodeID,
}

## Creates a new component representing a
## [`SceneGraph`](crate::scene::SceneGraph) model instance node with the
## given ID.
new : Scene.ModelInstanceNodeID.ModelInstanceNodeID -> SceneGraphModelInstanceNode
new = |node_id|
    { id: node_id }

## Creates a new component representing a
## [`SceneGraph`](crate::scene::SceneGraph) model instance node with the
## given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Scene.ModelInstanceNodeID.ModelInstanceNodeID -> Entity.Data
add_new = |data, node_id|
    add(data, new(node_id))

## Adds a value of the [SceneGraphModelInstanceNode] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphModelInstanceNode -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SceneGraphModelInstanceNode] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SceneGraphModelInstanceNode -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SceneGraphModelInstanceNode -> List U8
write_packet = |bytes, value|
    type_id = 16196741923898652498
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SceneGraphModelInstanceNode -> List U8
write_multi_packet = |bytes, values|
    type_id = 16196741923898652498
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

## Serializes a value of [SceneGraphModelInstanceNode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneGraphModelInstanceNode -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Scene.ModelInstanceNodeID.write_bytes(value.id)

## Deserializes a value of [SceneGraphModelInstanceNode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneGraphModelInstanceNode _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 16 }) |> Scene.ModelInstanceNodeID.from_bytes?,
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
