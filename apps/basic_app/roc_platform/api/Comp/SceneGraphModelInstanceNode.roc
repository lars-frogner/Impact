# Hash: 2ba298306db8c6bc8b1496157d0944ebbb9b33c07b766fc22797acfe1ca22d89
# Generated: 2025-05-23T21:48:57+00:00
# Rust type: impact::scene::components::SceneGraphModelInstanceNodeComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    SceneGraphModelInstanceNode,
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
add_new = |entity_data, node_id|
    add(entity_data, new(node_id))

## Creates a new component representing a
## [`SceneGraph`](crate::scene::SceneGraph) model instance node with the
## given ID.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Scene.ModelInstanceNodeID.ModelInstanceNodeID) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, node_id|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            node_id,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SceneGraphModelInstanceNode] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphModelInstanceNode -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SceneGraphModelInstanceNode] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SceneGraphModelInstanceNode) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SceneGraphModelInstanceNode.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SceneGraphModelInstanceNode -> List U8
write_packet = |bytes, val|
    type_id = 16196741923898652498
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SceneGraphModelInstanceNode -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16196741923898652498
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
