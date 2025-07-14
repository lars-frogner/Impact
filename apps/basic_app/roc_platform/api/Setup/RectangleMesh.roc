# Hash: 0b7d697aee56d4e3a304fa9ac32f6c52ff61abcbae136dd4b9cbb8da0897bd79
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_mesh::setup::RectangleMesh
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    RectangleMesh,
    unit_square,
    new,
    add_unit_square,
    add_multiple_unit_square,
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

## A mesh consisting of an axis-aligned horizontal rectangle centered on
## the origin, whose front face is on the positive y side.
RectangleMesh : {
    ## The extent of the rectangle in the x-direction.
    extent_x : F32,
    ## The extent of the rectangle in the z-direction.
    extent_z : F32,
}

unit_square : RectangleMesh
unit_square = { extent_x: 1.0, extent_z: 1.0 }

add_unit_square : Entity.Data -> Entity.Data
add_unit_square = |entity_data|
    add(entity_data, unit_square)

add_multiple_unit_square : Entity.MultiData -> Entity.MultiData
add_multiple_unit_square = |entity_data|
    res = add_multiple(
        entity_data,
        Same(unit_square)
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in RectangleMesh.add_multiple_unit_square: ${Inspect.to_str(err)}"


## Defines a a rectangle mesh with the given horizontal extents.
new : F32, F32 -> RectangleMesh
new = |extent_x, extent_z|
    { extent_x, extent_z }

## Defines a a rectangle mesh with the given horizontal extents.
## Adds the component to the given entity's data.
add_new : Entity.Data, F32, F32 -> Entity.Data
add_new = |entity_data, extent_x, extent_z|
    add(entity_data, new(extent_x, extent_z))

## Defines a a rectangle mesh with the given horizontal extents.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (F32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, extent_x, extent_z|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            extent_x, extent_z,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [RectangleMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, RectangleMesh -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [RectangleMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (RectangleMesh) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in RectangleMesh.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, RectangleMesh -> List U8
write_packet = |bytes, val|
    type_id = 1976238558371830470
    size = 8
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List RectangleMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 1976238558371830470
    size = 8
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

## Serializes a value of [RectangleMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RectangleMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(value.extent_x)
    |> Builtin.write_bytes_f32(value.extent_z)

## Deserializes a value of [RectangleMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RectangleMesh _
from_bytes = |bytes|
    Ok(
        {
            extent_x: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            extent_z: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
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
