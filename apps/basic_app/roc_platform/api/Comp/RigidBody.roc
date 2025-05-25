# Hash: b7fd9a2c299728dd25c6874ab7836593447a5fb472fbe932f4b2ad42372b056a
# Generated: 2025-05-23T20:19:02+00:00
# Rust type: impact::physics::rigid_body::components::RigidBodyComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    RigidBody,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Physics.RigidBody
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that have a
## rigid body. Transparently wraps a [`RigidBody`].
RigidBody : Physics.RigidBody.RigidBody

## Adds a value of the [RigidBody] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, RigidBody -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [RigidBody] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (RigidBody) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in RigidBody.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, RigidBody -> List U8
write_packet = |bytes, val|
    type_id = 5315438492690380404
    size = 272
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List RigidBody -> List U8
write_multi_packet = |bytes, vals|
    type_id = 5315438492690380404
    size = 272
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

## Serializes a value of [RigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(272)
    |> Physics.RigidBody.write_bytes(value)

## Deserializes a value of [RigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RigidBody _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 272 }) |> Physics.RigidBody.from_bytes?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 272 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
