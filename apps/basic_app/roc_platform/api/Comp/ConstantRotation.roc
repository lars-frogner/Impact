# Hash: 89682b870e2778682681cf6aa4de561951e204c0eac6309c47aedc44112145ec
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::motion::analytical::constant_rotation::components::ConstantRotationComp
# Type category: Component
# Commit: d505d37
module [
    ConstantRotation,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Physics.AngularVelocity
import core.Builtin
import core.UnitQuaternion

## [`Component`](impact_ecs::component::Component) for entities that rotate
## with a constant angular velocity over time.
##
## For this component to have an effect, the entity also needs a
## [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp).
ConstantRotation : {
    ## When (in simulation time) the entity should have the initial
    ## orientation.
    initial_time : F64,
    ## The orientation of the entity at the initial time.
    initial_orientation : UnitQuaternion.UnitQuaternion Binary64,
    ## The angular velocity of the entity.
    angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

## Creates a new component for constant rotation defined by the given
## initial time and orientation and angular velocity.
new : F64, UnitQuaternion.UnitQuaternion Binary64, Physics.AngularVelocity.AngularVelocity -> ConstantRotation
new = |initial_time, initial_orientation, angular_velocity|
    {
        initial_time,
        initial_orientation,
        angular_velocity,
    }

## Creates a new component for constant rotation defined by the given
## initial time and orientation and angular velocity.
## Adds the component to the given entity's data.
add_new : Entity.Data, F64, UnitQuaternion.UnitQuaternion Binary64, Physics.AngularVelocity.AngularVelocity -> Entity.Data
add_new = |data, initial_time, initial_orientation, angular_velocity|
    add(data, new(initial_time, initial_orientation, angular_velocity))

## Adds a value of the [ConstantRotation] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ConstantRotation -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [ConstantRotation] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List ConstantRotation -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, ConstantRotation -> List U8
write_packet = |bytes, value|
    type_id = 2023892632481490330
    size = 72
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List ConstantRotation -> List U8
write_multi_packet = |bytes, values|
    type_id = 2023892632481490330
    size = 72
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

## Serializes a value of [ConstantRotation] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantRotation -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(72)
    |> Builtin.write_bytes_f64(value.initial_time)
    |> UnitQuaternion.write_bytes_64(value.initial_orientation)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [ConstantRotation] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantRotation _
from_bytes = |bytes|
    Ok(
        {
            initial_time: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            initial_orientation: bytes |> List.sublist({ start: 8, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            angular_velocity: bytes |> List.sublist({ start: 40, len: 32 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 72 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
