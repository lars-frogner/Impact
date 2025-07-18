# Hash: 178e91282ba4b6560b488e07e7c0c133559f26b11808f478903d608dd73fa4af
# Generated: 2025-07-15T17:32:43+00:00
# Rust type: impact::control::motion::components::MotionControlComp
# Type category: Component
# Commit: 1fbb6f6b (dirty)
module [
    MotionControl,
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
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities whose motion
## that can be controlled by a user.
MotionControl : {
    control_velocity : Vector3.Vector3 Binary64,
}

## Creates a new component for motion control.
new : {} -> MotionControl
new = |{}|
    { control_velocity: Vector3.zero }

## Creates a new component for motion control.
## Adds the component to the given entity's data.
add_new : Entity.Data -> Entity.Data
add_new = |entity_data|
    add(entity_data, new({}))

## Creates a new component for motion control.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData -> Entity.MultiData
add_multiple_new = |entity_data|
    res = add_multiple(
        entity_data,
        Same(new({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in MotionControl.add_multiple_new: ${Inspect.to_str(err)}"

## Adds a value of the [MotionControl] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MotionControl -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MotionControl] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (MotionControl) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MotionControl.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MotionControl -> List U8
write_packet = |bytes, val|
    type_id = 15665890016094755610
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MotionControl -> List U8
write_multi_packet = |bytes, vals|
    type_id = 15665890016094755610
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

## Serializes a value of [MotionControl] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MotionControl -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes_64(value.control_velocity)

## Deserializes a value of [MotionControl] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MotionControl _
from_bytes = |bytes|
    Ok(
        {
            control_velocity: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
        },
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
