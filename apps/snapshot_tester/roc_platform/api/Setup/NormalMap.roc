# Hash: fb10c4c53d87a0ab
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_material::setup::physical::NormalMap
# Type category: Component
module [
    NormalMap,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Texture.TextureID
import core.Builtin

## A normal map describing surface details.
NormalMap : Texture.TextureID.TextureID

## Adds a value of the [NormalMap] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, NormalMap -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [NormalMap] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (NormalMap) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in NormalMap.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, NormalMap -> List U8
write_packet = |bytes, val|
    type_id = 14004007724991533434
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List NormalMap -> List U8
write_multi_packet = |bytes, vals|
    type_id = 14004007724991533434
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

## Serializes a value of [NormalMap] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NormalMap -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Texture.TextureID.write_bytes(value)

## Deserializes a value of [NormalMap] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NormalMap _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Texture.TextureID.from_bytes?,
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
