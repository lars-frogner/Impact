# Hash: cab3393331fbca08918a7f73a95644809492571fe99c1d82bf65ebc169ffbbb6
# Generated: 2025-07-15T17:32:43+00:00
# Rust type: impact_material::setup::physical::UniformSpecularReflectance
# Type category: Component
# Commit: 1fbb6f6b (dirty)
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
    add_multiple_metal,
    add_water,
    add_multiple_water,
    add_skin,
    add_multiple_skin,
    add_in_range_of,
    add_multiple_in_range_of,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## A uniform scalar specular reflectance at normal incidence (the
## proportion of incident light specularly reflected by the material when
## the light direction is perpendicular to the surface).
UniformSpecularReflectance : F32

metal : UniformSpecularReflectance
metal = 1.0

add_metal : Entity.Data -> Entity.Data
add_metal = |entity_data|
    add(entity_data, metal)

add_multiple_metal : Entity.MultiData -> Entity.MultiData
add_multiple_metal = |entity_data|
    res = add_multiple(
        entity_data,
        Same(metal)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformSpecularReflectance.add_multiple_metal: ${Inspect.to_str(err)}"


water : UniformSpecularReflectance
water = 0.02

add_water : Entity.Data -> Entity.Data
add_water = |entity_data|
    add(entity_data, water)

add_multiple_water : Entity.MultiData -> Entity.MultiData
add_multiple_water = |entity_data|
    res = add_multiple(
        entity_data,
        Same(water)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformSpecularReflectance.add_multiple_water: ${Inspect.to_str(err)}"


skin : UniformSpecularReflectance
skin = 0.028

add_skin : Entity.Data -> Entity.Data
add_skin = |entity_data|
    add(entity_data, skin)

add_multiple_skin : Entity.MultiData -> Entity.MultiData
add_multiple_skin = |entity_data|
    res = add_multiple(
        entity_data,
        Same(skin)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformSpecularReflectance.add_multiple_skin: ${Inspect.to_str(err)}"


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
add_in_range_of = |entity_data, range, percentage|
    add(entity_data, in_range_of(range, percentage))

add_multiple_in_range_of : Entity.MultiData, Entity.Arg.Broadcasted ((F32, F32)), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_in_range_of = |entity_data, range, percentage|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            range, percentage,
            Entity.multi_count(entity_data),
            in_range_of
        ))
    )

## Adds a value of the [UniformSpecularReflectance] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformSpecularReflectance -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UniformSpecularReflectance] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (UniformSpecularReflectance) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UniformSpecularReflectance.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, UniformSpecularReflectance -> List U8
write_packet = |bytes, val|
    type_id = 13172741666417303630
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UniformSpecularReflectance -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13172741666417303630
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
