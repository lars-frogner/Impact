# Hash: 02aaa24ff4e0c65c97e886930e5e5317a7b9ca68a9b3ba77af3f29eaab97a8aa
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::force::spring_force::DynamicKinematicSpringForceGenerator
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    DynamicKinematicSpringForceGenerator,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Comp.KinematicRigidBodyID
import Entity
import Entity.Arg
import Physics.SpringForce
import core.Builtin

## Generator for a spring force between a dynamic and a kinematic rigid body.
DynamicKinematicSpringForceGenerator : {
    ## The dynamic rigid body the spring is attached to.
    rigid_body_1 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The kinematic rigid body the spring is attached to.
    rigid_body_2 : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The spring force between the two bodies.
    force : Physics.SpringForce.SpringForce,
}

new : Comp.DynamicRigidBodyID.DynamicRigidBodyID, Comp.KinematicRigidBodyID.KinematicRigidBodyID, Physics.SpringForce.SpringForce -> DynamicKinematicSpringForceGenerator
new = |rigid_body_1, rigid_body_2, force|
    { rigid_body_1, rigid_body_2, force }

add_new : Entity.Data, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Comp.KinematicRigidBodyID.KinematicRigidBodyID, Physics.SpringForce.SpringForce -> Entity.Data
add_new = |entity_data, rigid_body_1, rigid_body_2, force|
    add(entity_data, new(rigid_body_1, rigid_body_2, force))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Comp.DynamicRigidBodyID.DynamicRigidBodyID), Entity.Arg.Broadcasted (Comp.KinematicRigidBodyID.KinematicRigidBodyID), Entity.Arg.Broadcasted (Physics.SpringForce.SpringForce) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, rigid_body_1, rigid_body_2, force|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            rigid_body_1, rigid_body_2, force,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [DynamicKinematicSpringForceGenerator] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DynamicKinematicSpringForceGenerator -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DynamicKinematicSpringForceGenerator] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DynamicKinematicSpringForceGenerator) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DynamicKinematicSpringForceGenerator.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DynamicKinematicSpringForceGenerator -> List U8
write_packet = |bytes, val|
    type_id = 13264792234284941810
    size = 96
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicKinematicSpringForceGenerator -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13264792234284941810
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

## Serializes a value of [DynamicKinematicSpringForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicKinematicSpringForceGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(96)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_1)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_2)
    |> Physics.SpringForce.write_bytes(value.force)

## Deserializes a value of [DynamicKinematicSpringForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicKinematicSpringForceGenerator _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_1: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            rigid_body_2: bytes |> List.sublist({ start: 8, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
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
