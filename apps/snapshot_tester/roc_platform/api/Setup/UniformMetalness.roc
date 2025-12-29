# Hash: f48c29b5d65dc823
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_material::setup::physical::UniformMetalness
# Type category: Component
module [
    UniformMetalness,
    dielectric,
    metal,
    add_dielectric,
    add_multiple_dielectric,
    add_metal,
    add_multiple_metal,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## A uniform metalness.
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
UniformMetalness : F32

dielectric : UniformMetalness
dielectric = 0.0

add_dielectric : Entity.ComponentData -> Entity.ComponentData
add_dielectric = |entity_data|
    add(entity_data, dielectric)

add_multiple_dielectric : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_dielectric = |entity_data|
    res = add_multiple(
        entity_data,
        Same(dielectric)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformMetalness.add_multiple_dielectric: ${Inspect.to_str(err)}"


metal : UniformMetalness
metal = 1.0

add_metal : Entity.ComponentData -> Entity.ComponentData
add_metal = |entity_data|
    add(entity_data, metal)

add_multiple_metal : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_metal = |entity_data|
    res = add_multiple(
        entity_data,
        Same(metal)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformMetalness.add_multiple_metal: ${Inspect.to_str(err)}"


## Adds a value of the [UniformMetalness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, UniformMetalness -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UniformMetalness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (UniformMetalness) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UniformMetalness.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, UniformMetalness -> List U8
write_packet = |bytes, val|
    type_id = 12654226026169816215
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UniformMetalness -> List U8
write_multi_packet = |bytes, vals|
    type_id = 12654226026169816215
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

## Serializes a value of [UniformMetalness] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformMetalness -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value)

## Deserializes a value of [UniformMetalness] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformMetalness _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
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
