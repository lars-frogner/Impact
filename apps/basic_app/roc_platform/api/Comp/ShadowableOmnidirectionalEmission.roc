# Hash: 70b10d7e318d7e8aa3ac15e8dddfe20212607392af5959c5b1ebadb3673f1672
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::light::components::ShadowableOmnidirectionalEmissionComp
# Type category: Component
# Commit: d505d37
module [
    ShadowableOmnidirectionalEmission,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that emit light
## uniformly in all directions. The light can be shadowed (use
## [`OmnidirectionalEmissionComp`] for light without shadows).
ShadowableOmnidirectionalEmission : {
    ## The luminous intensity of the emitted light.
    ##
    ## # Unit
    ## Candela (cd = lm/sr)
    luminous_intensity : Vector3.Vector3 Binary32,
    ## The physical extent of the light source, which determines the extent of
    ## specular highlights and the softness of shadows.
    ##
    ## # Unit
    ## Meter (m)
    source_extent : F32,
}

## Creates a new shadowable omnidirectional emission component with
## the given luminous intensity (in candela) and source extent.
new : Vector3.Vector3 Binary32, F32 -> ShadowableOmnidirectionalEmission
new = |luminous_intensity, source_extent|
    { luminous_intensity, source_extent }

## Creates a new shadowable omnidirectional emission component with
## the given luminous intensity (in candela) and source extent.
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary32, F32 -> Entity.Data
add_new = |data, luminous_intensity, source_extent|
    add(data, new(luminous_intensity, source_extent))

## Adds a value of the [ShadowableOmnidirectionalEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableOmnidirectionalEmission -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [ShadowableOmnidirectionalEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List ShadowableOmnidirectionalEmission -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, ShadowableOmnidirectionalEmission -> List U8
write_packet = |bytes, value|
    type_id = 7325642505044986640
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List ShadowableOmnidirectionalEmission -> List U8
write_multi_packet = |bytes, values|
    type_id = 7325642505044986640
    size = 16
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

## Serializes a value of [ShadowableOmnidirectionalEmission] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ShadowableOmnidirectionalEmission -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Vector3.write_bytes_32(value.luminous_intensity)
    |> Builtin.write_bytes_f32(value.source_extent)

## Deserializes a value of [ShadowableOmnidirectionalEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ShadowableOmnidirectionalEmission _
from_bytes = |bytes|
    Ok(
        {
            luminous_intensity: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
            source_extent: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
