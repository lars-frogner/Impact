# Hash: c5279ab57d8be770a66b75cd25dfd99382435a75b7c68a384cb37cef6cacaa91
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::UniformColorComp
# Type category: Component
# Commit: d505d37
module [
    UniformColor,
    iron,
    copper,
    brass,
    gold,
    aluminum,
    silver,
    add_iron,
    add_copper,
    add_brass,
    add_gold,
    add_aluminum,
    add_silver,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a uniform base color.
##
## The base color affects the color and amount of light reflected and emitted
## by the material in a way that depends on the material's conductive
## properties. For dielectric materials, the base color is equivalent to the
## material's the albedo (the proportion of incident light diffusely
## reflected by the material). For metallic materials, the base color affects
## the material's specular reflectance. For emissive materials, the base color
## affects the material's emissive luminance.
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
UniformColor : Vector3.Vector3 Binary32

iron : UniformColor
iron = (0.562, 0.565, 0.578)

add_iron : Entity.Data -> Entity.Data
add_iron = |data|
    add(data, iron)

copper : UniformColor
copper = (0.955, 0.638, 0.538)

add_copper : Entity.Data -> Entity.Data
add_copper = |data|
    add(data, copper)

brass : UniformColor
brass = (0.910, 0.778, 0.423)

add_brass : Entity.Data -> Entity.Data
add_brass = |data|
    add(data, brass)

gold : UniformColor
gold = (1.000, 0.782, 0.344)

add_gold : Entity.Data -> Entity.Data
add_gold = |data|
    add(data, gold)

aluminum : UniformColor
aluminum = (0.913, 0.922, 0.924)

add_aluminum : Entity.Data -> Entity.Data
add_aluminum = |data|
    add(data, aluminum)

silver : UniformColor
silver = (0.972, 0.960, 0.915)

add_silver : Entity.Data -> Entity.Data
add_silver = |data|
    add(data, silver)

## Adds a value of the [UniformColor] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformColor -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UniformColor] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UniformColor -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UniformColor -> List U8
write_packet = |bytes, value|
    type_id = 798125558113623870
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UniformColor -> List U8
write_multi_packet = |bytes, values|
    type_id = 798125558113623870
    size = 12
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

## Serializes a value of [UniformColor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformColor -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Vector3.write_bytes_32(value)

## Deserializes a value of [UniformColor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformColor _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
