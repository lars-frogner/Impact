# Hash: 9a3134c00172a8e0ad08f6c29a87dd899f5f40baedce4df639a174b893659049
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_light::AmbientEmission
# Type category: Component
# Commit: 397d36d3 (dirty)
module [
    AmbientEmission,
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

## A spatially uniform and isotropic (ambient) light field.
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
add_new = |entity_data, illuminance|
    add(entity_data, new(illuminance))

## Creates a new ambient light emission component with the given
## illuminance (in lux).
## Adds multiple values of the component to the data of
## a set of entities of the same archetype's data.
add_multiple_new : Entity.MultiData, Entity.Arg.Broadcasted (Vector3.Vector3 Binary32) -> Result Entity.MultiData Str
add_multiple_new = |entity_data, illuminance|
    add_multiple(
        entity_data,
        All(Entity.Arg.broadcasted_map1(
            illuminance,
            Entity.multi_count(entity_data),
            new
        ))
    )

## Adds a value of the [AmbientEmission] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, AmbientEmission -> Entity.Data
add = |entity_data, comp_value|
    entity_data |> Entity.append_component(write_packet, comp_value)

## Adds multiple values of the [AmbientEmission] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, Entity.Arg.Broadcasted (AmbientEmission) -> Result Entity.MultiData Str
add_multiple = |entity_data, comp_values|
    entity_data
    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))
    |> Result.map_err(
        |CountMismatch(new_count, orig_count)|
            "Got ${Inspect.to_str(new_count)} values in AmbientEmission.add_multiple, expected ${Inspect.to_str(orig_count)}",
    )

write_packet : List U8, AmbientEmission -> List U8
write_packet = |bytes, val|
    type_id = 643851755473699111
    size = 12
    alignment = 4
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(val)

write_multi_packet : List U8, List AmbientEmission -> List U8
write_multi_packet = |bytes, vals|
    type_id = 643851755473699111
    size = 12
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
