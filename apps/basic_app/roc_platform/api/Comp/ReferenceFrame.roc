# Hash: ad24fbb2ecd7f645f434377ccaa83183834e6f1d12bbb35a827823109e965480
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_geometry::reference_frame::ReferenceFrame
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    ReferenceFrame,
    new,
    unscaled,
    unoriented,
    unoriented_scaled,
    unlocated,
    unlocated_scaled,
    scaled,
    unoriented_with_offset_origin,
    unoriented_scaled_with_offset_origin,
    scaled_with_offset_origin,
    with_offset_origin,
    add_new,
    add_multiple_new,
    add_unscaled,
    add_multiple_unscaled,
    add_unoriented,
    add_multiple_unoriented,
    add_unoriented_scaled,
    add_multiple_unoriented_scaled,
    add_unlocated,
    add_multiple_unlocated,
    add_unlocated_scaled,
    add_multiple_unlocated_scaled,
    add_scaled,
    add_multiple_scaled,
    add_unoriented_with_offset_origin,
    add_multiple_unoriented_with_offset_origin,
    add_unoriented_scaled_with_offset_origin,
    add_multiple_unoriented_scaled_with_offset_origin,
    add_scaled_with_offset_origin,
    add_multiple_scaled_with_offset_origin,
    add_with_offset_origin,
    add_multiple_with_offset_origin,
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
import core.Vector3

## A reference frame defined an origin position, an orientation and a scale
## factor, as well as an internal offset for displacing the origin within
## the reference frame.
ReferenceFrame : {
    ## The offset, expressed in the entity's reference frame (before scaling),
    ## from the original origin of the entity's reference frame to the point
    ## that should be used as the actual origin.
    origin_offset : Vector3.Vector3 Binary64,
    ## The coordinates of the origin of the entity's reference frame measured
    ## in the parent space.
    position : Point3.Point3 Binary64,
    ## The 3D orientation of the entity's reference frame in the parent space.
    orientation : UnitQuaternion.UnitQuaternion Binary64,
    ## The uniform scale factor of the entity's reference frame (distance in
    ## world space per distance in the reference frame).
    scaling : F64,
}

## Creates a new reference frame with the given position, orientation and
## scaling, retaining the original origin of the entity's reference frame.
new : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
new = |position, orientation, scaling|
    { origin_offset: Vector3.zero, position, orientation, scaling }

## Creates a new reference frame with the given position, orientation and
## scaling, retaining the original origin of the entity's reference frame.
## Adds the component to the given entity's data.
add_new : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_new = |entity_data, position, orientation, scaling|
    add(entity_data, new(position, orientation, scaling))

## Creates a new reference frame with the given position, orientation and
## scaling, retaining the original origin of the entity's reference frame.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, position, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            position, orientation, scaling,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Creates a new reference frame with the given position and orientation,
## retaining the original origin of the entity's reference frame and no
## scaling.
unscaled : Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
unscaled = |position, orientation|
    new(position, orientation, 1.0)

## Creates a new reference frame with the given position and orientation,
## retaining the original origin of the entity's reference frame and no
## scaling.
## Adds the component to the given entity's data.
add_unscaled : Entity.Data, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_unscaled = |entity_data, position, orientation|
    add(entity_data, unscaled(position, orientation))

## Creates a new reference frame with the given position and orientation,
## retaining the original origin of the entity's reference frame and no
## scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unscaled : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_unscaled = |entity_data, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, orientation,
            Entity.multi_count(entity_data),
            unscaled
        ))
    )

## Creates a new reference frame with the given position, retaining the
## original origin of the entity's reference frame and the identity
## orientation and scaling.
unoriented : Point3.Point3 Binary64 -> ReferenceFrame
unoriented = |position|
    unoriented_scaled(position, 1.0)

## Creates a new reference frame with the given position, retaining the
## original origin of the entity's reference frame and the identity
## orientation and scaling.
## Adds the component to the given entity's data.
add_unoriented : Entity.Data, Point3.Point3 Binary64 -> Entity.Data
add_unoriented = |entity_data, position|
    add(entity_data, unoriented(position))

