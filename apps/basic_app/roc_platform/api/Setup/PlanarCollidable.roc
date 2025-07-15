# Hash: c57f8598e08afed73d8447e5ab4f3e564588739114f430b23bc035ea6a1fef35
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_physics::collision::setup::PlanarCollidable
# Type category: Component
# Commit: 189570ab (dirty)
module [
    PlanarCollidable,
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
import Physics.ContactResponseParameters
import core.Builtin
import core.Plane

## A planar collidable.
PlanarCollidable : {
    kind : U64,
    plane : Plane.Plane Binary64,
    response_params : Physics.ContactResponseParameters.ContactResponseParameters,
}

new : Physics.CollidableKind.CollidableKind, Plane.Plane Binary64, Physics.ContactResponseParameters.ContactResponseParameters -> PlanarCollidable
new = |kind, plane, response_params|
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        plane,
        response_params,
    }

add_new : Entity.Data, Physics.CollidableKind.CollidableKind, Plane.Plane Binary64, Physics.ContactResponseParameters.ContactResponseParameters -> Entity.Data
add_new = |entity_data, kind, plane, response_params|
    add(entity_data, new(kind, plane, response_params))

add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Physics.CollidableKind.CollidableKind), Entity.Arg.Broadcasted (Plane.Plane Binary64), Entity.Arg.Broadcasted (Physics.ContactResponseParameters.ContactResponseParameters) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, kind, plane, response_params|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            kind, plane, response_params,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [PlanarCollidable] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, PlanarCollidable -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [PlanarCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (PlanarCollidable) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in PlanarCollidable.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, PlanarCollidable -> List U8
write_packet = |bytes, val|
    type_id = 13177454990089127351
    size = 64
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List PlanarCollidable -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13177454990089127351
    size = 64
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

## Serializes a value of [PlanarCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlanarCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(64)
    |> Builtin.write_bytes_u64(value.kind)
    |> Plane.write_bytes_64(value.plane)
    |> Physics.ContactResponseParameters.write_bytes(value.response_params)

## Deserializes a value of [PlanarCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlanarCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_u64?,
            plane: bytes |> List.sublist({ start: 8, len: 32 }) |> Plane.from_bytes_64?,
            response_params: bytes |> List.sublist({ start: 40, len: 24 }) |> Physics.ContactResponseParameters.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 64 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
