# Hash: c93ee1c03b5e92d5ce8f918cf4984bd46706c1ac745833e9b6fab1ded16356e3
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::TexturedRoughnessComp
# Type category: Component
# Commit: d505d37
module [
    TexturedRoughness,
    unscaled,
    add_unscaled,
    add,
    add_multiple,
]

import Entity
import Rendering.TextureID
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a textured surface roughness. The roughness ranges from
## zero (perfectly smooth) to one (completely diffuse).
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
TexturedRoughness : {
    texture_id : Rendering.TextureID.TextureID,
    scale_factor : F32,
}

unscaled : Rendering.TextureID.TextureID -> TexturedRoughness
unscaled = |texture_id|
    { texture_id, scale_factor: 1.0 }

add_unscaled : Entity.Data, Rendering.TextureID.TextureID -> Entity.Data
add_unscaled = |data, texture_id|
    add(data, unscaled(texture_id))

## Adds a value of the [TexturedRoughness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, TexturedRoughness -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [TexturedRoughness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List TexturedRoughness -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, TexturedRoughness -> List U8
write_packet = |bytes, value|
    type_id = 1963253879334442476
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List TexturedRoughness -> List U8
write_multi_packet = |bytes, values|
    type_id = 1963253879334442476
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

## Serializes a value of [TexturedRoughness] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TexturedRoughness -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Rendering.TextureID.write_bytes(value.texture_id)
    |> Builtin.write_bytes_f32(value.scale_factor)

## Deserializes a value of [TexturedRoughness] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TexturedRoughness _
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
