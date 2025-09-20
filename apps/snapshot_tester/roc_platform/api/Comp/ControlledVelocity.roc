# Hash: 3b3e3ad931e0d7d43099609e15a8be9ed0a89d1ca03cf162560ec203257dba7c
# Generated: 2025-09-20T15:21:45+00:00
# Rust type: impact_controller::motion::ControlledVelocity
# Type category: Component
# Commit: d4065e65 (dirty)
module [
    ControlledVelocity,
    new,
    add_new,
    add_multiple_new,
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
import core.Builtin
import core.Vector3

## Velocity controller by a user.
ControlledVelocity : Vector3.Vector3 Binary64

## Creates a new controlled velocity.
new : {} -> ControlledVelocity
new = |{}|
    (Vector3.zero,)

## Creates a new controlled velocity.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData -> Entity.ComponentData
add_new = |entity_data|
    add(entity_data, new({}))

## Creates a new controlled velocity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_new = |entity_data|
    res = add_multiple(
        entity_data,
        Same(new({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in ControlledVelocity.add_multiple_new: ${Inspect.to_str(err)}"

## Adds a value of the [ControlledVelocity] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ControlledVelocity -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ControlledVelocity] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ControlledVelocity) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ControlledVelocity.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [ControlledVelocity] component.
component_id = 8336497689488528547

## Adds the ID of the [ControlledVelocity] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result ControlledVelocity Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No ControlledVelocity component in data"
                Decode(decode_err) -> "Failed to decode ControlledVelocity component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result ControlledVelocity Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : ControlledVelocity, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, ControlledVelocity -> List U8
write_packet = |bytes, val|
    type_id = 8336497689488528547
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ControlledVelocity -> List U8
write_multi_packet = |bytes, vals|
    type_id = 8336497689488528547
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

## Serializes a value of [ControlledVelocity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControlledVelocity -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes_64(value)

## Deserializes a value of [ControlledVelocity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControlledVelocity _
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
