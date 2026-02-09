# Hash: 914c9a9991b2d8c7
# Generated: 2026-02-09T22:23:05.797966103
# Rust type: impact_physics::force::spring_force::HasDynamicKinematicSpringForceGenerator
# Type category: Component
module [
    HasDynamicKinematicSpringForceGenerator,
    add,
    add_multiple,
    component_id,
    add_component_id,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## Marks that an entity has a dynamic-kinematic spring force generator
## identified by a [`DynamicKinematicSpringForceGeneratorID`].
##
## Use [`DynamicKinematicSpringForceGeneratorID::from_entity_id`] to obtain
## the generator ID from the entity ID.
HasDynamicKinematicSpringForceGenerator : {}

## Adds the [HasDynamicKinematicSpringForceGenerator] component to an entity's data.
add : Entity.ComponentData -> Entity.ComponentData
add = |entity_data|
    entity_data |> Entity.append_component(write_packet, {})

## Adds the [HasDynamicKinematicSpringForceGenerator] component to each entity's data.
add_multiple : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple = |entity_data|
    res = entity_data
        |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(Same({}), Entity.multi_count(entity_data)))
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in HasDynamicKinematicSpringForceGenerator.add_multiple: ${Inspect.to_str(err)}"

## The ID of the [HasDynamicKinematicSpringForceGenerator] component.
component_id = 1912309565031143008

## Adds the ID of the [HasDynamicKinematicSpringForceGenerator] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

write_packet : List U8, HasDynamicKinematicSpringForceGenerator -> List U8
write_packet = |bytes, val|
    type_id = 1912309565031143008
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List HasDynamicKinematicSpringForceGenerator -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1912309565031143008
    size = 0
    alignment = 1
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

## Serializes a value of [HasDynamicKinematicSpringForceGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, HasDynamicKinematicSpringForceGenerator -> List U8
write_bytes = |bytes, _value|
    bytes

## Deserializes a value of [HasDynamicKinematicSpringForceGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result HasDynamicKinematicSpringForceGenerator _
from_bytes = |_bytes|
    Ok({})

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 0 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
