# Hash: 79cfecccb03167b0
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_physics::collision::setup::PlanarCollidable
# Type category: Component
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
    kind : U32,
    plane : Plane.Plane,
    response_params : Physics.ContactResponseParameters.ContactResponseParameters,
}

new : Physics.CollidableKind.CollidableKind, Plane.Plane, Physics.ContactResponseParameters.ContactResponseParameters -> PlanarCollidable
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

add_new : Entity.ComponentData, Physics.CollidableKind.CollidableKind, Plane.Plane, Physics.ContactResponseParameters.ContactResponseParameters -> Entity.ComponentData
add_new = |entity_data, kind, plane, response_params|
    add(entity_data, new(kind, plane, response_params))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Physics.CollidableKind.CollidableKind), Entity.Arg.Broadcasted (Plane.Plane), Entity.Arg.Broadcasted (Physics.ContactResponseParameters.ContactResponseParameters) -> Result Entity.MultiComponentData Str
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
add : Entity.ComponentData, PlanarCollidable -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [PlanarCollidable] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (PlanarCollidable) -> Result Entity.MultiComponentData Str
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
    size = 32
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List PlanarCollidable -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13177454990089127351
    size = 32
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

## Serializes a value of [PlanarCollidable] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlanarCollidable -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_u32(value.kind)
    |> Plane.write_bytes(value.plane)
    |> Physics.ContactResponseParameters.write_bytes(value.response_params)

## Deserializes a value of [PlanarCollidable] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlanarCollidable _
from_bytes = |bytes|
    Ok(
        {
            kind: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_u32?,
            plane: bytes |> List.sublist({ start: 4, len: 16 }) |> Plane.from_bytes?,
            response_params: bytes |> List.sublist({ start: 20, len: 12 }) |> Physics.ContactResponseParameters.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
