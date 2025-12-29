# Hash: 02f1747dae98463f
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact_light::UnidirectionalEmission
# Type category: Component
module [
    UnidirectionalEmission,
    new,
    add_new,
    add_multiple_new,
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
import core.Degrees
import core.UnitVector3
import core.Vector3

## Emission of light in a single direction. The light can not be shadowed
## (use [`ShadowableUnidirectionalEmission`] for light with shadows).
UnidirectionalEmission : {
    ## The illuminance (incident flux per area) of an illuminated surface
    ## perpendicular to the light direction.
    ##
    ## # Unit
    ## Lux (lx = lm/m²)
    perpendicular_illuminance : Vector3.Vector3,
    ## The direction of the emitted light.
    direction : UnitVector3.UnitVector3,
    ## The angular extent of the light source, which determines the extent of
    ## specular highlights.
    angular_source_extent : Degrees.Degrees,
}

## Creates a new unidirectional emission component with the given
## perpendicular illuminance (in lux), direction, and angular
## source extent.
new : Vector3.Vector3, UnitVector3.UnitVector3, Degrees.Degrees -> UnidirectionalEmission
new = |perpendicular_illuminance, direction, angular_source_extent|
    { perpendicular_illuminance, direction, angular_source_extent }

## Creates a new unidirectional emission component with the given
## perpendicular illuminance (in lux), direction, and angular
## source extent.
## Adds the component to the given entity's data.
add_new : Entity.ComponentData, Vector3.Vector3, UnitVector3.UnitVector3, Degrees.Degrees -> Entity.ComponentData
add_new = |entity_data, perpendicular_illuminance, direction, angular_source_extent|
    add(entity_data, new(perpendicular_illuminance, direction, angular_source_extent))

## Creates a new unidirectional emission component with the given
## perpendicular illuminance (in lux), direction, and angular
## source extent.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiComponentData, Entity.Arg.Broadcasted (Vector3.Vector3), Entity.Arg.Broadcasted (UnitVector3.UnitVector3), Entity.Arg.Broadcasted (Degrees.Degrees) -> Result Entity.MultiComponentData Str
add_multiple_new = |entity_data, perpendicular_illuminance, direction, angular_source_extent|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            perpendicular_illuminance, direction, angular_source_extent,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [UnidirectionalEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.ComponentData, UnidirectionalEmission -> Entity.ComponentData
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [UnidirectionalEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiComponentData, Entity.Arg.Broadcasted (UnidirectionalEmission) -> Result Entity.MultiComponentData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in UnidirectionalEmission.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

## The ID of the [UnidirectionalEmission] component.
component_id = 4263202137654376205

## Adds the ID of the [UnidirectionalEmission] component to the component list.
add_component_id : Entity.ComponentIds -> Entity.ComponentIds
add_component_id = |component_ids|
    component_ids |> Entity.append_component_id(component_id)

## Reads the component from the given entity data. 
read : Entity.ComponentData -> Result UnidirectionalEmission Str
read = |data|
    Entity.read_component(data, component_id, from_bytes)
    |> Result.map_err(
        |err|
            when err is
                ComponentMissing -> "No UnidirectionalEmission component in data"
                Decode(decode_err) -> "Failed to decode UnidirectionalEmission component: ${Inspect.to_str(decode_err)}",
    )

## Fetches the value of this component for the given entity.
get_for_entity! : Entity.Id => Result UnidirectionalEmission Str
get_for_entity! = |entity_id|
    Entity.get_component!(entity_id, component_id)? |> read

## Sets the value of this component for the given entity to the
## specified value.
set_for_entity! : UnidirectionalEmission, Entity.Id => Result {} Str
set_for_entity! = |value, entity_id|
    Entity.new_component_data |> add(value) |> Entity.update!(entity_id)

write_packet : List U8, UnidirectionalEmission -> List U8
write_packet = |bytes, val|
    type_id = 4263202137654376205
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List UnidirectionalEmission -> List U8
write_multi_packet = |bytes, vals|
    type_id = 4263202137654376205
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

## Serializes a value of [UnidirectionalEmission] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, UnidirectionalEmission -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Vector3.write_bytes(value.perpendicular_illuminance)
    |> UnitVector3.write_bytes(value.direction)
    |> Degrees.write_bytes(value.angular_source_extent)

## Deserializes a value of [UnidirectionalEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result UnidirectionalEmission _
from_bytes = |bytes|
    Ok(
        {
            perpendicular_illuminance: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes?,
            direction: bytes |> List.sublist({ start: 12, len: 12 }) |> UnitVector3.from_bytes?,
            angular_source_extent: bytes |> List.sublist({ start: 24, len: 4 }) |> Degrees.from_bytes?,
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
