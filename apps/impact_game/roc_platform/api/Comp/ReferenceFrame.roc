# Hash: ea189b568582e40e
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact_geometry::reference_frame::ReferenceFrame
# Type category: Component
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
    component_id,
    add_component_id,
    read,
    get_for_entity!,
    set_for_entity!,
    write_bytes,
    from_bytes,
]

import Entity
import Entity.Arg
import core.Builtin
import core.Point3
import core.UnitQuaternion

## A reference frame defined by an origin position and an orientation.
ReferenceFrame : {
    ## The coordinates of the origin of the entity's reference frame measured
    ## in the parent space.
    position : Point3.Point3,
    ## The 3D orientation of the entity's reference frame in the parent space.
    orientation : UnitQuaternion.UnitQuaternion,
}

## Creates a new reference frame with the given position and orientation.
new : Point3.Point3, UnitQuaternion.UnitQuaternion -> ReferenceFrame
new = |position, orientation|
    { position, orientation }

## Creates a new reference frame with the given position and orientation.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, Point3.Point3, UnitQuaternion.UnitQuaternion -> Entity.ComponentData
add_new = |entity_data, position, orientation|
    add(entity_data, new(position, orientation))

## Creates a new reference frame with the given position and orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Point3.Point3), Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion) -> Result Entity.MultiComponentData Str
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
unoriented : Point3.Point3 -> ReferenceFrame
unoriented = |position|
    new(position, UnitQuaternion.identity)

## Creates a new reference frame with the given position and the identity
## orientation.
## Adds the component to the given entity's data.
add_unoriented : Entity.ComponentData, Point3.Point3 -> Entity.ComponentData
add_unoriented = |entity_data, position|
    add(entity_data, unoriented(position))

## Creates a new reference frame with the given position and the identity
## orientation.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unoriented : Entity.MultiComponentData, Entity.Arg.Broadcasted (Point3.Point3) -> Result Entity.MultiComponentData Str
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
unlocated : UnitQuaternion.UnitQuaternion -> ReferenceFrame
unlocated = |orientation|
    new(Point3.origin, orientation)

## Creates a new reference frame with the given orientation, located at the
## origin.
## Adds the component to the given entity's data.
add_unlocated : Entity.ComponentData, UnitQuaternion.UnitQuaternion -> Entity.ComponentData
add_unlocated = |entity_data, orientation|
    add(entity_data, unlocated(orientation))

## Creates a new reference frame with the given orientation, located at the
## origin.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_unlocated : Entity.MultiComponentData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion) -> Result Entity.MultiComponentData Str
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
add : Entity.ComponentData, ReferenceFrame -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ReferenceFrame] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (ReferenceFrame) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ReferenceFrame.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [ReferenceFrame] component.
component_id = 13511111226856695413

## Adds the ID of the [ReferenceFrame] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result ReferenceFrame Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No ReferenceFrame component in data"
                Decode(decode_err) -> "Failed to decode ReferenceFrame component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result ReferenceFrame Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : ReferenceFrame, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, ReferenceFrame -> List U8
write_packet = |bytes, val|
    type_id = 13511111226856695413
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ReferenceFrame -> List U8
write_multi_packet = |bytes, vals|
    type_id = 13511111226856695413
    size = 28
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

## Serializes a value of [ReferenceFrame] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ReferenceFrame -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Point3.write_bytes(value.position)
    |> UnitQuaternion.write_bytes(value.orientation)

## Deserializes a value of [ReferenceFrame] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ReferenceFrame _
from_bytes = |bytes|
    Ok(
        {
            position: bytes |> List.sublist({ start: 0, len: 12 }) |> Point3.from_bytes?,
            orientation: bytes |> List.sublist({ start: 12, len: 16 }) |> UnitQuaternion.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 28 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
