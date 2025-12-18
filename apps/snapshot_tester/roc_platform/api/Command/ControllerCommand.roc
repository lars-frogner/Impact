# Hash: 3a60e26a51cd851fca9b9a73051c8dc4462e631b16877fb15886ac2b868d77a4
# Generated: 2025-12-17T23:58:42+00:00
# Rust type: impact::command::controller::ControllerCommand
# Type category: Inline
# Commit: 7d41822d (dirty)
module [
    ControllerCommand,
    write_bytes,
    from_bytes,
]

import Control.MotionDirection
import Control.MotionState
import core.Builtin

ControllerCommand : [
    SetMotion {
            state : Control.MotionState.MotionState,
            direction : Control.MotionDirection.MotionDirection,
        },
    StopMotion,
    SetMovementSpeed F32,
]

## Serializes a value of [ControllerCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControllerCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetMotion { state, direction } ->
            bytes
            |> List.reserve(5)
            |> List.append(0)
            |> Control.MotionState.write_bytes(state)
            |> Control.MotionDirection.write_bytes(direction)
            |> List.concat(List.repeat(0, 2))

        StopMotion ->
            bytes
            |> List.reserve(5)
            |> List.append(1)
            |> List.concat(List.repeat(0, 4))

        SetMovementSpeed(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(2)
            |> Builtin.write_bytes_f32(val)

## Deserializes a value of [ControllerCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControllerCommand _
from_bytes = |bytes|
    if List.len(bytes) != 5 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetMotion     {
                        state: data_bytes |> List.sublist({ start: 0, len: 1 }) |> Control.MotionState.from_bytes?,
                        direction: data_bytes |> List.sublist({ start: 1, len: 1 }) |> Control.MotionDirection.from_bytes?,
                    },
                )

            [1, ..] -> Ok(StopMotion)
            [2, .. as data_bytes] ->
                Ok(
                    SetMovementSpeed(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
