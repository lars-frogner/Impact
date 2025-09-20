# Hash: 5428175c6d0d2dec98480c51287dc33aab918b53572275cfc99f46a8155854eb
# Generated: 2025-09-20T12:42:00+00:00
# Rust type: impact_physics::force::spring_force::DynamicKinematicSpringForceProperties
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    DynamicKinematicSpringForceProperties,
    new,
    add_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Comp.KinematicRigidBodyID
import Entity
import Entity.Arg
import Physics.Spring
import core.Builtin
import core.Point3

## Generator for a spring force between two dynamic rigid bodies.
DynamicKinematicSpringForceProperties : {
    ## The dynamic rigid body the spring is attached to.
    rigid_body_1 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The point where the spring is attached to the first (dynamic) body,
    ## in that body's model space.
    attachment_point_1 : Point3.Point3 Binary64,
    ## The kinematic rigid body the spring is attached to.
    rigid_body_2 : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The point where the spring is attached to the second (kinematic)
    ## body, in that body's model space.
    attachment_point_2 : Point3.Point3 Binary64,
    ## The spring connecting the bodies.
    spring : Physics.Spring.Spring,
}

new : Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Comp.KinematicRigidBodyID.KinematicRigidBodyID, Point3.Point3 Binary64, Physics.Spring.Spring -> DynamicKinematicSpringForceProperties
new = |rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring|
    { rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring }

add_new : Entity.ComponentData, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Comp.KinematicRigidBodyID.KinematicRigidBodyID, Point3.Point3 Binary64, Physics.Spring.Spring -> Entity.ComponentData
add_new = |entity_data, rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring|
    add(entity_data, new(rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring))

## Adds a value of the [DynamicKinematicSpringForceProperties] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, DynamicKinematicSpringForceProperties -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DynamicKinematicSpringForceProperties] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (DynamicKinematicSpringForceProperties) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DynamicKinematicSpringForceProperties.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DynamicKinematicSpringForceProperties -> List U8
write_packet = |bytes, val|
    type_id = 887094482201544204
    size = 96
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicKinematicSpringForceProperties -> List U8
write_multi_packet = |bytes, vals|
    type_id = 887094482201544204
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

## Serializes a value of [DynamicKinematicSpringForceProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicKinematicSpringForceProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(96)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_1)
    |> Point3.write_bytes_64(value.attachment_point_1)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_2)
    |> Point3.write_bytes_64(value.attachment_point_2)
    |> Physics.Spring.write_bytes(value.spring)

## Deserializes a value of [DynamicKinematicSpringForceProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicKinematicSpringForceProperties _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_1: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            attachment_point_1: bytes |> List.sublist({ start: 8, len: 24 }) |> Point3.from_bytes_64?,
            rigid_body_2: bytes |> List.sublist({ start: 32, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            attachment_point_2: bytes |> List.sublist({ start: 40, len: 24 }) |> Point3.from_bytes_64?,
            spring: bytes |> List.sublist({ start: 64, len: 32 }) |> Physics.Spring.from_bytes?,
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
