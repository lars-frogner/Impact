# Hash: 1c12ab6c814784bbc140cf6ddf4b34e022fb0ccfe8101450a4c9d9e08d3831e1
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::motion::components::LogsKineticEnergy
# Type category: Component
# Commit: d505d37
module [
    LogsKineticEnergy,
    add,
    add_multiple,
]

import Entity
import core.Builtin

## Marker [`Component`](impact_ecs::component::Component) for entities whose
## translational and rotational kinetic energy should be written to the log at
## each time step.
LogsKineticEnergy : {}

## Adds a value of the [LogsKineticEnergy] component to an entity's data.
## Note that an entity never should have more than a single value of
## the same component type.
add : Entity.Data, LogsKineticEnergy -> Entity.Data
add = |data, value|
    data |> Entity.append_component(write_packet, value)

## Adds multiple values of the [LogsKineticEnergy] component to the data of
## a set of entities of the same archetype's data.
## Note that the number of values should match the number of entities
## in the set and that an entity never should have more than a single
## value of the same component type.
add_multiple : Entity.MultiData, List LogsKineticEnergy -> Entity.MultiData
add_multiple = |data, values|
    data |> Entity.append_components(write_multi_packet, values)

write_packet : List U8, LogsKineticEnergy -> List U8
write_packet = |bytes, value|
    type_id = 10262101972912963312
    size = 0
    alignment = 1
    bytes
    |> List.reserve(24 + size)
    |> Builtin.write_bytes_u64(type_id)
    |> Builtin.write_bytes_u64(size)
    |> Builtin.write_bytes_u64(alignment)
    |> write_bytes(value)

write_multi_packet : List U8, List LogsKineticEnergy -> List U8
write_multi_packet = |bytes, values|
    type_id = 10262101972912963312
    size = 0
    alignment = 1
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

## Serializes a value of [LogsKineticEnergy] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LogsKineticEnergy -> List U8
write_bytes = |bytes, _value|
    bytes

## Deserializes a value of [LogsKineticEnergy] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LogsKineticEnergy _
from_bytes = |_bytes|
    Ok({})

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 0 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
