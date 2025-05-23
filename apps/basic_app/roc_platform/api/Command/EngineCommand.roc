# Hash: 7ffdd60aef12c67b680a22a87e17575f48807ee116f6c648b08bc418d2421459
# Generated: 2025-05-22T18:34:24+00:00
# Rust type: impact::engine::command::EngineCommand
# Type category: Inline
# Commit: 8a339ce (dirty)
module [
    EngineCommand,
    write_bytes,
    from_bytes,
]

import Command.CaptureCommand
import Command.ControlCommand
import Command.PhysicsCommand
import Command.RenderingCommand
import Command.SceneCommand
import Command.UICommand

EngineCommand : [
    Rendering Command.RenderingCommand.RenderingCommand,
    Physics Command.PhysicsCommand.PhysicsCommand,
    Scene Command.SceneCommand.SceneCommand,
    Control Command.ControlCommand.ControlCommand,
    UI Command.UICommand.UICommand,
    Capture Command.CaptureCommand.CaptureCommand,
    Exit,
]

## Serializes a value of [EngineCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, EngineCommand -> List U8
write_bytes = |bytes, value|
    when value is
        Rendering(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(0)
            |> Command.RenderingCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 26))

        Physics(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(1)
            |> Command.PhysicsCommand.write_bytes(val)

        Scene(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(2)
            |> Command.SceneCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 23))

        Control(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(3)
            |> Command.ControlCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 24))

        UI(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(4)
            |> Command.UICommand.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        Capture(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(5)
            |> Command.CaptureCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        Exit ->
            bytes
            |> List.reserve(34)
            |> List.append(6)
            |> List.concat(List.repeat(0, 33))

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
                    Rendering(
                        data_bytes |> List.sublist({ start: 0, len: 7 }) |> Command.RenderingCommand.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    Physics(
                        data_bytes |> List.sublist({ start: 0, len: 33 }) |> Command.PhysicsCommand.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    Scene(
                        data_bytes |> List.sublist({ start: 0, len: 10 }) |> Command.SceneCommand.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    Control(
                        data_bytes |> List.sublist({ start: 0, len: 9 }) |> Command.ControlCommand.from_bytes?,
                    ),
                )

            [4, .. as data_bytes] ->
                Ok(
                    UI(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.UICommand.from_bytes?,
                    ),
                )

            [5, .. as data_bytes] ->
                Ok(
                    Capture(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.CaptureCommand.from_bytes?,
                    ),
                )

            [6, ..] -> Ok(Exit)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
