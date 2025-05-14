# Hash: 2dda765a97d6b629d3e49c4e0ea1f4232f753a2d55ec981cbd22101a25a59dbc
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::collision::components::PlaneCollidableComp
# Type category: Component
# Commit: d505d37
module [
    PlaneCollidable,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import Physics.CollidableKind
import core.Builtin
import core.Plane

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a planar collidable.
##
## The purpose of this component is to aid in constructing a [`CollidableComp`]
## for the entity. It is therefore not kept after entity creation.
PlaneCollidable : {
    kind : U64,
    plane : Plane.Plane Binary64,
}

new : Physics.CollidableKind.CollidableKind, Plane.Plane Binary64 -> PlaneCollidable
new = |kind, plane|
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        plane,
    }

add_new : Entity.Data, Physics.CollidableKind.CollidableKind, Plane.Plane Binary64 -> Entity.Data
add_new = |data, kind, plane|
    add(data, new(kind, plane))

## Adds a value of the [PlaneCollidable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, PlaneCollidable -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [PlaneCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List PlaneCollidable -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, PlaneCollidable -> List U8
write_packet = |bytes, value|
    type_id = 474102428447958754
    size = 40
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List PlaneCollidable -> List U8
write_multi_packet = |bytes, values|
    type_id = 474102428447958754
    size = 40
    alignment = 8
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

## Serializes a value of [PlaneCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlaneCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(40)
    |> Builtin.write_bytes_u64(value.kind)
    |> Plane.write_bytes_64(value.plane)

## Deserializes a value of [PlaneCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlaneCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
            plane: bytes |> List.sublist({ start: 8, len: 32 }) |> Plane.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 40 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
