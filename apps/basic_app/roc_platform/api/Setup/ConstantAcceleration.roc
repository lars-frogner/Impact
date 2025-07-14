# Hash: b19b45c60307534762553d49b9db56444ecf839cc2d0f76db3b597613c83306d
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::force::constant_acceleration::ConstantAcceleration
# Type category: Component
# Commit: b1b4dfd8 (dirty)
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
ConstantAcceleration : Vector3.Vector3 Binary64

## The downward acceleration at the surface of Earth [m/s^2].
earth_downward_acceleration : F64
earth_downward_acceleration = 9.81

new : Vector3.Vector3 Binary64 -> ConstantAcceleration
new = |acceleration|
    (acceleration,)

add_new : Entity.Data, Vector3.Vector3 Binary64 -> Entity.Data
add_new = |entity_data, acceleration|
    add(entity_data, new(acceleration))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64) -> Result Entity.MultiData Str
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
downward : F64 -> ConstantAcceleration
downward = |acceleration|
    new((0.0, -acceleration, 0.0))

## Constant acceleration in the negative y-direction.
## Adds the component to the given entity's data.
add_downward : Entity.Data, F64 -> Entity.Data
add_downward = |entity_data, acceleration|
    add(entity_data, downward(acceleration))

## Constant acceleration in the negative y-direction.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_downward : Entity.MultiData, Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
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
add_earth : Entity.Data -> Entity.Data
add_earth = |entity_data|
    add(entity_data, earth({}))

## The downward gravitational acceleration at the surface of Earth.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_earth : Entity.MultiData -> Entity.MultiData
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
add : Entity.Data, ConstantAcceleration -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConstantAcceleration] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ConstantAcceleration) -> Result Entity.MultiData Str
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
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConstantAcceleration -> List U8
write_multi_packet = |bytes, vals|
    type_id = 7546236152181188141
    size = 24
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

## Serializes a value of [ConstantAcceleration] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAcceleration -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes_64(value)

## Deserializes a value of [ConstantAcceleration] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAcceleration _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
