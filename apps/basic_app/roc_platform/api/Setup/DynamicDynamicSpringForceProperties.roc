# Hash: f28d6c03fbb4ee76fff792439c86100bc15cabd1b957a9ca9c738503b5c45e3f
# Generated: 2025-08-15T19:06:44+00:00
# Rust type: impact_physics::force::spring_force::DynamicDynamicSpringForceProperties
# Type category: Component
# Commit: e6f6ed4f (dirty)
module [
    DynamicDynamicSpringForceProperties,
    new,
    add_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Entity
import Entity.Arg
import Physics.Spring
import core.Builtin
import core.Point3

## Generator for a spring force between two dynamic rigid bodies.
DynamicDynamicSpringForceProperties : {
    ## The first dynamic rigid body the spring is attached to.
    rigid_body_1 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The point where the spring is attached to the first body, in that
    ## body's model space.
    attachment_point_1 : Point3.Point3 Binary64,
    ## The second dynamic rigid body the spring is attached to.
    rigid_body_2 : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The point where the spring is attached to the second body, in that
    ## body's model space.
    attachment_point_2 : Point3.Point3 Binary64,
    ## The spring connecting the bodies.
    spring : Physics.Spring.Spring,
}

new : Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Physics.Spring.Spring -> DynamicDynamicSpringForceProperties
new = |rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring|
    { rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring }

add_new : Entity.Data, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Comp.DynamicRigidBodyID.DynamicRigidBodyID, Point3.Point3 Binary64, Physics.Spring.Spring -> Entity.Data
add_new = |entity_data, rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring|
    add(entity_data, new(rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring))

## Adds a value of the [DynamicDynamicSpringForceProperties] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, DynamicDynamicSpringForceProperties -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DynamicDynamicSpringForceProperties] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (DynamicDynamicSpringForceProperties) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DynamicDynamicSpringForceProperties.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DynamicDynamicSpringForceProperties -> List U8
write_packet = |bytes, val|
    type_id = 15279784466618597196
    size = 96
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicDynamicSpringForceProperties -> List U8
write_multi_packet = |bytes, vals|
    type_id = 15279784466618597196
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

## Serializes a value of [DynamicDynamicSpringForceProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicDynamicSpringForceProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(96)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_1)
    |> Point3.write_bytes_64(value.attachment_point_1)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_2)
    |> Point3.write_bytes_64(value.attachment_point_2)
    |> Physics.Spring.write_bytes(value.spring)

## Deserializes a value of [DynamicDynamicSpringForceProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicDynamicSpringForceProperties _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_1: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            attachment_point_1: bytes |> List.sublist({ start: 8, len: 24 }) |> Point3.from_bytes_64?,
            rigid_body_2: bytes |> List.sublist({ start: 32, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
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
