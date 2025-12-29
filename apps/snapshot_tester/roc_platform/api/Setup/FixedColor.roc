# Hash: 3e895b64b3f53c54
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_material::setup::fixed::FixedColor
# Type category: Component
module [
    FixedColor,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Vector3

## A fixed, uniform color that is independent of lighting.
FixedColor : Vector3.Vector3

## Adds a value of the [FixedColor] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, FixedColor -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [FixedColor] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (FixedColor) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in FixedColor.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, FixedColor -> List U8
write_packet = |bytes, val|
    type_id = 10940777262413544777
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List FixedColor -> List U8
write_multi_packet = |bytes, vals|
    type_id = 10940777262413544777
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

## Serializes a value of [FixedColor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FixedColor -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Vector3.write_bytes(value)

## Deserializes a value of [FixedColor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FixedColor _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes?,
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
