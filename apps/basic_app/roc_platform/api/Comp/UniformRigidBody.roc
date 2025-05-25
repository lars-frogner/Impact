# Hash: 663d619684bc2a13496c6cde4a9a5aef948e810cfc90a6061c0f6e12847c5fd9
# Generated: 2025-05-23T20:19:02+00:00
# Rust type: impact::physics::rigid_body::components::UniformRigidBodyComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    UniformRigidBody,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a rigid body with a uniform mass density.
##
## The purpose of this component is to aid in constructing a [`RigidBodyComp`]
## for the entity. It is therefore not kept after entity creation.
UniformRigidBody : {
    mass_density : F64,
}

## Adds a value of the [UniformRigidBody] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformRigidBody -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UniformRigidBody] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (UniformRigidBody) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UniformRigidBody.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, UniformRigidBody -> List U8
write_packet = |bytes, val|
    type_id = 751460017682335564
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UniformRigidBody -> List U8
write_multi_packet = |bytes, vals|
    type_id = 751460017682335564
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

## Serializes a value of [UniformRigidBody] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformRigidBody -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f64(value.mass_density)

## Deserializes a value of [UniformRigidBody] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformRigidBody _
from_bytes = |bytes|
    Ok(
        {
            mass_density: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
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
