# Hash: 4d4f4b919d2a3454eead5949f4a18bf20ecb619e9619fdeea5d639eb99e4d7eb
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::physics::command::PhysicsCommand
# Type category: Inline
# Commit: c6462c2 (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
