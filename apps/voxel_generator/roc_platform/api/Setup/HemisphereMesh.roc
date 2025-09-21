# Hash: fa8196c08500edb69f30cced7cecad04095ab9934da2a0c75205a1d4d57d622f
# Generated: 2025-09-20T12:42:13+00:00
# Rust type: impact_mesh::setup::HemisphereMesh
# Type category: Component
# Commit: f9b55709 (dirty)
module [
    HemisphereMesh,
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
import core.Builtin

## A mesh consisting of a unit diameter hemisphere whose disk lies in the
## xz-plane and is centered on the origin.
HemisphereMesh : {
    ## The number of horizontal circular cross-sections of vertices making up
    ## the hemisphere. The number of vertices comprising each ring is
    ## proportional to `n_rings`, resulting in an approximately uniform
    ## resolution.
    n_rings : U32,
}

## Defines a hemisphere mesh with the given number of rings.
new : U32 -> HemisphereMesh
new = |n_rings|
    { n_rings }

## Defines a hemisphere mesh with the given number of rings.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, U32 -> Entity.ComponentData
add_new = |entity_data, n_rings|
    add(entity_data, new(n_rings))

## Defines a hemisphere mesh with the given number of rings.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (U32) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, n_rings|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            n_rings,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [HemisphereMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, HemisphereMesh -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [HemisphereMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (HemisphereMesh) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in HemisphereMesh.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, HemisphereMesh -> List U8
write_packet = |bytes, val|
    type_id = 389361481597160827
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List HemisphereMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 389361481597160827
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

## Serializes a value of [HemisphereMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, HemisphereMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_u32(value.n_rings)

## Deserializes a value of [HemisphereMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result HemisphereMesh _
from_bytes = |bytes|
    Ok(
        {
            n_rings: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
        },
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
