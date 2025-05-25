# Hash: 0538789cef94f00bf2ee5cb24ef89fd29a85dd8d784f8ae36566282b024f72de
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::physics::command::PhysicsCommand
# Type category: Inline
# Commit: 31f3514 (dirty)
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.ToSimulationSpeedMultiplier
import Command.ToSubstepCount
import Physics.UniformMedium

PhysicsCommand : [
    SetSimulationSubstepCount Command.ToSubstepCount.ToSubstepCount,
    SetSimulationSpeed Command.ToSimulationSpeedMultiplier.ToSimulationSpeedMultiplier,
    SetMedium Physics.UniformMedium.UniformMedium,
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetSimulationSubstepCount(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(0)
            |> Command.ToSubstepCount.write_bytes(val)
            |> List.concat(List.repeat(0, 27))

        SetSimulationSpeed(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(1)
            |> Command.ToSimulationSpeedMultiplier.write_bytes(val)
            |> List.concat(List.repeat(0, 23))

        SetMedium(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(2)
            |> Physics.UniformMedium.write_bytes(val)

## Deserializes a value of [PhysicsCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PhysicsCommand _
from_bytes = |bytes|
    if List.len(bytes) != 33 then
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

            [2, .. as data_bytes] ->
                Ok(
                    SetMedium(
                        data_bytes |> List.sublist({ start: 0, len: 32 }) |> Physics.UniformMedium.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
