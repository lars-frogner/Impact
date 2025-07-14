# Hash: bfbee28cea6da98ccb283270f45c5e267373b02112ac80e4c6de6a2d5e48145b
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_light::ShadowableOmnidirectionalEmission
# Type category: Component
# Commit: b1b4dfd8 (dirty)
module [
    ShadowableOmnidirectionalEmission,
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
import core.Vector3

## Uniform emission of light in all directions. The light can be shadowed
## (use [`OmnidirectionalEmission`] for light without shadows).
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
add_new = |entity_data, luminous_intensity, source_extent|
    add(entity_data, new(luminous_intensity, source_extent))

## Creates a new shadowable omnidirectional emission component with
## the given luminous intensity (in candela) and source extent.
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32), Entity.Arg.Broadcasted (F32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, luminous_intensity, source_extent|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map2(
            luminous_intensity, source_extent,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [ShadowableOmnidirectionalEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, ShadowableOmnidirectionalEmission -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [ShadowableOmnidirectionalEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (ShadowableOmnidirectionalEmission) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in ShadowableOmnidirectionalEmission.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, ShadowableOmnidirectionalEmission -> List U8
write_packet = |bytes, val|
    type_id = 6126578492634658920
    size = 16
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List ShadowableOmnidirectionalEmission -> List U8
write_multi_packet = |bytes, vals|
    type_id = 6126578492634658920
    size = 16
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
