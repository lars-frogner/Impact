# Hash: dd0bd2e84f938b4a
# Generated: 2026-01-24T10:14:45.774876843
# Rust type: impact::command::UserCommand
# Type category: Inline
module [
    EngineCommand,
    write_bytes,
    from_bytes,
]

import Command.ControlCommand
import Command.PhysicsCommand
import Command.SceneCommand

EngineCommand : [
    Scene Command.SceneCommand.SceneCommand,
    Control Command.ControlCommand.ControlCommand,
    Physics Command.PhysicsCommand.PhysicsCommand,
]

## Serializes a value of [EngineCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, EngineCommand -> List U8
write_bytes = |bytes, value|
    when value is
        Scene(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(0)
            |> Command.SceneCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 16))

        Control(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(1)
            |> Command.ControlCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 28))

        Physics(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(2)
            |> Command.PhysicsCommand.write_bytes(val)

## Deserializes a value of [EngineCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result EngineCommand _
from_bytes = |bytes|
    if List.len(bytes) != 34 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Scene(
                        data_bytes |> List.sublist({ start: 0, len: 17 }) |> Command.SceneCommand.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    Control(
                        data_bytes |> List.sublist({ start: 0, len: 5 }) |> Command.ControlCommand.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    Physics(
                        data_bytes |> List.sublist({ start: 0, len: 33 }) |> Command.PhysicsCommand.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
