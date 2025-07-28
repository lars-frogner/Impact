# Hash: d59ddb272fc516ec7a5b8c22955eb3115cc301126837a85e2d738f9bc90d4b9d
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_physics::quantities::Motion
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    Motion,
    new,
    linear,
    angular,
    stationary,
    add_new,
    add_multiple_new,
    add_linear,
    add_multiple_linear,
    add_angular,
    add_multiple_angular,
    add_stationary,
    add_multiple_stationary,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Physics.AngularVelocity
import core.Builtin
import core.Vector3

## A linear and angular velocity.
Motion : {
    linear_velocity : Vector3.Vector3 Binary64,
    angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

new : Vector3.Vector3 Binary64, Physics.AngularVelocity.AngularVelocity -> Motion
new = |linear_velocity, angular_velocity|
    {
        linear_velocity,
        angular_velocity,
    }

add_new : Entity.Data, Vector3.Vector3 Binary64, Physics.AngularVelocity.AngularVelocity -> Entity.Data
add_new = |entity_data, linear_velocity, angular_velocity|
    add(entity_data, new(linear_velocity, angular_velocity))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Physics.AngularVelocity.AngularVelocity) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, linear_velocity, angular_velocity|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            linear_velocity, angular_velocity,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Motion with the given linear velocity and zero angular velocity.
linear : Vector3.Vector3 Binary64 -> Motion
linear = |velocity|
    new(velocity, Physics.AngularVelocity.zero({}))

## Motion with the given linear velocity and zero angular velocity.
## Adds the component to the given entity's data.
add_linear : Entity.Data, Vector3.Vector3 Binary64 -> Entity.Data
add_linear = |entity_data, velocity|
    add(entity_data, linear(velocity))

## Motion with the given linear velocity and zero angular velocity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_linear : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64) -> Result Entity.MultiData Str
add_multiple_linear = |entity_data, velocity|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            velocity,
            Entity.multi_count(entity_data),
            linear
        ))
    )

## Motion with with the given angular velocity and zero linear velocity.
angular : Physics.AngularVelocity.AngularVelocity -> Motion
angular = |velocity|
    new(Vector3.zero, velocity)

## Motion with with the given angular velocity and zero linear velocity.
## Adds the component to the given entity's data.
add_angular : Entity.Data, Physics.AngularVelocity.AngularVelocity -> Entity.Data
add_angular = |entity_data, velocity|
    add(entity_data, angular(velocity))

## Motion with with the given angular velocity and zero linear velocity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_angular : Entity.MultiData, Entity.Arg.Broadcasted (Physics.AngularVelocity.AngularVelocity) -> Result Entity.MultiData Str
add_multiple_angular = |entity_data, velocity|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            velocity,
            Entity.multi_count(entity_data),
            angular
        ))
    )

## No linear or angular motion.
stationary : {} -> Motion
stationary = |{}|
    linear(Vector3.zero)

## No linear or angular motion.
## Adds the component to the given entity's data.
add_stationary : Entity.Data -> Entity.Data
add_stationary = |entity_data|
    add(entity_data, stationary({}))

## No linear or angular motion.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_stationary : Entity.MultiData -> Entity.MultiData
add_multiple_stationary = |entity_data|
    res = add_multiple(
        entity_data,
        Same(stationary({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in Motion.add_multiple_stationary: ${Inspect.to_str(err)}"

## Adds a value of the [Motion] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, Motion -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [Motion] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (Motion) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in Motion.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, Motion -> List U8
write_packet = |bytes, val|
    type_id = 4790743300244228286
    size = 56
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List Motion -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4790743300244228286
    size = 56
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

## Serializes a value of [Motion] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Motion -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Vector3.write_bytes_64(value.linear_velocity)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [Motion] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Motion _
from_bytes = |bytes|
    Ok(
        {
            linear_velocity: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            angular_velocity: bytes |> List.sublist({ start: 24, len: 32 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
