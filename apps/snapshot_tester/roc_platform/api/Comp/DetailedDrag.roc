# Hash: b30d889839296e81697e11b5b7816ff088ac6d887480f8b09a85ae3fbdc63693
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::physics::rigid_body::forces::detailed_drag::components::DetailedDragComp
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    DetailedDrag,
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

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that should be affected by a drag force and torque computed from
## aggregating drag on each point on the body.
##
## The purpose of this component is to aid in constructing a
## [`DragLoadMapComp`] for the entity. It is therefore not kept after entity
## creation.
DetailedDrag : {
    ## The drag coefficient of the body.
    drag_coefficient : F64,
}

## Creates a new component for detailed drag with the given drag
## coefficient.
new : F64 -> DetailedDrag
new = |drag_coefficient|
    { drag_coefficient }

## Creates a new component for detailed drag with the given drag
## coefficient.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64 -> Entity.Data
add_new = |entity_data, drag_coefficient|
    add(entity_data, new(drag_coefficient))

## Creates a new component for detailed drag with the given drag
## coefficient.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
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

## Adds a value of the [DetailedDrag] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DetailedDrag -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DetailedDrag] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DetailedDrag) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DetailedDrag.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DetailedDrag -> List U8
write_packet = |bytes, val|
    type_id = 8840532613153999594
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DetailedDrag -> List U8
write_multi_packet = |bytes, vals|
    type_id = 8840532613153999594
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

## Serializes a value of [DetailedDrag] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DetailedDrag -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f64(value.drag_coefficient)

## Deserializes a value of [DetailedDrag] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DetailedDrag _
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
