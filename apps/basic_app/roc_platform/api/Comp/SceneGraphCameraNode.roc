# Hash: 926ddc4039f9e0c65a6bbef35ab06035299bcf5a3c748b45363342854c643984
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::components::SceneGraphCameraNodeComp
# Type category: Component
# Commit: d505d37
module [
    SceneGraphCameraNode,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Scene.CameraNodeID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have a
## camera node in the [`SceneGraph`](crate::scene::SceneGraph).
SceneGraphCameraNode : {
    ## The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    ## representing the entity.
    id : Scene.CameraNodeID.CameraNodeID,
}

## Creates a new component representing a
## [`SceneGraph`](crate::scene::SceneGraph) camera node with the given ID.
new : Scene.CameraNodeID.CameraNodeID -> SceneGraphCameraNode
new = |node_id|
    { id: node_id }

## Creates a new component representing a
## [`SceneGraph`](crate::scene::SceneGraph) camera node with the given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Scene.CameraNodeID.CameraNodeID -> Entity.Data
add_new = |data, node_id|
    add(data, new(node_id))

## Adds a value of the [SceneGraphCameraNode] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphCameraNode -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SceneGraphCameraNode] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SceneGraphCameraNode -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SceneGraphCameraNode -> List U8
write_packet = |bytes, value|
    type_id = 14072253263621446441
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SceneGraphCameraNode -> List U8
write_multi_packet = |bytes, values|
    type_id = 14072253263621446441
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

## Serializes a value of [SceneGraphCameraNode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneGraphCameraNode -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Scene.CameraNodeID.write_bytes(value.id)

## Deserializes a value of [SceneGraphCameraNode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneGraphCameraNode _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 16 }) |> Scene.CameraNodeID.from_bytes?,
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
