# Hash: 18dd9fa5d9517b0d509e7bed68c405ddb4a7177253adb2c9119c0f3b7a6d938d
# Generated: 2025-08-01T06:54:20+00:00
# Rust type: impact_material::setup::physical::TexturedMetalness
# Type category: Component
# Commit: 5cd592d6 (dirty)
module [
    TexturedMetalness,
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

## A textured metalness.
##
## The metalness describes the conductive properties of the material. A value
## of zero means that the material is dielectric, while a value of one means
## that the material is a metal.
##
## A dielectric material will have an RGB diffuse reflectance corresponding
## to the material's base color, and a specular reflectance that is the
## same for each color component (and equal to the scalar specular
## reflectance).
##
## A metallic material will have zero diffuse reflectance, and an RGB
## specular reflectance corresponding to the material's base color
## multiplied by the scalar specular reflectance.
TexturedMetalness : {
    texture_id : Texture.TextureID.TextureID,
    scale_factor : F64,
}

unscaled : Texture.TextureID.TextureID -> TexturedMetalness
unscaled = |texture_id|
    { texture_id, scale_factor: 1.0 }

add_unscaled : Entity.Data, Texture.TextureID.TextureID -> Entity.Data
add_unscaled = |entity_data, texture_id|
    add(entity_data, unscaled(texture_id))

add_multiple_unscaled : Entity.MultiData, Entity.Arg.Broadcasted (Texture.TextureID.TextureID) -> Result Entity.MultiData Str
add_multiple_unscaled = |entity_data, texture_id|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            texture_id,
            Entity.multi_count(entity_data),
            unscaled
        ))
    )

## Adds a value of the [TexturedMetalness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, TexturedMetalness -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [TexturedMetalness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (TexturedMetalness) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in TexturedMetalness.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, TexturedMetalness -> List U8
write_packet = |bytes, val|
    type_id = 1182806869316090646
    size = 16
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List TexturedMetalness -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1182806869316090646
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

## Serializes a value of [TexturedMetalness] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TexturedMetalness -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Texture.TextureID.write_bytes(value.texture_id)
    |> Builtin.write_bytes_f64(value.scale_factor)

## Deserializes a value of [TexturedMetalness] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TexturedMetalness _
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
