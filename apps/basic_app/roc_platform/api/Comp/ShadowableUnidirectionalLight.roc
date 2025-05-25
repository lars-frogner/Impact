# Hash: 013e7254b6bee37115ed88d06a78c1fb8067214117fd82a40105cf0561f50f1e
# Generated: 2025-05-23T20:19:02+00:00
# Rust type: impact::light::components::ShadowableUnidirectionalLightComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    ShadowableUnidirectionalLight,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Light.LightID
import core.Builtin

## [`Component`](impact_ecs::component::Component) for entities that
## have a [`ShadowableUnidirectionalLight`](crate::light::ShadowableUnidirectionalLight).
ShadowableUnidirectionalLight : {
    ## The ID of the entity's
    ## [`ShadowableUnidirectionalLight`](crate::light::ShadowableUnidirectionalLight).
    id : Light.LightID.LightID,
}

## Adds a value of the [ShadowableUnidirectionalLight] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableUnidirectionalLight -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ShadowableUnidirectionalLight] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ShadowableUnidirectionalLight) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ShadowableUnidirectionalLight.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ShadowableUnidirectionalLight -> List U8
write_packet = |bytes, val|
    type_id = 1118200380627632838
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ShadowableUnidirectionalLight -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1118200380627632838
    size = 4
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

## Serializes a value of [ShadowableUnidirectionalLight] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ShadowableUnidirectionalLight -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Light.LightID.write_bytes(value.id)

## Deserializes a value of [ShadowableUnidirectionalLight] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ShadowableUnidirectionalLight _
from_bytes = |bytes|
    Ok(
        {
            id: bytes |> List.sublist({ start: 0, len: 4 }) |> Light.LightID.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 4 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
