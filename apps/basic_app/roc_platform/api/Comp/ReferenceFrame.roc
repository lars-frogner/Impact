# Hash: 71f0e091253da07e255faeacccf072e9b31c345e9609b2ad98563d210b90cf88
# Generated: 2025-07-15T17:32:17+00:00
# Rust type: impact_geometry::reference_frame::ReferenceFrame
# Type category: Component
# Commit: 1fbb6f6b (dirty)
module [
    ReferenceFrame,
    new,
    unoriented,
    unlocated,
    add_new,
    add_multiple_new,
    add_unoriented,
    add_multiple_unoriented,
    add_unlocated,
    add_multiple_unlocated,
    add,
    add_multiple,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.UnitQuaternion

## A reference frame defined an origin position, an orientation and a scale
## factor, as well as an internal offset for displacing the origin within
## the reference frame.
ReferenceFrame : {
    ## The coordinates of the origin of the entity's reference frame measured
    ## in the parent space.
    position : Point3.Point3 Binary64,
    ## The 3D orientation of the entity's reference frame in the parent space.
    orientation : UnitQuaternion.UnitQuaternion Binary64,
}

## Creates a new reference frame with the given position and orientation.
new : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
new = |position, orientation|
    { position, orientation }

## Creates a new reference frame with the given position and orientation.
## Adds the component to the given entity's data.
add_new : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_new = |entity_data, position, orientation|
    add(entity_data, new(position, orientation))

## Creates a new reference frame with the given position and orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, orientation,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates a new reference frame with the given position and the identity
## orientation.
unoriented : Point3.Point3 Binary64 -> ReferenceFrame
unoriented = |position|
    new(position, UnitQuaternion.identity)

## Creates a new reference frame with the given position and the identity
## orientation.
## Adds the component to the given entity's data.
add_unoriented : Entity.Data, Point3.Point3 Binary64 -> Entity.Data
add_unoriented = |entity_data, position|
    add(entity_data, unoriented(position))

## Creates a new reference frame with the given position and the identity
## orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_unoriented = |entity_data, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            position,
            Entity.multi_count(entity_data),
            unoriented
        ))
    )

## Creates a new reference frame with the given orientation, located at the
## origin.
unlocated : UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
unlocated = |orientation|
    new(Point3.origin, orientation)

## Creates a new reference frame with the given orientation, located at the
## origin.
## Adds the component to the given entity's data.
add_unlocated : Entity.Data, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_unlocated = |entity_data, orientation|
    add(entity_data, unlocated(orientation))

## Creates a new reference frame with the given orientation, located at the
## origin.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unlocated : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_unlocated = |entity_data, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            orientation,
            Entity.multi_count(entity_data),
            unlocated
        ))
    )

## Adds a value of the [ReferenceFrame] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ReferenceFrame -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ReferenceFrame] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ReferenceFrame) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ReferenceFrame.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ReferenceFrame -> List U8
write_packet = |bytes, val|
    type_id = 13511111226856695413
    size = 56
    alignment = 8
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ReferenceFrame -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13511111226856695413
    size = 56
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

## Serializes a value of [ReferenceFrame] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ReferenceFrame -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(56)
    |> Point3.write_bytes_64(value.position)
    |> UnitQuaternion.write_bytes_64(value.orientation)

## Deserializes a value of [ReferenceFrame] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ReferenceFrame _
from_bytes = |bytes|
    Ok(
        {
            position: bytes |> List.sublist({ start: 0, len: 24 }) |> Point3.from_bytes_64?,
            orientation: bytes |> List.sublist({ start: 24, len: 32 }) |> UnitQuaternion.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 56 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
