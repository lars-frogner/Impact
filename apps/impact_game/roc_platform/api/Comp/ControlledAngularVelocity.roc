# Hash: 63c51fd10f191076
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact_controller::orientation::ControlledAngularVelocity
# Type category: Component
module [
    ControlledAngularVelocity,
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
import Physics.AngularVelocity
import core.Builtin

## Angular velocity controller by a user.
ControlledAngularVelocity : Physics.AngularVelocity.AngularVelocity

## Creates a new controlled angular velocity.
new : {} -> ControlledAngularVelocity
new = |{}|
    (Physics.AngularVelocity.zero({}),)

## Creates a new controlled angular velocity.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData -> Entity.ComponentData
add_new = |entity_data|
    add(entity_data, new({}))

## Creates a new controlled angular velocity.
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
        Err(err) -> crash "unexpected error in ControlledAngularVelocity.add_multiple_new: ${Inspect.to_str(err)}"

## Adds a value of the [ControlledAngularVelocity] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ControlledAngularVelocity -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ControlledAngularVelocity] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ControlledAngularVelocity) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ControlledAngularVelocity.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [ControlledAngularVelocity] component.
component_id = 15898146010921466381

## Adds the ID of the [ControlledAngularVelocity] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result ControlledAngularVelocity Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No ControlledAngularVelocity component in data"
                Decode(decode_err) -> "Failed to decode ControlledAngularVelocity component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result ControlledAngularVelocity Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : ControlledAngularVelocity, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, ControlledAngularVelocity -> List U8
write_packet = |bytes, val|
    type_id = 15898146010921466381
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ControlledAngularVelocity -> List U8
write_multi_packet = |bytes, vals|
    type_id = 15898146010921466381
    size = 16
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

## Serializes a value of [ControlledAngularVelocity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControlledAngularVelocity -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Physics.AngularVelocity.write_bytes(value)

## Deserializes a value of [ControlledAngularVelocity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControlledAngularVelocity _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 16 }) |> Physics.AngularVelocity.from_bytes?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
