# Hash: cf69b2dd03df200f2ad5d4fbb1602a0daf8c84bc97adfe682ddfd8d5728970b3
# Generated: 2025-12-21T23:04:45+00:00
# Rust type: impact_physics::force::local_force::LocalForce
# Type category: Component
# Commit: d4c84c05 (dirty)
module [
    LocalForce,
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
import core.Point3
import core.Vector3

## A constant force vector and the point where it is applied, all in the
## body-fixed frame.
LocalForce : {
    ## The force vector in the body-fixed frame.
    force : Vector3.Vector3,
    ## The point where the force is applied, in the body's model space.
    point : Point3.Point3,
}

new : Vector3.Vector3, Point3.Point3 -> LocalForce
new = |force, point|
    { force, point }

add_new : Entity.ComponentData, Vector3.Vector3, Point3.Point3 -> Entity.ComponentData
add_new = |entity_data, force, point|
    add(entity_data, new(force, point))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3), Entity.Arg.Broadcasted (Point3.Point3) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, force, point|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            force, point,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [LocalForce] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, LocalForce -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [LocalForce] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (LocalForce) -> Result Entity.MultiComponentData Str
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
    size = 24
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List LocalForce -> List U8
write_multi_packet = |bytes, vals|
    type_id = 16434524121723371577
    size = 24
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

## Serializes a value of [LocalForce] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LocalForce -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Vector3.write_bytes(value.force)
    |> Point3.write_bytes(value.point)

## Deserializes a value of [LocalForce] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LocalForce _
from_bytes = |bytes|
    Ok(
        {
            force: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes?,
            point: bytes |> List.sublist({ start: 12, len: 12 }) |> Point3.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
