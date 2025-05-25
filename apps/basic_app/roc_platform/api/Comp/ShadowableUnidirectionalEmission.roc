# Hash: e5fa6000030e3771d2f3e13711fc3a2278cd259f3caabf83453885d52e0133dc
# Generated: 2025-05-23T21:48:57+00:00
# Rust type: impact::light::components::ShadowableUnidirectionalEmissionComp
# Type category: Component
# Commit: 31f3514 (dirty)
module [
    ShadowableUnidirectionalEmission,
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
import core.Builtin
import core.Degrees
import core.UnitVector3
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that emit light
## in a single direction. The light can be shadowed (use
## [`UnidirectionalEmissionComp`] for light without shadows).
ShadowableUnidirectionalEmission : {
    ## The illuminance (incident flux per area) of an illuminated surface
    ## perpendicular to the light direction.
    ##
    ## # Unit
    ## Lux (lx = lm/m²)
    perpendicular_illuminance : Vector3.Vector3 Binary32,
    ## The direction of the emitted light.
    direction : UnitVector3.UnitVector3 Binary32,
    ## The angular extent of the light source, which determines the extent of
    ## specular highlights and the softness of shadows.
    angular_source_extent : Degrees.Degrees Binary32,
}

## Creates a new shadowable unidirectional emission component with the
## given perpendicular illuminance (in lux), direction, and angular
## source extent.
new : Vector3.Vector3 Binary32, UnitVector3.UnitVector3 Binary32, Degrees.Degrees Binary32 -> ShadowableUnidirectionalEmission
new = |perpendicular_illuminance, direction, angular_source_extent|
    { perpendicular_illuminance, direction, angular_source_extent }

## Creates a new shadowable unidirectional emission component with the
## given perpendicular illuminance (in lux), direction, and angular
## source extent.
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary32, UnitVector3.UnitVector3 Binary32, Degrees.Degrees Binary32 -> Entity.Data
add_new = |entity_data, perpendicular_illuminance, direction, angular_source_extent|
    add(entity_data, new(perpendicular_illuminance, direction, angular_source_extent))

## Creates a new shadowable unidirectional emission component with the
## given perpendicular illuminance (in lux), direction, and angular
## source extent.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32), Entity.Arg.Broadcasted (UnitVector3.UnitVector3 Binary32), Entity.Arg.Broadcasted (Degrees.Degrees Binary32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, perpendicular_illuminance, direction, angular_source_extent|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map3(
            perpendicular_illuminance, direction, angular_source_extent,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [ShadowableUnidirectionalEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableUnidirectionalEmission -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ShadowableUnidirectionalEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ShadowableUnidirectionalEmission) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ShadowableUnidirectionalEmission.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ShadowableUnidirectionalEmission -> List U8
write_packet = |bytes, val|
    type_id = 11687163781022109071
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ShadowableUnidirectionalEmission -> List U8
write_multi_packet = |bytes, vals|
    type_id = 11687163781022109071
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

## Serializes a value of [ShadowableUnidirectionalEmission] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ShadowableUnidirectionalEmission -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(28)
    |> Vector3.write_bytes_32(value.perpendicular_illuminance)
    |> UnitVector3.write_bytes_32(value.direction)
    |> Degrees.write_bytes_32(value.angular_source_extent)

## Deserializes a value of [ShadowableUnidirectionalEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ShadowableUnidirectionalEmission _
from_bytes = |bytes|
    Ok(
        {
            perpendicular_illuminance: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
            direction: bytes |> List.sublist({ start: 12, len: 12 }) |> UnitVector3.from_bytes_32?,
            angular_source_extent: bytes |> List.sublist({ start: 24, len: 4 }) |> Degrees.from_bytes_32?,
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
