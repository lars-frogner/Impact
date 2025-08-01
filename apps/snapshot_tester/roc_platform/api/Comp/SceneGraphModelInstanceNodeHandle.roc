# Hash: 953381528246e45be3211234128a1e56e788ae30fba59a73293088e53d925cc2
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_scene::SceneGraphModelInstanceNodeHandle
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    SceneGraphModelInstanceNodeHandle,
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

## Handle to a model instance node in a scene graph.
SceneGraphModelInstanceNodeHandle : {
    ## The ID of the model instance node in the
    ## [`SceneGraph`](crate::graph::SceneGraph).
    id : Scene.ModelInstanceNodeID.ModelInstanceNodeID,
}

## Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
## model instance node with the given ID.
new : Scene.ModelInstanceNodeID.ModelInstanceNodeID -> SceneGraphModelInstanceNodeHandle
new = |node_id|
    { id: node_id }

## Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
## model instance node with the given ID.
## Adds the component to the given entity's data.
add_new : Entity.Data, Scene.ModelInstanceNodeID.ModelInstanceNodeID -> Entity.Data
add_new = |entity_data, node_id|
    add(entity_data, new(node_id))

## Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
## model instance node with the given ID.
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

## Adds a value of the [SceneGraphModelInstanceNodeHandle] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SceneGraphModelInstanceNodeHandle -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SceneGraphModelInstanceNodeHandle] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SceneGraphModelInstanceNodeHandle) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SceneGraphModelInstanceNodeHandle.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SceneGraphModelInstanceNodeHandle -> List U8
write_packet = |bytes, val|
    type_id = 10488504241303494120
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SceneGraphModelInstanceNodeHandle -> List U8
write_multi_packet = |bytes, vals|
    type_id = 10488504241303494120
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

## Serializes a value of [SceneGraphModelInstanceNodeHandle] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneGraphModelInstanceNodeHandle -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Scene.ModelInstanceNodeID.write_bytes(value.id)

## Deserializes a value of [SceneGraphModelInstanceNodeHandle] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneGraphModelInstanceNodeHandle _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 8 }) |> Scene.ModelInstanceNodeID.from_bytes?,
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
