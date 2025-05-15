# Hash: 47c49091b8bf20842bea1a14fefb3bb303d2b270de7525ee308633b8e855e0f7
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::UniformMetalnessComp
# Type category: Component
# Commit: d505d37
module [
    UniformMetalness,
    dielectric,
    metal,
    add_dielectric,
    add_metal,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a uniform metalness.
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
UniformMetalness : F32

dielectric : UniformMetalness
dielectric = 0.0

add_dielectric : Entity.Data -> Entity.Data
add_dielectric = |data|
    add(data, dielectric)

metal : UniformMetalness
metal = 1.0

add_metal : Entity.Data -> Entity.Data
add_metal = |data|
    add(data, metal)

## Adds a value of the [UniformMetalness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformMetalness -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UniformMetalness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UniformMetalness -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UniformMetalness -> List U8
write_packet = |bytes, value|
    type_id = 16657324603404166233
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UniformMetalness -> List U8
write_multi_packet = |bytes, values|
    type_id = 16657324603404166233
    size = 4
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
