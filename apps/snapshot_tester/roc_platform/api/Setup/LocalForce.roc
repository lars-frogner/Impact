# Hash: 396e5ad76f7b55899ea8eac105318476ee41700123bbda27290faa6f22e5a122
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_physics::force::local_force::LocalForce
# Type category: Component
# Commit: 189570ab (dirty)
module [
    LocalForce,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.Vector3

## A constant force vector and the point where it is applied, all in the body's
## local reference frame.
LocalForce : {
    ## The force vector in the body's local reference frame.
    force : Vector3.Vector3 Binary64,
    ## The point where the force is applied, in the body's local reference
    ## frame.
    point : Point3.Point3 Binary64,
}

## Adds a value of the [LocalForce] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, LocalForce -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [LocalForce] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (LocalForce) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in LocalForce.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, LocalForce -> List U8
write_packet = |bytes, val|
    type_id = 16434524121723371577
    size = 48
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List LocalForce -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16434524121723371577
    size = 48
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

## Serializes a value of [LocalForce] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LocalForce -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> Vector3.write_bytes_64(value.force)
    |> Point3.write_bytes_64(value.point)

## Deserializes a value of [LocalForce] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LocalForce _
from_bytes = |bytes|
    Ok(
        {
            force: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            point: bytes |> List.sublist({ start: 24, len: 24 }) |> Point3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 48 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
