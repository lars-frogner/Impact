# Hash: e936201debc34465
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact::command::UserCommand
# Type category: Inline
module [
    EngineCommand,
    write_bytes,
    from_bytes,
]

import Command.ControllerCommand
import Command.SceneCommand

EngineCommand : [
    Scene Command.SceneCommand.SceneCommand,
    Controller Command.ControllerCommand.ControllerCommand,
]

## Serializes a value of [EngineCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, EngineCommand -> List U8
write_bytes = |bytes, value|
    when value is
        Scene(val) ->
            bytes
            |> List.reserve(18)
            |> List.append(0)
            |> Command.SceneCommand.write_bytes(val)

        Controller(val) ->
            bytes
            |> List.reserve(18)
            |> List.append(1)
            |> Command.ControllerCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 12))

## Deserializes a value of [EngineCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result EngineCommand _
from_bytes = |bytes|
    if List.len(bytes) != 18 then
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
                    Controller(
                        data_bytes |> List.sublist({ start: 0, len: 5 }) |> Command.ControllerCommand.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
