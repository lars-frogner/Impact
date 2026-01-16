# Hash: ee5b6fdbc21ce3ae
# Generated: 2026-01-16T10:02:50.675953324
# Rust type: impact_physics::quantities::Motion
# Type category: Component
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
    component_id,
    add_component_id,
    read,
    get_for_entity!,
    set_for_entity!,
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
    linear_velocity : Vector3.Vector3,
    angular_velocity : Physics.AngularVelocity.AngularVelocity,
}

new : Vector3.Vector3, Physics.AngularVelocity.AngularVelocity -> Motion
new = |linear_velocity, angular_velocity|
    {
        linear_velocity,
        angular_velocity,
    }

add_new : Entity.ComponentData, Vector3.Vector3, Physics.AngularVelocity.AngularVelocity -> Entity.ComponentData
add_new = |entity_data, linear_velocity, angular_velocity|
    add(entity_data, new(linear_velocity, angular_velocity))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3), Entity.Arg.Broadcasted (Physics.AngularVelocity.AngularVelocity) -> Result Entity.MultiComponentData Str
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
linear : Vector3.Vector3 -> Motion
linear = |velocity|
    new(velocity, Physics.AngularVelocity.zero({}))

## Motion with the given linear velocity and zero angular velocity.
## Adds the component to the given entity's data.
add_linear : Entity.ComponentData, Vector3.Vector3 -> Entity.ComponentData
add_linear = |entity_data, velocity|
    add(entity_data, linear(velocity))

## Motion with the given linear velocity and zero angular velocity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_linear : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3) -> Result Entity.MultiComponentData Str
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
    new(Vector3.zeros, velocity)

## Motion with with the given angular velocity and zero linear velocity.
## Adds the component to the given entity's data.
add_angular : Entity.ComponentData, Physics.AngularVelocity.AngularVelocity -> Entity.ComponentData
add_angular = |entity_data, velocity|
    add(entity_data, angular(velocity))

## Motion with with the given angular velocity and zero linear velocity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_angular : Entity.MultiComponentData, Entity.Arg.Broadcasted (Physics.AngularVelocity.AngularVelocity) -> Result Entity.MultiComponentData Str
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
    linear(Vector3.zeros)

## No linear or angular motion.
## Adds the component to the given entity's data.
add_stationary : Entity.ComponentData -> Entity.ComponentData
add_stationary = |entity_data|
    add(entity_data, stationary({}))

## No linear or angular motion.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_stationary : Entity.MultiComponentData -> Entity.MultiComponentData
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
add : Entity.ComponentData, Motion -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [Motion] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (Motion) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in Motion.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [Motion] component.
component_id = 4790743300244228286

## Adds the ID of the [Motion] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result Motion Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No Motion component in data"
                Decode(decode_err) -> "Failed to decode Motion component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result Motion Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : Motion, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, Motion -> List U8
write_packet = |bytes, val|
    type_id = 4790743300244228286
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List Motion -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4790743300244228286
    size = 28
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

## Serializes a value of [Motion] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Motion -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Vector3.write_bytes(value.linear_velocity)
    |> Physics.AngularVelocity.write_bytes(value.angular_velocity)

## Deserializes a value of [Motion] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Motion _
from_bytes = |bytes|
    Ok(
        {
            linear_velocity: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes?,
            angular_velocity: bytes |> List.sublist({ start: 12, len: 16 }) |> Physics.AngularVelocity.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 28 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
