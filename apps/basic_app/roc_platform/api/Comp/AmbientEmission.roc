# Hash: af8f68b0f10bffc95d78a89df8b6d276e642cb0189e3d521911aea02e649e1d8
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::light::components::AmbientEmissionComp
# Type category: Component
# Commit: d505d37
module [
    AmbientEmission,
    new,
    add_new,
    add,
    add_multiple,
]

import Entity
import core.Builtin
import core.Vector3

## [`Component`](impact_ecs::component::Component) for entities that produce a
## spatially uniform and isotropic (ambient) light field.
AmbientEmission : {
    ## The illuminance (incident flux per area) of a surface due to the ambient
    ## emission.
    ##
    ## # Unit
    ## Lux (lx = lm/m²)
    illuminance : Vector3.Vector3 Binary32,
}

## Creates a new ambient light emission component with the given
## illuminance (in lux).
new : Vector3.Vector3 Binary32 -> AmbientEmission
new = |illuminance|
    { illuminance }

## Creates a new ambient light emission component with the given
## illuminance (in lux).
## Adds the component to the given entity's data.
add_new : Entity.Data, Vector3.Vector3 Binary32 -> Entity.Data
add_new = |data, illuminance|
    add(data, new(illuminance))

## Adds a value of the [AmbientEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, AmbientEmission -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [AmbientEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List AmbientEmission -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, AmbientEmission -> List U8
write_packet = |bytes, value|
    type_id = 10976986305333878027
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List AmbientEmission -> List U8
write_multi_packet = |bytes, values|
    type_id = 10976986305333878027
    size = 12
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

## Serializes a value of [AmbientEmission] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, AmbientEmission -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(12)
    |> Vector3.write_bytes_32(value.illuminance)

## Deserializes a value of [AmbientEmission] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result AmbientEmission _
from_bytes = |bytes|
    Ok(
        {
            illuminance: bytes |> List.sublist({ start: 0, len: 12 }) |> Vector3.from_bytes_32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 12 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
