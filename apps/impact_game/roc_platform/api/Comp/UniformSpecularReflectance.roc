# Hash: 9ead67f108814026ea55196297cf350a961959d1b5a6c18cab80cb11639a04fa
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::material::components::UniformSpecularReflectanceComp
# Type category: Component
# Commit: d505d37
module [
    UniformSpecularReflectance,
    metal,
    water,
    skin,
    living_tissue,
    fabric,
    stone,
    plastic,
    glass,
    in_range_of,
    add_metal,
    add_water,
    add_skin,
    add_in_range_of,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a uniform scalar specular reflectance at normal incidence
## (the proportion of incident light specularly reflected by the material when
## the light direction is perpendicular to the surface).
##
## The purpose of this component is to aid in constructing a [`MaterialComp`]
## for the entity. It is therefore not kept after entity creation.
UniformSpecularReflectance : F32

metal : UniformSpecularReflectance
metal = 1.0

add_metal : Entity.Data -> Entity.Data
add_metal = |data|
    add(data, metal)

water : UniformSpecularReflectance
water = 0.02

add_water : Entity.Data -> Entity.Data
add_water = |data|
    add(data, water)

skin : UniformSpecularReflectance
skin = 0.028

add_skin : Entity.Data -> Entity.Data
add_skin = |data|
    add(data, skin)

living_tissue : (F32, F32)
living_tissue = (0.02, 0.04)

fabric : (F32, F32)
fabric = (0.04, 0.056)

stone : (F32, F32)
stone = (0.035, 0.056)

plastic : (F32, F32)
plastic = (0.04, 0.05)

glass : (F32, F32)
glass = (0.04, 0.05)

in_range_of : (F32, F32), F32 -> UniformSpecularReflectance
in_range_of = |range, percentage|
    range.0 + 0.01 * percentage * (range.1 - range.0)

add_in_range_of : Entity.Data, (F32, F32), F32 -> Entity.Data
add_in_range_of = |data, range, percentage|
    add(data, in_range_of(range, percentage))

## Adds a value of the [UniformSpecularReflectance] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformSpecularReflectance -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [UniformSpecularReflectance] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List UniformSpecularReflectance -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, UniformSpecularReflectance -> List U8
write_packet = |bytes, value|
    type_id = 12412346638566824456
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List UniformSpecularReflectance -> List U8
write_multi_packet = |bytes, values|
    type_id = 12412346638566824456
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

## Serializes a value of [UniformSpecularReflectance] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformSpecularReflectance -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value)

## Deserializes a value of [UniformSpecularReflectance] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformSpecularReflectance _
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
