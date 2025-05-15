# Hash: 8d02778e8c5438a2cf41f7ed0ba1f922034262a2778c5d62969eafcf1954c897
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::forces::detailed_drag::components::DetailedDragComp
# Type category: Component
# Commit: d505d37
module [
    DetailedDrag,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
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
add_new = |data, drag_coefficient|
    add(data, new(drag_coefficient))

## Adds a value of the [DetailedDrag] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DetailedDrag -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [DetailedDrag] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List DetailedDrag -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, DetailedDrag -> List U8
write_packet = |bytes, value|
    type_id = 8840532613153999594
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List DetailedDrag -> List U8
write_multi_packet = |bytes, values|
    type_id = 8840532613153999594
    size = 8
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
