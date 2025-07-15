# Hash: 79cdc619304a612c32beb8d6567c4908ce885276c1bf1dafeee44a72d64ba2b0
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_material::setup::physical::UniformColor
# Type category: Component
# Commit: 189570ab (dirty)
module [
    UniformColor,
    iron,
    copper,
    brass,
    gold,
    aluminum,
    silver,
    add_iron,
    add_multiple_iron,
    add_copper,
    add_multiple_copper,
    add_brass,
    add_multiple_brass,
    add_gold,
    add_multiple_gold,
    add_aluminum,
    add_multiple_aluminum,
    add_silver,
    add_multiple_silver,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## A uniform base color.
##
## The base color affects the color and amount of light reflected and emitted
## by the material in a way that depends on the material's conductive
## properties. For dielectric materials, the base color is equivalent to the
## material's the albedo (the proportion of incident light diffusely
## reflected by the material). For metallic materials, the base color affects
## the material's specular reflectance. For emissive materials, the base color
## affects the material's emissive luminance.
UniformColor : Vector3.Vector3 Binary32

iron : UniformColor
iron = (0.562, 0.565, 0.578)

add_iron : Entity.Data -> Entity.Data
add_iron = |entity_data|
    add(entity_data, iron)

add_multiple_iron : Entity.MultiData -> Entity.MultiData
add_multiple_iron = |entity_data|
    res = add_multiple(
        entity_data,
        Same(iron)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_iron: ${Inspect.to_str(err)}"


copper : UniformColor
copper = (0.955, 0.638, 0.538)

add_copper : Entity.Data -> Entity.Data
add_copper = |entity_data|
    add(entity_data, copper)

add_multiple_copper : Entity.MultiData -> Entity.MultiData
add_multiple_copper = |entity_data|
    res = add_multiple(
        entity_data,
        Same(copper)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_copper: ${Inspect.to_str(err)}"


brass : UniformColor
brass = (0.910, 0.778, 0.423)

add_brass : Entity.Data -> Entity.Data
add_brass = |entity_data|
    add(entity_data, brass)

add_multiple_brass : Entity.MultiData -> Entity.MultiData
add_multiple_brass = |entity_data|
    res = add_multiple(
        entity_data,
        Same(brass)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_brass: ${Inspect.to_str(err)}"


gold : UniformColor
gold = (1.000, 0.782, 0.344)

add_gold : Entity.Data -> Entity.Data
add_gold = |entity_data|
    add(entity_data, gold)

add_multiple_gold : Entity.MultiData -> Entity.MultiData
add_multiple_gold = |entity_data|
    res = add_multiple(
        entity_data,
        Same(gold)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_gold: ${Inspect.to_str(err)}"


aluminum : UniformColor
aluminum = (0.913, 0.922, 0.924)

add_aluminum : Entity.Data -> Entity.Data
add_aluminum = |entity_data|
    add(entity_data, aluminum)

add_multiple_aluminum : Entity.MultiData -> Entity.MultiData
add_multiple_aluminum = |entity_data|
    res = add_multiple(
        entity_data,
        Same(aluminum)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_aluminum: ${Inspect.to_str(err)}"


silver : UniformColor
silver = (0.972, 0.960, 0.915)

add_silver : Entity.Data -> Entity.Data
add_silver = |entity_data|
    add(entity_data, silver)

add_multiple_silver : Entity.MultiData -> Entity.MultiData
add_multiple_silver = |entity_data|
    res = add_multiple(
        entity_data,
        Same(silver)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in UniformColor.add_multiple_silver: ${Inspect.to_str(err)}"


## Adds a value of the [UniformColor] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformColor -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UniformColor] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (UniformColor) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UniformColor.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, UniformColor -> List U8
write_packet = |bytes, val|
    type_id = 1241797728352198472
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UniformColor -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1241797728352198472
    size = 12
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
