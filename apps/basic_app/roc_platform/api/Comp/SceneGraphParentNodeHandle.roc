# Hash: a5c8246d5e36a1400e0c9e4d2011164dd84351a0854bbff14f2221271b59ee58
# Generated: 2025-09-20T11:57:44+00:00
# Rust type: impact_scene::SceneGraphParentNodeHandle
# Type category: Component
# Commit: ac7f80d7 (dirty)
module [
    SceneGraphParentNodeHandle,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    component_id,
    add_component_id,
    read,
    get_for_entity!,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Scene.GroupNodeID
import core.Builtin

## Handle to a parent group node in a scene graph.
SceneGraphParentNodeHandle : {
    ## The ID of the parent node in the
    ## [`SceneGraph`](crate::graph::SceneGraph).
    id : Scene.GroupNodeID.GroupNodeID,
}

## Creates a new handle to the parent
## [`SceneGraph`](crate::graph::SceneGraph) group node with the given ID.
new : Scene.GroupNodeID.GroupNodeID -> SceneGraphParentNodeHandle
new = |parent_node_id|
    { id: parent_node_id }

## Creates a new handle to the parent
## [`SceneGraph`](crate::graph::SceneGraph) group node with the given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Scene.GroupNodeID.GroupNodeID -> Entity.Data
add_new = |entity_data, parent_node_id|
    add(entity_data, new(parent_node_id))

## Creates a new handle to the parent
## [`SceneGraph`](crate::graph::SceneGraph) group node with the given ID.
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

## Adds a value of the [SceneGraphParentNodeHandle] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphParentNodeHandle -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SceneGraphParentNodeHandle] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SceneGraphParentNodeHandle) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SceneGraphParentNodeHandle.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [SceneGraphParentNodeHandle] component.
component_id = 17542722866500004650

## Adds the ID of the [SceneGraphParentNodeHandle] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.Data -> Result SceneGraphParentNodeHandle Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No SceneGraphParentNodeHandle component in data"
                Decode(decode_err) -> "Failed to decode SceneGraphParentNodeHandle component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result SceneGraphParentNodeHandle Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

write_packet : List U8, SceneGraphParentNodeHandle -> List U8
write_packet = |bytes, val|
    type_id = 17542722866500004650
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SceneGraphParentNodeHandle -> List U8
write_multi_packet = |bytes, vals|
    type_id = 17542722866500004650
    size = 8
    alignment = 4
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

## Serializes a value of [SceneGraphParentNodeHandle] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneGraphParentNodeHandle -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Scene.GroupNodeID.write_bytes(value.id)

## Deserializes a value of [SceneGraphParentNodeHandle] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneGraphParentNodeHandle _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 8 }) |> Scene.GroupNodeID.from_bytes?,
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