## Creates a new reference frame with the given position, retaining the
## original origin of the entity's reference frame and the identity
## orientation and scaling.
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

## Creates a new reference frame with the given position and scaling,
## retaining the original origin of the entity's reference frame and the
## identity orientation.
unoriented_scaled : Point3.Point3 Binary64, F64 -> ReferenceFrame
unoriented_scaled = |position, scaling|
    new(position, UnitQuaternion.identity, scaling)

## Creates a new reference frame with the given position and scaling,
## retaining the original origin of the entity's reference frame and the
## identity orientation.
## Adds the component to the given entity's data.
add_unoriented_scaled : Entity.Data, Point3.Point3 Binary64, F64 -> Entity.Data
add_unoriented_scaled = |entity_data, position, scaling|
    add(entity_data, unoriented_scaled(position, scaling))

## Creates a new reference frame with the given position and scaling,
## retaining the original origin of the entity's reference frame and the
## identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_scaled : Entity.MultiData, Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unoriented_scaled = |entity_data, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            position, scaling,
            Entity.multi_count(entity_data),
            unoriented_scaled
        ))
    )

## Creates a new reference frame with the given orientation, retaining the
## original origin of the entity's reference frame and located at the
## origin with no scaling.
unlocated : UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
unlocated = |orientation|
    unlocated_scaled(orientation, 1.0)

## Creates a new reference frame with the given orientation, retaining the
## original origin of the entity's reference frame and located at the
## origin with no scaling.
## Adds the component to the given entity's data.
add_unlocated : Entity.Data, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_unlocated = |entity_data, orientation|
    add(entity_data, unlocated(orientation))

## Creates a new reference frame with the given orientation, retaining the
## original origin of the entity's reference frame and located at the
## origin with no scaling.
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

## Creates a new reference frame with the given orientation and scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin.
unlocated_scaled : UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
unlocated_scaled = |orientation, scaling|
    new(Point3.origin, orientation, scaling)

## Creates a new reference frame with the given orientation and scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin.
## Adds the component to the given entity's data.
add_unlocated_scaled : Entity.Data, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_unlocated_scaled = |entity_data, orientation, scaling|
    add(entity_data, unlocated_scaled(orientation, scaling))

## Creates a new reference frame with the given orientation and scaling,
## retaining the original origin of the entity's reference frame and
## located at the origin.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unlocated_scaled : Entity.MultiData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unlocated_scaled = |entity_data, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            orientation, scaling,
            Entity.multi_count(entity_data),
            unlocated_scaled
        ))
    )

## Creates a new reference frame with the given scaling, retaining the
## original origin of the entity's reference frame and located at the
## origin with the identity orientation.
scaled : F64 -> ReferenceFrame
scaled = |scaling|
    unoriented_scaled(Point3.origin, scaling)

## Creates a new reference frame with the given scaling, retaining the
## original origin of the entity's reference frame and located at the
## origin with the identity orientation.
## Adds the component to the given entity's data.
add_scaled : Entity.Data, F64 -> Entity.Data
add_scaled = |entity_data, scaling|
    add(entity_data, scaled(scaling))

## Creates a new reference frame with the given scaling, retaining the
## original origin of the entity's reference frame and located at the
## origin with the identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_scaled : Entity.MultiData, Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_scaled = |entity_data, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            scaling,
            Entity.multi_count(entity_data),
            scaled
        ))
    )

## Creates a new reference frame with the given origin offset and position,
## and with the identity orientation and scaling.
unoriented_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> ReferenceFrame
unoriented_with_offset_origin = |origin_offset, position|
    unoriented_scaled_with_offset_origin(origin_offset, position, 1.0)

## Creates a new reference frame with the given origin offset and position,
## and with the identity orientation and scaling.
## Adds the component to the given entity's data.
add_unoriented_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64 -> Entity.Data
add_unoriented_with_offset_origin = |entity_data, origin_offset, position|
    add(entity_data, unoriented_with_offset_origin(origin_offset, position))

## Creates a new reference frame with the given origin offset and position,
## and with the identity orientation and scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64) -> Result Entity.MultiData Str
add_multiple_unoriented_with_offset_origin = |entity_data, origin_offset, position|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            origin_offset, position,
            Entity.multi_count(entity_data),
            unoriented_with_offset_origin
        ))
    )

