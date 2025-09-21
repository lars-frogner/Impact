# Hash: 517365ef657e92eecd984489154cb5c66b31333eb4e70de5dfc5f51578ab1075
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact::command::physics::PhysicsCommand
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState
import Command.ToSimulationSpeedMultiplier
import Command.ToSubstepCount
import Physics.UniformMedium

PhysicsCommand : [
    SetSimulation Command.ToActiveState.ToActiveState,
    SetSimulationSubstepCount Command.ToSubstepCount.ToSubstepCount,
    SetSimulationSpeed Command.ToSimulationSpeedMultiplier.ToSimulationSpeedMultiplier,
    SetMedium Physics.UniformMedium.UniformMedium,
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetSimulation(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(0)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        SetSimulationSubstepCount(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(1)
            |> Command.ToSubstepCount.write_bytes(val)
            |> List.concat(List.repeat(0, 27))

        SetSimulationSpeed(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(2)
            |> Command.ToSimulationSpeedMultiplier.write_bytes(val)
            |> List.concat(List.repeat(0, 23))

        SetMedium(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(3)
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
                    SetSimulation(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetSimulationSubstepCount(
                        data_bytes |> List.sublist({ start: 0, len: 5 }) |> Command.ToSubstepCount.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetSimulationSpeed(
                        data_bytes |> List.sublist({ start: 0, len: 9 }) |> Command.ToSimulationSpeedMultiplier.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    SetMedium(
                        data_bytes |> List.sublist({ start: 0, len: 32 }) |> Physics.UniformMedium.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
