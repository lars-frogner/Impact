# Hash: 9cfbdbf06a8e5fe19d84e5b11957c5b57dbac75ff6bfdd6d2c6d8a0ff67c1103
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::force::detailed_drag::setup::DetailedDragProperties
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    DetailedDragProperties,
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

## The properties governing the effect of a shape-dependent drag on a body.
DetailedDragProperties : {
    ## The drag coefficient of the body.
    drag_coefficient : F64,
}

new : F64 -> DetailedDragProperties
new = |drag_coefficient|
    { drag_coefficient }

add_new : Entity.Data, F64 -> Entity.Data
add_new = |entity_data, drag_coefficient|
    add(entity_data, new(drag_coefficient))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, drag_coefficient|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            drag_coefficient,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [DetailedDragProperties] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DetailedDragProperties -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DetailedDragProperties] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DetailedDragProperties) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DetailedDragProperties.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DetailedDragProperties -> List U8
write_packet = |bytes, val|
    type_id = 7526283440430984683
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DetailedDragProperties -> List U8
write_multi_packet = |bytes, vals|
    type_id = 7526283440430984683
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

## Serializes a value of [DetailedDragProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDragProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f64(value.drag_coefficient)

## Deserializes a value of [DetailedDragProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDragProperties _
from_bytes = |bytes|
    Ok(
        {
            drag_coefficient: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
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