## Creates a new reference frame with the given origin offset, position and
## scaling, and with the identity orientation.
unoriented_scaled_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> ReferenceFrame
unoriented_scaled_with_offset_origin = |origin_offset, position, scaling|
    scaled_with_offset_origin(origin_offset, position, UnitQuaternion.identity, scaling)

## Creates a new reference frame with the given origin offset, position and
## scaling, and with the identity orientation.
## Adds the component to the given entity's data.
add_unoriented_scaled_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, F64 -> Entity.Data
add_unoriented_scaled_with_offset_origin = |entity_data, origin_offset, position, scaling|
    add(entity_data, unoriented_scaled_with_offset_origin(origin_offset, position, scaling))

## Creates a new reference frame with the given origin offset, position and
## scaling, and with the identity orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented_scaled_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_unoriented_scaled_with_offset_origin = |entity_data, origin_offset, position, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, position, scaling,
            Entity.multi_count(entity_data),
            unoriented_scaled_with_offset_origin
        ))
    )

## Creates a new reference frame with the given origin offset, position
## orientation, and scaling.
scaled_with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> ReferenceFrame
scaled_with_offset_origin = |origin_offset, position, orientation, scaling|
    { origin_offset, position, orientation, scaling }

## Creates a new reference frame with the given origin offset, position
## orientation, and scaling.
## Adds the component to the given entity's data.
add_scaled_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64, F64 -> Entity.Data
add_scaled_with_offset_origin = |entity_data, origin_offset, position, orientation, scaling|
    add(entity_data, scaled_with_offset_origin(origin_offset, position, orientation, scaling))

## Creates a new reference frame with the given origin offset, position
## orientation, and scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_scaled_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64), Entity.Arg.Broadcasted (F64) -> Result Entity.MultiData Str
add_multiple_scaled_with_offset_origin = |entity_data, origin_offset, position, orientation, scaling|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map4(
            origin_offset, position, orientation, scaling,
            Entity.multi_count(entity_data),
            scaled_with_offset_origin
        ))
    )

## Creates a new reference frame with the given origin offset, position and
## orientation and no scaling.
with_offset_origin : Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> ReferenceFrame
with_offset_origin = |origin_offset, position, orientation|
    scaled_with_offset_origin(origin_offset, position, orientation, 1.0)

## Creates a new reference frame with the given origin offset, position and
## orientation and no scaling.
## Adds the component to the given entity's data.
add_with_offset_origin : Entity.Data, Vector3.Vector3 Binary64, Point3.Point3 Binary64, UnitQuaternion.UnitQuaternion Binary64 -> Entity.Data
add_with_offset_origin = |entity_data, origin_offset, position, orientation|
    add(entity_data, with_offset_origin(origin_offset, position, orientation))

## Creates a new reference frame with the given origin offset, position and
## orientation and no scaling.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_with_offset_origin : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary64), Entity.Arg.Broadcasted (Point3.Point3 Binary64), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion Binary64) -> Result Entity.MultiData Str
add_multiple_with_offset_origin = |entity_data, origin_offset, position, orientation|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            origin_offset, position, orientation,
            Entity.multi_count(entity_data),
            with_offset_origin
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
    size = 88
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
    size = 88
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
    |> List.reserve(88)
    |> Vector3.write_bytes_64(value.origin_offset)
    |> Point3.write_bytes_64(value.position)
    |> UnitQuaternion.write_bytes_64(value.orientation)
    |> Builtin.write_bytes_f64(value.scaling)

## Deserializes a value of [ReferenceFrame] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ReferenceFrame _
from_bytes = |bytes|
    Ok(
        {
            origin_offset: bytes |> List.sublist({ start: 0, len: 24 }) |> Vector3.from_bytes_64?,
            position: bytes |> List.sublist({ start: 24, len: 24 }) |> Point3.from_bytes_64?,
            orientation: bytes |> List.sublist({ start: 48, len: 32 }) |> UnitQuaternion.from_bytes_64?,
            scaling: bytes |> List.sublist({ start: 80, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 88 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
