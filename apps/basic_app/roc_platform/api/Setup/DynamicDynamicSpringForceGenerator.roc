# Hash: ba1f9b97c593472c6520956875a25488225a2e1b0d96442002a58e6d069b84c0
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_physics::force::spring_force::DynamicDynamicSpringForceGenerator
# Type category: Component
# Commit: 189570ab (dirty)
module [
    DynamicDynamicSpringForceGenerator,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Entity
import Entity.Arg
import Physics.SpringForce
import core.Builtin

## Generator for a spring force between two dynamic rigid bodies.
DynamicDynamicSpringForceGenerator : {
    ## The first dynamic rigid body the spring is attached to.
    rigid_body_1 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The second dynamic rigid body the spring is attached to.
    rigid_body_2 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The spring force between the two bodies.
    force : Physics.SpringForce.SpringForce,
}

new : Comp.DynamicRigidBodyID.DynamicRigidBodyID, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Physics.SpringForce.SpringForce -> DynamicDynamicSpringForceGenerator
new = |rigid_body_1, rigid_body_2, force|
    { rigid_body_1, rigid_body_2, force }

add_new : Entity.Data, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Physics.SpringForce.SpringForce -> Entity.Data
add_new = |entity_data, rigid_body_1, rigid_body_2, force|
    add(entity_data, new(rigid_body_1, rigid_body_2, force))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Comp.DynamicRigidBodyID.DynamicRigidBodyID), Entity.Arg.Broadcasted (Comp.DynamicRigidBodyID.DynamicRigidBodyID), Entity.Arg.Broadcasted (Physics.SpringForce.SpringForce) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, rigid_body_1, rigid_body_2, force|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            rigid_body_1, rigid_body_2, force,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [DynamicDynamicSpringForceGenerator] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DynamicDynamicSpringForceGenerator -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DynamicDynamicSpringForceGenerator] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DynamicDynamicSpringForceGenerator) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DynamicDynamicSpringForceGenerator.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DynamicDynamicSpringForceGenerator -> List U8
write_packet = |bytes, val|
    type_id = 2377531448442325554
    size = 96
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicDynamicSpringForceGenerator -> List U8
write_multi_packet = |bytes, vals|
    type_id = 2377531448442325554
    size = 96
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

## Serializes a value of [DynamicDynamicSpringForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicDynamicSpringForceGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(96)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_1)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_2)
    |> Physics.SpringForce.write_bytes(value.force)

## Deserializes a value of [DynamicDynamicSpringForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicDynamicSpringForceGenerator _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_1: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            rigid_body_2: bytes |> List.sublist({ start: 8, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            force: bytes |> List.sublist({ start: 16, len: 80 }) |> Physics.SpringForce.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 96 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
