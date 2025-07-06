# Hash: 6e15955d1588ab129753c96cd65ea13450cd6e192d4a44351408dcf8fa9b622a
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact_light::ShadowableOmnidirectionalLightHandle
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    ShadowableOmnidirectionalLightHandle,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Light.LightID
import core.Builtin

## Handle to a [`ShadowableOmnidirectionalLight`].
ShadowableOmnidirectionalLightHandle : {
    ## The ID of the [`ShadowableOmnidirectionalLight`] in the
    ## [`LightStorage`].
    id : Light.LightID.LightID,
}

## Adds a value of the [ShadowableOmnidirectionalLightHandle] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableOmnidirectionalLightHandle -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ShadowableOmnidirectionalLightHandle] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ShadowableOmnidirectionalLightHandle) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ShadowableOmnidirectionalLightHandle.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ShadowableOmnidirectionalLightHandle -> List U8
write_packet = |bytes, val|
    type_id = 17033188383839064735
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ShadowableOmnidirectionalLightHandle -> List U8
write_multi_packet = |bytes, vals|
    type_id = 17033188383839064735
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

## Serializes a value of [ShadowableOmnidirectionalLightHandle] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ShadowableOmnidirectionalLightHandle -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Light.LightID.write_bytes(value.id)

## Deserializes a value of [ShadowableOmnidirectionalLightHandle] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ShadowableOmnidirectionalLightHandle _
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
