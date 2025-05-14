# Hash: 5c7da0f3042590201eea55032e30844b068c7f93103cd8ff7201f8bdf6de2afb
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::light::components::ShadowableUnidirectionalEmissionComp
# Type category: Component
# Commit: d505d37
module [
    ShadowableUnidirectionalEmission,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
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
add_new = |data, perpendicular_illuminance, direction, angular_source_extent|
    add(data, new(perpendicular_illuminance, direction, angular_source_extent))

## Adds a value of the [ShadowableUnidirectionalEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableUnidirectionalEmission -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [ShadowableUnidirectionalEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List ShadowableUnidirectionalEmission -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, ShadowableUnidirectionalEmission -> List U8
write_packet = |bytes, value|
    type_id = 11687163781022109071
    size = 28
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List ShadowableUnidirectionalEmission -> List U8
write_multi_packet = |bytes, values|
    type_id = 11687163781022109071
    size = 28
    alignment = 4
    count = List.len(values)
    bytes_with_header =
        bytes
        |> List.reserve(32 + size * count)
        |> Builtin.write_bytes_u64(type_id)
        |> Builtin.write_bytes_u64(size)
        |> Builtin.write_bytes_u64(alignment)
        |> Builtin.write_bytes_u64(count)
    values
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
