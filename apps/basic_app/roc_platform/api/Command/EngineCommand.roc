# Hash: 2c2ea5f09d6617a1d1f1a432ae43766e83d8c1f48662a15606588d0e4fe07218
# Generated: 2025-05-16T20:58:03+00:00
# Rust type: impact::engine::command::EngineCommand
# Type category: Inline
# Commit: 3e6e703 (dirty)
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
            |> List.reserve(11)
            |> List.append(0)
            |> Command.RenderingCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 3))

        Physics(val) ->
            bytes
            |> List.reserve(11)
            |> List.append(1)
            |> Command.PhysicsCommand.write_bytes(val)

        Scene(val) ->
            bytes
            |> List.reserve(11)
            |> List.append(2)
            |> Command.SceneCommand.write_bytes(val)

        Control(val) ->
            bytes
            |> List.reserve(11)
            |> List.append(3)
            |> Command.ControlCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 1))

        UI(val) ->
            bytes
            |> List.reserve(11)
            |> List.append(4)
            |> Command.UICommand.write_bytes(val)
            |> List.concat(List.repeat(0, 8))

        Capture(val) ->
            bytes
            |> List.reserve(11)
            |> List.append(5)
            |> Command.CaptureCommand.write_bytes(val)
            |> List.concat(List.repeat(0, 8))

        Exit ->
            bytes
            |> List.reserve(11)
            |> List.append(6)
            |> List.concat(List.repeat(0, 10))

## Deserializes a value of [EngineCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result EngineCommand _
from_bytes = |bytes|
    if List.len(bytes) != 11 then
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
                        data_bytes |> List.sublist({ start: 0, len: 10 }) |> Command.PhysicsCommand.from_bytes?,
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
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 8, 3)?
    test_roundtrip_for_variant(1, 11, 0)?
    test_roundtrip_for_variant(2, 11, 0)?
    test_roundtrip_for_variant(3, 10, 1)?
    test_roundtrip_for_variant(4, 3, 8)?
    test_roundtrip_for_variant(5, 3, 8)?
    test_roundtrip_for_variant(6, 1, 10)?
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
