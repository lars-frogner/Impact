# Hash: b92ad097e9b8eec5bcf9227396920cc1749f0e22bfc73d571a96aa5b725db7f5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::components::SphereMeshComp
# Type category: Component
# Commit: d505d37
module [
    SphereMesh,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities whose mesh is a unit diameter sphere centered on the origin.
##
## The purpose of this component is to aid in constructing a [`MeshComp`] for
## the entity. It is therefore not kept after entity creation.
SphereMesh : {
    ## The number of horizontal circular cross-sections of vertices making up
    ## the sphere. The number of vertices comprising each ring is proportional
    ## to `n_rings`, resulting in an approximately uniform resolution.
    n_rings : U32,
}

## Creates a new component for a sphere mesh with the given number of
## rings.
new : U32 -> SphereMesh
new = |n_rings|
    { n_rings }

## Creates a new component for a sphere mesh with the given number of
## rings.
## Adds the component to the given entity's data.
add_new : Entity.Data, U32 -> Entity.Data
add_new = |data, n_rings|
    add(data, new(n_rings))

## Adds a value of the [SphereMesh] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SphereMesh -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [SphereMesh] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List SphereMesh -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, SphereMesh -> List U8
write_packet = |bytes, value|
    type_id = 3326404544739324621
    size = 4
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List SphereMesh -> List U8
write_multi_packet = |bytes, values|
    type_id = 3326404544739324621
    size = 4
    alignment = 4
    count = List.len(values)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    values
    |> List.walk(
        bytes_with_header,
        |bts, value| bts |> write_bytes(value),
    )

## Serializes a value of [SphereMesh] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SphereMesh -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Builtin.write_bytes_u32(value.n_rings)

## Deserializes a value of [SphereMesh] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SphereMesh _
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
