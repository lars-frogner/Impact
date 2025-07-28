# Hash: c00f6db2f9c8c63cf96883e12d6d29d39af73c752f2e39b5ce731e09ee2c2069
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_material::setup::physical::UniformRoughness
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    UniformRoughness,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin

## A uniform surface roughness. The roughness ranges from zero (perfectly
## smooth) to one (completely diffuse).
UniformRoughness : F32

## Adds a value of the [UniformRoughness] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, UniformRoughness -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UniformRoughness] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (UniformRoughness) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UniformRoughness.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, UniformRoughness -> List U8
write_packet = |bytes, val|
    type_id = 9043020506446719557
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UniformRoughness -> List U8
write_multi_packet = |bytes, vals|
    type_id = 9043020506446719557
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

## Serializes a value of [UniformRoughness] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UniformRoughness -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_f32(value)

## Deserializes a value of [UniformRoughness] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UniformRoughness _
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
