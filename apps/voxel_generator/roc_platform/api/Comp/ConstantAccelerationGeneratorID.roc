# Hash: 8e927c67b4ccde6f
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_physics::force::constant_acceleration::ConstantAccelerationGeneratorID
# Type category: Component
module [
    ConstantAccelerationGeneratorID,
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

## Identifier for a [`ConstantAccelerationGenerator`].
ConstantAccelerationGeneratorID : U64

## Adds a value of the [ConstantAccelerationGeneratorID] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ConstantAccelerationGeneratorID -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ConstantAccelerationGeneratorID] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ConstantAccelerationGeneratorID) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ConstantAccelerationGeneratorID.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [ConstantAccelerationGeneratorID] component.
component_id = 17640757938144737719

## Adds the ID of the [ConstantAccelerationGeneratorID] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result ConstantAccelerationGeneratorID Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No ConstantAccelerationGeneratorID component in data"
                Decode(decode_err) -> "Failed to decode ConstantAccelerationGeneratorID component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result ConstantAccelerationGeneratorID Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : ConstantAccelerationGeneratorID, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, ConstantAccelerationGeneratorID -> List U8
write_packet = |bytes, val|
    type_id = 17640757938144737719
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ConstantAccelerationGeneratorID -> List U8
write_multi_packet = |bytes, vals|
    type_id = 17640757938144737719
    size = 8
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

## Serializes a value of [ConstantAccelerationGeneratorID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAccelerationGeneratorID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_u64(value)

## Deserializes a value of [ConstantAccelerationGeneratorID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationGeneratorID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
