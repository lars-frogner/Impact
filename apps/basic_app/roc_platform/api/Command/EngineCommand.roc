# Hash: 008e7f8634db141da34a0b60b0007e4e9a64fa40d56d5eec62619db26c565a84
# Generated: 2025-07-22T11:50:31+00:00
# Rust type: impact::command::EngineCommand
# Type category: Inline
# Commit: 0c4a6fe6 (dirty)
module [
    EngineCommand,
    write_bytes,
    from_bytes,
]

import Command.CaptureCommand
import Command.ControllerCommand
import Command.GameLoopCommand
import Command.InstrumentationCommand
import Command.PhysicsCommand
import Command.RenderingCommand
import Command.SceneCommand

EngineCommand : [
    Rendering Command.RenderingCommand.RenderingCommand,
    Physics Command.PhysicsCommand.PhysicsCommand,
    Scene Command.SceneCommand.SceneCommand,
    Controller Command.ControllerCommand.ControllerCommand,
    Capture Command.CaptureCommand.CaptureCommand,
    Instrumentation Command.InstrumentationCommand.InstrumentationCommand,
    GameLoop Command.GameLoopCommand.GameLoopCommand,
    Shutdown,
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
            |> List.concat(List.repeat(0, 27))

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

        Controller(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(3)
            |> Command.ControllerCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 24))

        Capture(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(4)
            |> Command.CaptureCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        Instrumentation(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(5)
            |> Command.InstrumentationCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        GameLoop(val) ->
            bytes
            |> List.reserve(34)
            |> List.append(6)
            |> Command.GameLoopCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 31))

        Shutdown ->
            bytes
            |> List.reserve(34)
            |> List.append(7)
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
                        data_bytes |> List.sublist({ start: 0, len: 6 }) |> Command.RenderingCommand.from_bytes?,
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
                    Controller(
                        data_bytes |> List.sublist({ start: 0, len: 9 }) |> Command.ControllerCommand.from_bytes?,
                    ),
                )

            [4, .. as data_bytes] ->
                Ok(
                    Capture(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.CaptureCommand.from_bytes?,
                    ),
                )

            [5, .. as data_bytes] ->
                Ok(
                    Instrumentation(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.InstrumentationCommand.from_bytes?,
                    ),
                )

            [6, .. as data_bytes] ->
                Ok(
                    GameLoop(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.GameLoopCommand.from_bytes?,
                    ),
                )

            [7, ..] -> Ok(Shutdown)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
