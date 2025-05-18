# Hash: 98ca116054a405848f26c5b58444a333e75eb1ca8b673031646444e64f918b67
# Generated: 2025-05-18T19:54:00+00:00
# Rust type: impact::physics::rigid_body::forces::spring::components::SpringComp
# Type category: Component
# Commit: b32d32f (dirty)
module [
    Spring,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Physics.Spring
import Physics.SpringState
import core.Builtin
import core.Point3

## [`Component`](impact_ecs::component::Component) for entities that have a
## spring connecting two other entities.
Spring : {
    ## The first entity the spring is attached to.
    entity_1_id : Entity.Id,
    ## The second entity the spring is attached to.
    entity_2_id : Entity.Id,
    ## The point where the spring is attached to the first entity, in that
    ## entity's reference frame.
    attachment_point_1 : Point3.Point3 Binary64,
    ## The point where the spring is attached to the second entity, in that
    ## entity's reference frame.
    attachment_point_2 : Point3.Point3 Binary64,
    ## The spring connecting the entities.
    spring : Physics.Spring.Spring,
    ## The current state of the spring.
    spring_state : Physics.SpringState.SpringState,
}

## Creates a new component for a spring connecting two entities.
new : Entity.Id, Entity.Id, Point3.Point3 Binary64, Point3.Point3 Binary64, Physics.Spring.Spring -> Spring
new = |entity_1_id, entity_2_id, attachment_point_1, attachment_point_2, spring|
    {
        entity_1_id,
        entity_2_id,
        attachment_point_1,
        attachment_point_2,
        spring,
        spring_state: Physics.SpringState.new({})
    }

## Creates a new component for a spring connecting two entities.
## Adds the component to the given entity's data.
add_new : Entity.Data, Entity.Id, Entity.Id, Point3.Point3 Binary64, Point3.Point3 Binary64, Physics.Spring.Spring -> Entity.Data
add_new = |data, entity_1_id, entity_2_id, attachment_point_1, attachment_point_2, spring|
    add(data, new(entity_1_id, entity_2_id, attachment_point_1, attachment_point_2, spring))

## Adds a value of the [Spring] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Spring -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [Spring] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List Spring -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, Spring -> List U8
write_packet = |bytes, value|
    type_id = 11003029665670884895
    size = 144
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List Spring -> List U8
write_multi_packet = |bytes, values|
    type_id = 11003029665670884895
    size = 144
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

## Serializes a value of [Spring] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Spring -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(144)
    |> Entity.write_bytes_id(value.entity_1_id)
    |> Entity.write_bytes_id(value.entity_2_id)
    |> Point3.write_bytes_64(value.attachment_point_1)
    |> Point3.write_bytes_64(value.attachment_point_2)
    |> Physics.Spring.write_bytes(value.spring)
    |> Physics.SpringState.write_bytes(value.spring_state)

## Deserializes a value of [Spring] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Spring _
from_bytes = |bytes|
    Ok(
        {
            entity_1_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
            entity_2_id: bytes |> List.sublist({ start: 8, len: 8 }) |> Entity.from_bytes_id?,
            attachment_point_1: bytes |> List.sublist({ start: 16, len: 24 }) |> Point3.from_bytes_64?,
            attachment_point_2: bytes |> List.sublist({ start: 40, len: 24 }) |> Point3.from_bytes_64?,
            spring: bytes |> List.sublist({ start: 64, len: 32 }) |> Physics.Spring.from_bytes?,
            spring_state: bytes |> List.sublist({ start: 96, len: 48 }) |> Physics.SpringState.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 144 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
