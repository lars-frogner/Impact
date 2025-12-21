# Hash: 896e6acbc3669af50a65fb729642507072b530e21da7c3b2225dbff3a29e004c
# Generated: 2025-12-21T22:57:59+00:00
# Rust type: impact_material::setup::physical::ParallaxMap
# Type category: Component
# Commit: d4c84c05 (dirty)
module [
    ParallaxMap,
    new,
    add_new,
    add_multiple_new,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import Texture.TextureID
import core.Builtin
import core.Vector2

## A parallax map describing surface details.
ParallaxMap : {
    height_map_texture_id : Texture.TextureID.TextureID,
    displacement_scale : F64,
    uv_per_distance : Vector2.Vector2,
}

new : Texture.TextureID.TextureID, F64, Vector2.Vector2 -> ParallaxMap
new = |height_map_texture_id, displacement_scale, uv_per_distance|
    { height_map_texture_id, displacement_scale, uv_per_distance }

add_new : Entity.ComponentData, Texture.TextureID.TextureID, F64, Vector2.Vector2 -> Entity.ComponentData
add_new = |entity_data, height_map_texture_id, displacement_scale, uv_per_distance|
    add(entity_data, new(height_map_texture_id, displacement_scale, uv_per_distance))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Texture.TextureID.TextureID), Entity.Arg.Broadcasted (F64), Entity.Arg.Broadcasted (Vector2.Vector2) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, height_map_texture_id, displacement_scale, uv_per_distance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            height_map_texture_id, displacement_scale, uv_per_distance,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [ParallaxMap] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, ParallaxMap -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ParallaxMap] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ParallaxMap) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ParallaxMap.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ParallaxMap -> List U8
write_packet = |bytes, val|
    type_id = 13523383454192306898
    size = 24
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ParallaxMap -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13523383454192306898
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

## Serializes a value of [ParallaxMap] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ParallaxMap -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Texture.TextureID.write_bytes(value.height_map_texture_id)
    |> Builtin.write_bytes_f64(value.displacement_scale)
    |> Vector2.write_bytes(value.uv_per_distance)

## Deserializes a value of [ParallaxMap] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ParallaxMap _
from_bytes = |bytes|
    Ok(
        {
            height_map_texture_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Texture.TextureID.from_bytes?,
            displacement_scale: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            uv_per_distance: bytes |> List.sublist({ start: 16, len: 8 }) |> Vector2.from_bytes?,
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
