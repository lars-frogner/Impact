# Hash: 0632ec9f3370723e84f84d34cd8db06c123c5fce1ab4539cebe3653129a9adb5
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact::physics::command::ToSimulationSpeedMultiplier
# Type category: Inline
# Commit: 189570ab (dirty)
module [
    ToSimulationSpeedMultiplier,
    write_bytes,
    from_bytes,
]

import core.Builtin

ToSimulationSpeedMultiplier : [
    Higher,
    Lower,
    Specific F64,
]

## Serializes a value of [ToSimulationSpeedMultiplier] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToSimulationSpeedMultiplier -> List U8
write_bytes = |bytes, value|
    when value is
        Higher ->
            bytes
            |> List.reserve(9)
            |> List.append(0)
            |> List.concat(List.repeat(0, 8))

        Lower ->
            bytes
            |> List.reserve(9)
            |> List.append(1)
            |> List.concat(List.repeat(0, 8))

        Specific(val) ->
            bytes
            |> List.reserve(9)
            |> List.append(2)
            |> Builtin.write_bytes_f64(val)

## Deserializes a value of [ToSimulationSpeedMultiplier] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToSimulationSpeedMultiplier _
from_bytes = |bytes|
    if List.len(bytes) != 9 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Higher)
            [1, ..] -> Ok(Lower)
            [2, .. as data_bytes] ->
                Ok(
                    Specific(
                        data_bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
