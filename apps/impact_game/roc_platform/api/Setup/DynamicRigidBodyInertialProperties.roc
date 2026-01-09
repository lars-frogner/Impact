# Hash: 9f10a53422f14c15
# Generated: 2026-01-07T21:07:46.097611253
# Rust type: impact_physics::rigid_body::setup::DynamicRigidBodyInertialProperties
# Type category: Component
module [
    DynamicRigidBodyInertialProperties,
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
import core.Matrix3
import core.Point3

## The inertial properties of a dynamic rigid body.
DynamicRigidBodyInertialProperties : {
    mass : F32,
    ## The center of mass of the rigid body.
    center_of_mass : Point3.Point3,
    inertia_tensor : Matrix3.Matrix3,
}

new : F32, Point3.Point3, Matrix3.Matrix3 -> DynamicRigidBodyInertialProperties
new = |mass, center_of_mass, inertia_tensor|
    { mass, center_of_mass, inertia_tensor }

add_new : Entity.ComponentData, F32, Point3.Point3, Matrix3.Matrix3 -> Entity.ComponentData
add_new = |entity_data, mass, center_of_mass, inertia_tensor|
    add(entity_data, new(mass, center_of_mass, inertia_tensor))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (Point3.Point3), Entity.Arg.Broadcasted (Matrix3.Matrix3) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, mass, center_of_mass, inertia_tensor|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            mass, center_of_mass, inertia_tensor,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [DynamicRigidBodyInertialProperties] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, DynamicRigidBodyInertialProperties -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DynamicRigidBodyInertialProperties] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (DynamicRigidBodyInertialProperties) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DynamicRigidBodyInertialProperties.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, DynamicRigidBodyInertialProperties -> List U8
write_packet = |bytes, val|
    type_id = 17335812379283144649
    size = 52
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DynamicRigidBodyInertialProperties -> List U8
write_multi_packet = |bytes, vals|
    type_id = 17335812379283144649
    size = 52
    alignment = 4
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

## Serializes a value of [DynamicRigidBodyInertialProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DynamicRigidBodyInertialProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(52)
    |> Builtin.write_bytes_f32(value.mass)
    |> Point3.write_bytes(value.center_of_mass)
    |> Matrix3.write_bytes(value.inertia_tensor)

## Deserializes a value of [DynamicRigidBodyInertialProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DynamicRigidBodyInertialProperties _
from_bytes = |bytes|
    Ok(
        {
            mass: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            center_of_mass: bytes |> List.sublist({ start: 4, len: 12 }) |> Point3.from_bytes?,
            inertia_tensor: bytes |> List.sublist({ start: 16, len: 36 }) |> Matrix3.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 52 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
