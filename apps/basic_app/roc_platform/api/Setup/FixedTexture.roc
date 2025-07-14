# Hash: b68868fa794c9492abcdc57c5785987ad8a3a883926828234df47730cddfaf55
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_material::setup::fixed::FixedTexture
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    FixedTexture,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Rendering.TextureID
import core.Builtin

## A fixed, textured color that is independent of lighting.
FixedTexture : Rendering.TextureID.TextureID

## Adds a value of the [FixedTexture] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, FixedTexture -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [FixedTexture] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (FixedTexture) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in FixedTexture.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, FixedTexture -> List U8
write_packet = |bytes, val|
    type_id = 1701685739367250505
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List FixedTexture -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1701685739367250505
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

## Serializes a value of [FixedTexture] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FixedTexture -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Rendering.TextureID.write_bytes(value)

## Deserializes a value of [FixedTexture] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FixedTexture _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Rendering.TextureID.from_bytes?,
        ),
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
