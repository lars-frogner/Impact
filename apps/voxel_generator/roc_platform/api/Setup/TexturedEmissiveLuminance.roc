# Hash: bbe0e03d7200c951
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_material::setup::physical::TexturedEmissiveLuminance
# Type category: Component
module [
    TexturedEmissiveLuminance,
    unscaled,
    add_unscaled,
    add_multiple_unscaled,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Texture.TextureID
import core.Builtin

## A textured monochromatic emissive luminance.
##
## The RGB emissive luminance will be the material's base color multiplied by
## this scalar.
TexturedEmissiveLuminance : {
    texture_id : Texture.TextureID.TextureID,
    scale_factor : F64,
}

unscaled : Texture.TextureID.TextureID -> TexturedEmissiveLuminance
unscaled = |texture_id|
    { texture_id, scale_factor: 1.0 }

add_unscaled : Entity.ComponentData, Texture.TextureID.TextureID -> Entity.ComponentData
add_unscaled = |entity_data, texture_id|
    add(entity_data, unscaled(texture_id))

add_multiple_unscaled : Entity.MultiComponentData, Entity.Arg.Broadcasted (Texture.TextureID.TextureID) -> Result Entity.MultiComponentData Str
add_multiple_unscaled = |entity_data, texture_id|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            texture_id,
            Entity.multi_count(entity_data),
            unscaled
        ))
    )

## Adds a value of the [TexturedEmissiveLuminance] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, TexturedEmissiveLuminance -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [TexturedEmissiveLuminance] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (TexturedEmissiveLuminance) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in TexturedEmissiveLuminance.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, TexturedEmissiveLuminance -> List U8
write_packet = |bytes, val|
    type_id = 18268115868222197081
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List TexturedEmissiveLuminance -> List U8
write_multi_packet = |bytes, vals|
    type_id = 18268115868222197081
    size = 16
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

## Serializes a value of [TexturedEmissiveLuminance] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TexturedEmissiveLuminance -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Texture.TextureID.write_bytes(value.texture_id)
    |> Builtin.write_bytes_f64(value.scale_factor)

## Deserializes a value of [TexturedEmissiveLuminance] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TexturedEmissiveLuminance _
from_bytes = |bytes|
    Ok(
        {
            texture_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Texture.TextureID.from_bytes?,
            scale_factor: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
        },
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
