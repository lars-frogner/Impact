# Hash: 667a524763e942c7
# Generated: 2026-01-16T08:12:29.620936497
# Rust type: impact_controller::orientation::AngularVelocityControl
# Type category: Component
module [
    AngularVelocityControl,
    all_directions,
    new,
    new_local,
    add_all_directions,
    add_multiple_all_directions,
    add_new,
    add_multiple_new,
    add_new_local,
    add_multiple_new_local,
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

import Control.AngularVelocityControlDirections
import Control.AngularVelocityControlFlags
import Entity
import Entity.Arg
import core.Builtin
import core.UnitQuaternion

## User control of angular velocity.
AngularVelocityControl : {
    ## The orientation of the reference frame in which the controls should
    ## be applied. This maps the local control directions to world-space
    ## directions.
    frame_orientation : UnitQuaternion.UnitQuaternion,
    ## Restrict control to these directions for applicable controllers.
    directions : Control.AngularVelocityControlDirections.AngularVelocityControlDirections,
    ## Flags for how to control angular velocity.
    flags : Control.AngularVelocityControlFlags.AngularVelocityControlFlags,
}

all_directions : {} -> AngularVelocityControl
all_directions = |{}|
    {
        frame_orientation: UnitQuaternion.identity,
        directions: Control.AngularVelocityControlDirections.all,
        flags: Control.AngularVelocityControlFlags.empty,
    }

add_all_directions : Entity.ComponentData -> Entity.ComponentData
add_all_directions = |entity_data|
    add(entity_data, all_directions({}))

add_multiple_all_directions : Entity.MultiComponentData -> Entity.MultiComponentData
add_multiple_all_directions = |entity_data|
    res = add_multiple(
        entity_data,
        Same(all_directions({}))
    )
    when res is
        Ok(res_data) -> res_data
        Err(err) -> crash "unexpected error in AngularVelocityControl.add_multiple_all_directions: ${Inspect.to_str(err)}"

new : Control.AngularVelocityControlDirections.AngularVelocityControlDirections, Control.AngularVelocityControlFlags.AngularVelocityControlFlags -> AngularVelocityControl
new = |directions, flags|
    { frame_orientation: UnitQuaternion.identity, directions, flags }

add_new : Entity.ComponentData, Control.AngularVelocityControlDirections.AngularVelocityControlDirections, Control.AngularVelocityControlFlags.AngularVelocityControlFlags -> Entity.ComponentData
add_new = |entity_data, directions, flags|
    add(entity_data, new(directions, flags))

add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Control.AngularVelocityControlDirections.AngularVelocityControlDirections), Entity.Arg.Broadcasted (Control.AngularVelocityControlFlags.AngularVelocityControlFlags) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, directions, flags|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            directions, flags,
            Entity.multi_count(entity_data),
            new
        ))
    )

new_local : UnitQuaternion.UnitQuaternion, Control.AngularVelocityControlDirections.AngularVelocityControlDirections, Control.AngularVelocityControlFlags.AngularVelocityControlFlags -> AngularVelocityControl
new_local = |frame_orientation, directions, flags|
    { frame_orientation, directions, flags }

add_new_local : Entity.ComponentData, UnitQuaternion.UnitQuaternion, Control.AngularVelocityControlDirections.AngularVelocityControlDirections, Control.AngularVelocityControlFlags.AngularVelocityControlFlags -> Entity.ComponentData
add_new_local = |entity_data, frame_orientation, directions, flags|
    add(entity_data, new_local(frame_orientation, directions, flags))

add_multiple_new_local : Entity.MultiComponentData, Entity.Arg.Broadcasted (UnitQuaternion.UnitQuaternion), Entity.Arg.Broadcasted (Control.AngularVelocityControlDirections.AngularVelocityControlDirections), Entity.Arg.Broadcasted (Control.AngularVelocityControlFlags.AngularVelocityControlFlags) -> Result Entity.MultiComponentData Str
add_multiple_new_local = |entity_data, frame_orientation, directions, flags|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            frame_orientation, directions, flags,
            Entity.multi_count(entity_data),
            new_local
        ))
    )

## Adds a value of the [AngularVelocityControl] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, AngularVelocityControl -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [AngularVelocityControl] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (AngularVelocityControl) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in AngularVelocityControl.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [AngularVelocityControl] component.
component_id = 698327266232627508

## Adds the ID of the [AngularVelocityControl] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result AngularVelocityControl Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No AngularVelocityControl component in data"
                Decode(decode_err) -> "Failed to decode AngularVelocityControl component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result AngularVelocityControl Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : AngularVelocityControl, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, AngularVelocityControl -> List U8
write_packet = |bytes, val|
    type_id = 698327266232627508
    size = 24
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List AngularVelocityControl -> List U8
write_multi_packet = |bytes, vals|
    type_id = 698327266232627508
    size = 24
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

## Serializes a value of [AngularVelocityControl] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AngularVelocityControl -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> UnitQuaternion.write_bytes(value.frame_orientation)
    |> Control.AngularVelocityControlDirections.write_bytes(value.directions)
    |> Control.AngularVelocityControlFlags.write_bytes(value.flags)

## Deserializes a value of [AngularVelocityControl] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AngularVelocityControl _
from_bytes = |bytes|
    Ok(
        {
            frame_orientation: bytes |> List.sublist({ start: 0, len: 16 }) |> UnitQuaternion.from_bytes?,
            directions: bytes |> List.sublist({ start: 16, len: 4 }) |> Control.AngularVelocityControlDirections.from_bytes?,
            flags: bytes |> List.sublist({ start: 20, len: 4 }) |> Control.AngularVelocityControlFlags.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
