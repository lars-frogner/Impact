# Hash: b46d15fad09b7344568970a35124162e88217b2271463f13f6f3c4a29bb79f2d
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::physics::collision::components::SphereCollidableComp
# Type category: Component
# Commit: ce2d27b (dirty)
module [
    SphereCollidable,
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
import Physics.CollidableKind
import core.Builtin
import core.Sphere

## [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
## entities that have a spherical collidable.
##
## The purpose of this component is to aid in constructing a [`CollidableComp`]
## for the entity. It is therefore not kept after entity creation.
SphereCollidable : {
    kind : U64,
    sphere : Sphere.Sphere Binary64,
}

new : Physics.CollidableKind.CollidableKind, Sphere.Sphere Binary64 -> SphereCollidable
new = |kind, sphere|
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        sphere,
    }

add_new : Entity.Data, Physics.CollidableKind.CollidableKind, Sphere.Sphere Binary64 -> Entity.Data
add_new = |entity_data, kind, sphere|
    add(entity_data, new(kind, sphere))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Physics.CollidableKind.CollidableKind), Entity.Arg.Broadcasted (Sphere.Sphere Binary64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, kind, sphere|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            kind, sphere,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [SphereCollidable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, SphereCollidable -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [SphereCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (SphereCollidable) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in SphereCollidable.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, SphereCollidable -> List U8
write_packet = |bytes, val|
    type_id = 11031774526575538057
    size = 40
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List SphereCollidable -> List U8
write_multi_packet = |bytes, vals|
    type_id = 11031774526575538057
    size = 40
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

## Serializes a value of [SphereCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SphereCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(40)
    |> Builtin.write_bytes_u64(value.kind)
    |> Sphere.write_bytes_64(value.sphere)

## Deserializes a value of [SphereCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SphereCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
            sphere: bytes |> List.sublist({ start: 8, len: 32 }) |> Sphere.from_bytes_64?,
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
