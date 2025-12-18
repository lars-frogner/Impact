# Hash: fad3b6b111e5d9b964b4b39ca74a6972233820d3e474f30f681a32688cf4a16b
# Generated: 2025-12-17T23:58:42+00:00
# Rust type: impact_physics::force::constant_acceleration::ConstantAcceleration
# Type category: Component
# Commit: 7d41822d (dirty)
module [
    ConstantAcceleration,
    earth_downward_acceleration,
    new,
    downward,
    earth,
    add_new,
    add_multiple_new,
    add_downward,
    add_multiple_downward,
    add_earth,
    add_multiple_earth,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## A constant acceleration vector.
ConstantAcceleration : Vector3.Vector3 Binary32

## The downward acceleration at the surface of Earth [m/s^2].
earth_downward_acceleration : F32
earth_downward_acceleration = 9.81

new : Vector3.Vector3 Binary32 -> ConstantAcceleration
new = |acceleration|
    (acceleration,)

add_new : Entity.ComponentData, Vector3.Vector3 Binary32 -> Entity.ComponentData
add_new = |entity_data, acceleration|
    add(entity_data, new(acceleration))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, acceleration|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            acceleration,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Constant acceleration in the negative y-direction.
downward : F32 -> ConstantAcceleration
downward = |acceleration|
    new((0.0, -acceleration, 0.0))

## Constant acceleration in the negative y-direction.
## Adds the component to the given entity's data.
add_downward : Entity.ComponentData, F32 -> Entity.ComponentData
add_downward = |entity_data, acceleration|
    add(entity_data, downward(acceleration))

## Constant acceleration in the negative y-direction.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_downward : Entity.MultiComponentData, Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_downward = |entity_data, acceleration|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            acceleration,
            Entity.multi_count(entity_data),
            downward
        ))
    )

## The downward gravitational acceleration at the surface of Earth.
earth : {} -> ConstantAcceleration
earth = |{}|
    downward(earth_downward_acceleration)

## The downward gravitational acceleration at the surface of Earth.
## Adds the component to the given entity's data.
add_earth : Entity.ComponentData -> Entity.ComponentData
add_earth = |entity_data|
    add(entity_data, earth({}))

## The downward gravitational acceleration at the surface of Earth.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_earth : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_earth = |entity_data|
    res = add_multiple(
        entity_data,
        Same(earth({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in ConstantAcceleration.add_multiple_earth: ${Inspect.to_str(err)}"

## Adds a value of the [ConstantAcceleration] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ConstantAcceleration -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConstantAcceleration] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ConstantAcceleration) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ConstantAcceleration.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ConstantAcceleration -> List U8
write_packet = |bytes, val|
    type_id = 7546236152181188141
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConstantAcceleration -> List U8
write_multi_packet = |bytes, vals|
    type_id = 7546236152181188141
    size = 12
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

## Serializes a value of [ConstantAcceleration] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAcceleration -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Vector3.write_bytes_32(value)

## Deserializes a value of [ConstantAcceleration] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAcceleration _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
