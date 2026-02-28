# Hash: 1a5d2a1413d7520c
# Generated: 2026-02-28T21:29:35.669709239
# Rust type: impact_scene::DistanceTriggeredRules
# Type category: Component
module [
    DistanceTriggeredRules,
    new,
    add_new,
    add_multiple_new,
    removal,
    add_removal,
    add_multiple_removal,
    no_shadowing,
    add_no_shadowing,
    add_multiple_no_shadowing,
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

## Rules defining entity behavior when it exceeds certain distances from an
## anchor entity.
DistanceTriggeredRules : {
    ## The ID of the entity the distance is measured from.
    anchor_id : Entity.Id,
    ## The square of the distance beyond which the entity will no longer
    ## cast shadows.
    no_shadowing_dist_squared : F64,
    ## The square of the distance at which the entity will be removed.
    removal_dist_squared : F64,
}

## Creates new rules for disabling shadowing and removal beyond the given
## distances from the given anchor entity.
new : Entity.Id, F32, F32 -> DistanceTriggeredRules
new = |anchor_id, no_shadowing_distance, removal_distance|
    {
        anchor_id,
        no_shadowing_dist_squared: Num.to_f64(no_shadowing_distance * no_shadowing_distance),
        removal_dist_squared: Num.to_f64(removal_distance * removal_distance),
    }

## Creates new rules for disabling shadowing and removal beyond the given
## distances from the given anchor entity.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, Entity.Id, F32, F32 -> Entity.ComponentData
add_new = |entity_data, anchor_id, no_shadowing_distance, removal_distance|
    add(entity_data, new(anchor_id, no_shadowing_distance, removal_distance))

## Creates new rules for disabling shadowing and removal beyond the given
## distances from the given anchor entity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Entity.Id), Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, anchor_id, no_shadowing_distance, removal_distance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            anchor_id, no_shadowing_distance, removal_distance,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates a new rule for removal beyond the given distance from the given
## anchor entity.
removal : Entity.Id, F32 -> DistanceTriggeredRules
removal = |anchor_id, removal_distance|
    {
        anchor_id,
        no_shadowing_dist_squared: Num.infinity_u64,
        removal_dist_squared: Num.to_f64(removal_distance * removal_distance),
    }

## Creates a new rule for removal beyond the given distance from the given
## anchor entity.
## Adds the component to the given entity's data.
add_removal : Entity.ComponentData, Entity.Id, F32 -> Entity.ComponentData
add_removal = |entity_data, anchor_id, removal_distance|
    add(entity_data, removal(anchor_id, removal_distance))

## Creates a new rule for removal beyond the given distance from the given
## anchor entity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_removal : Entity.MultiComponentData, Entity.Arg.Broadcasted (Entity.Id), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_removal = |entity_data, anchor_id, removal_distance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            anchor_id, removal_distance,
            Entity.multi_count(entity_data),
            removal
        ))
    )

## Creates a new rule for disabling shadowing beyond the given distance
## from the given anchor entity.
no_shadowing : Entity.Id, F32 -> DistanceTriggeredRules
no_shadowing = |anchor_id, no_shadowing_distance|
    {
        anchor_id,
        no_shadowing_dist_squared: Num.to_f64(no_shadowing_distance * no_shadowing_distance),
        removal_dist_squared: Num.infinity_u64,
    }

## Creates a new rule for disabling shadowing beyond the given distance
## from the given anchor entity.
## Adds the component to the given entity's data.
add_no_shadowing : Entity.ComponentData, Entity.Id, F32 -> Entity.ComponentData
add_no_shadowing = |entity_data, anchor_id, no_shadowing_distance|
    add(entity_data, no_shadowing(anchor_id, no_shadowing_distance))

## Creates a new rule for disabling shadowing beyond the given distance
## from the given anchor entity.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_no_shadowing : Entity.MultiComponentData, Entity.Arg.Broadcasted (Entity.Id), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiComponentData Str
add_multiple_no_shadowing = |entity_data, anchor_id, no_shadowing_distance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            anchor_id, no_shadowing_distance,
            Entity.multi_count(entity_data),
            no_shadowing
        ))
    )

## Adds a value of the [DistanceTriggeredRules] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, DistanceTriggeredRules -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [DistanceTriggeredRules] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (DistanceTriggeredRules) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in DistanceTriggeredRules.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [DistanceTriggeredRules] component.
component_id = 638872501745223447

## Adds the ID of the [DistanceTriggeredRules] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result DistanceTriggeredRules Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No DistanceTriggeredRules component in data"
                Decode(decode_err) -> "Failed to decode DistanceTriggeredRules component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result DistanceTriggeredRules Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : DistanceTriggeredRules, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, DistanceTriggeredRules -> List U8
write_packet = |bytes, val|
    type_id = 638872501745223447
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List DistanceTriggeredRules -> List U8
write_multi_packet = |bytes, vals|
    type_id = 638872501745223447
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

## Serializes a value of [DistanceTriggeredRules] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DistanceTriggeredRules -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Entity.write_bytes_id(value.anchor_id)
    |> Builtin.write_bytes_f64(value.no_shadowing_dist_squared)
    |> Builtin.write_bytes_f64(value.removal_dist_squared)

## Deserializes a value of [DistanceTriggeredRules] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DistanceTriggeredRules _
from_bytes = |bytes|
    Ok(
        {
            anchor_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
            no_shadowing_dist_squared: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            removal_dist_squared: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
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
