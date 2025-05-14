# Hash: 72c52675196c36058c273a3cf350c5f28ce382626d4bcdd3e953add0033f5bbc
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::command::PhysicsCommand
# Type category: Inline
# Commit: d505d37
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.ToSimulationSpeedMultiplier
import Command.ToSubstepCount

PhysicsCommand : [
    SetSimulationSubstepCount Command.ToSubstepCount.ToSubstepCount,
    SetSimulationSpeed Command.ToSimulationSpeedMultiplier.ToSimulationSpeedMultiplier,
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetSimulationSubstepCount(val) ->
            bytes
            |> List.reserve(10)
            |> List.append(0)
            |> Command.ToSubstepCount.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetSimulationSpeed(val) ->
            bytes
            |> List.reserve(10)
            |> List.append(1)
            |> Command.ToSimulationSpeedMultiplier.write_bytes(val)

## Deserializes a value of [PhysicsCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PhysicsCommand _
from_bytes = |bytes|
    if List.len(bytes) != 10 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetSimulationSubstepCount(
                        data_bytes |> List.sublist({ start: 0, len: 5 }) |> Command.ToSubstepCount.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetSimulationSpeed(
                        data_bytes |> List.sublist({ start: 0, len: 9 }) |> Command.ToSimulationSpeedMultiplier.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 6, 4)?
    test_roundtrip_for_variant(1, 10, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
