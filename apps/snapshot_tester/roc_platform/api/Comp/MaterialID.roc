# Hash: 42448d09a618d4fe30bf9b0f247861f8f504a0cbecc7ebb63acdc67232121134
# Generated: 2025-08-01T06:54:20+00:00
# Rust type: impact_material::MaterialID
# Type category: Component
# Commit: 5cd592d6 (dirty)
module [
    MaterialID,
    from_name,
    add_from_name,
    add_multiple_from_name,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Hashing

## Identifier for a material.
MaterialID : Hashing.StringHash64

## Creates a material ID hashed from the given name.
from_name : Str -> MaterialID
from_name = |name|
    Hashing.hash_str_64(name)

## Creates a material ID hashed from the given name.
## Adds the component to the given entity's data.
add_from_name : Entity.Data, Str -> Entity.Data
add_from_name = |entity_data, name|
    add(entity_data, from_name(name))

## Creates a material ID hashed from the given name.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_from_name : Entity.MultiData, Entity.Arg.Broadcasted (Str) -> Result Entity.MultiData Str
add_multiple_from_name = |entity_data, name|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            name,
            Entity.multi_count(entity_data),
            from_name
        ))
    )

## Adds a value of the [MaterialID] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, MaterialID -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [MaterialID] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (MaterialID) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in MaterialID.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, MaterialID -> List U8
write_packet = |bytes, val|
    type_id = 4484412674557634465
    size = 8
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List MaterialID -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4484412674557634465
    size = 8
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

## Serializes a value of [MaterialID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MaterialID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [MaterialID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MaterialID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Hashing.from_bytes_string_hash_64?,
        ),
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
