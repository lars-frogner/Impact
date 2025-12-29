# Hash: e538caabae1ff090
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_mesh::setup::ScreenFillingQuadMesh
# Type category: Component
module [
    ScreenFillingQuadMesh,
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

## A mesh consisting of two triangles that exactly fill the screen in clip space.
ScreenFillingQuadMesh : {}

## Creates a new screen-filling quad mesh.
new : {} -> ScreenFillingQuadMesh
new = |{}|
    {}

## Creates a new screen-filling quad mesh.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData -> Entity.ComponentData
add_new = |entity_data|
    add(entity_data)

## Creates a new screen-filling quad mesh.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_new = |entity_data|
    add_multiple(entity_data)

## Adds the [ScreenFillingQuadMesh] component to an entity's data.
add : Entity.ComponentData -> Entity.ComponentData
add = |entity_data|
    entity_data |> Entity.append_component(write_packet, {})

## Adds the [ScreenFillingQuadMesh] component to each entity's data.
add_multiple : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple = |entity_data|
    res = entity_data
        |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(Same({}), Entity.multi_count(entity_data)))
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in ScreenFillingQuadMesh.add_multiple: ${Inspect.to_str(err)}"

write_packet : List U8, ScreenFillingQuadMesh -> List U8
write_packet = |bytes, val|
    type_id = 2764229785939298167
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ScreenFillingQuadMesh -> List U8
write_multi_packet = |bytes, vals|
    type_id = 2764229785939298167
    size = 0
    alignment = 1
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

## Serializes a value of [ScreenFillingQuadMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ScreenFillingQuadMesh -> List U8
write_bytes = |bytes, _value|
    bytes

## Deserializes a value of [ScreenFillingQuadMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ScreenFillingQuadMesh _
from_bytes = |_bytes|
    Ok({})

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 0 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
