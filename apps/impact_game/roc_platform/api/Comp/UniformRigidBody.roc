# Hash: f522f78649f6cd048e74f177154f7f4b724512656cc2f15b1c8f45a7b64c20f9
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::components::UniformRigidBodyComp
# Type category: Component
# Commit: d505d37
module [
    UniformRigidBody,
    add,
    add_multiple,
]

import Entity
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
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UniformRigidBody] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UniformRigidBody -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UniformRigidBody -> List U8
write_packet = |bytes, value|
    type_id = 751460017682335564
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UniformRigidBody -> List U8
write_multi_packet = |bytes, values|
    type_id = 751460017682335564
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
