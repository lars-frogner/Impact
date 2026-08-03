# Hash: e85590ccade3e553
# Generated: 2026-08-03T13:31:40.248619334
# Rust type: impact_voxel::interaction::fracturing::FracturingProperties
# Type category: Component
module [
    FracturingProperties,
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

## Properties governing how an object fractures on impact.
FracturingProperties : {
    ## The minimum collisional force (assumed constant during the
    ## collision) required for an impact to cause fracturing.
    fracturing_force : F32,
    ## The collisional force per object surface area (approximated by the
    ## square of the object extent) required for the object to shatter (the
    ## fracturing radius reaching the characteristic size of the object).
    shattering_pressure : F32,
    ## The characteristic length scale of generated fragments, relative to
    ## the extent of the object.
    fragment_scale : F32,
    ## The target minimum extent of generated fragments. Will be converted
    ## to world units based on the object size such that the minimum
    ## fragment size in world space scale increases weakly with the object
    ## size.
    min_fragment_extent : F32,
    ## The target maximum extent of generated fragments, relative to the
    ## extent of the object.
    max_fragment_extent : F32,
}

## Adds a value of the [FracturingProperties] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, FracturingProperties -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [FracturingProperties] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (FracturingProperties) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in FracturingProperties.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [FracturingProperties] component.
component_id = 14765750072787839469

## Adds the ID of the [FracturingProperties] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result FracturingProperties Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No FracturingProperties component in data"
                Decode(decode_err) -> "Failed to decode FracturingProperties component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result FracturingProperties Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : FracturingProperties, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, FracturingProperties -> List U8
write_packet = |bytes, val|
    type_id = 14765750072787839469
    size = 20
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List FracturingProperties -> List U8
write_multi_packet = |bytes, vals|
    type_id = 14765750072787839469
    size = 20
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

## Serializes a value of [FracturingProperties] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FracturingProperties -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(20)
    |> Builtin.write_bytes_f32(value.fracturing_force)
    |> Builtin.write_bytes_f32(value.shattering_pressure)
    |> Builtin.write_bytes_f32(value.fragment_scale)
    |> Builtin.write_bytes_f32(value.min_fragment_extent)
    |> Builtin.write_bytes_f32(value.max_fragment_extent)

## Deserializes a value of [FracturingProperties] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FracturingProperties _
from_bytes = |bytes|
    Ok(
        {
            fracturing_force: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            shattering_pressure: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            fragment_scale: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            min_fragment_extent: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            max_fragment_extent: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 20 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
