# Hash: c37805b7fd505b987eb31a2f7f92c2e11d4aea15fd36aa97a64a90e1ffc08135
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::TexturedMetalnessComp
# Type category: Component
# Commit: d505d37
module [
    TexturedMetalness,
    unscaled,
    add_unscaled,
    add,
    add_multiple,
]

import Entity
import Rendering.TextureID
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a textured metalness.
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
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
TexturedMetalness : {
    texture_id : Rendering.TextureID.TextureID,
    scale_factor : F32,
}

unscaled : Rendering.TextureID.TextureID -> TexturedMetalness
unscaled = |texture_id|
    { texture_id, scale_factor: 1.0 }

add_unscaled : Entity.Data, Rendering.TextureID.TextureID -> Entity.Data
add_unscaled = |data, texture_id|
    add(data, unscaled(texture_id))

## Adds a value of the [TexturedMetalness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, TexturedMetalness -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [TexturedMetalness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List TexturedMetalness -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, TexturedMetalness -> List U8
write_packet = |bytes, value|
    type_id = 9176356752958161098
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List TexturedMetalness -> List U8
write_multi_packet = |bytes, values|
    type_id = 9176356752958161098
    size = 8
    alignment = 4
    count = List.len(values)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    values
    |> List.walk(
        bytes_with_header,
        |bts, value| bts |> write_bytes(value),
    )

## Serializes a value of [TexturedMetalness] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TexturedMetalness -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Rendering.TextureID.write_bytes(value.texture_id)
    |> Builtin.write_bytes_f32(value.scale_factor)

## Deserializes a value of [TexturedMetalness] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TexturedMetalness _
from_bytes = |bytes|
    Ok(
        {
            texture_id: bytes |> List.sublist({ start: 0, len: 4 }) |> Rendering.TextureID.from_bytes?,
            scale_factor: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
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
