# Hash: 9a59e09f4a6e275b54a89005945c7909f35de2ff15c03874b50c1e388716c0a7
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::control::command::ControlCommand
# Type category: Inline
# Commit: c6462c2 (dirty)
module [
    ControlCommand,
    write_bytes,
    from_bytes,
]

import Control.MotionDirection
import Control.MotionState
import core.Builtin

ControlCommand : [
    SetMotion {
            state : Control.MotionState.MotionState,
            direction : Control.MotionDirection.MotionDirection,
        },
    StopMotion,
    SetMovementSpeed F64,
]

## Serializes a value of [ControlCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControlCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetMotion { state, direction } ->
            bytes
            |> List.reserve(9)
            |> List.append(0)
            |> Control.MotionState.write_bytes(state)
            |> Control.MotionDirection.write_bytes(direction)
            |> List.concat(List.repeat(0, 6))

        StopMotion ->
            bytes
            |> List.reserve(9)
            |> List.append(1)
            |> List.concat(List.repeat(0, 8))

        SetMovementSpeed(val) ->
            bytes
            |> List.reserve(9)
            |> List.append(2)
            |> Builtin.write_bytes_f64(val)

## Deserializes a value of [ControlCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControlCommand _
from_bytes = |bytes|
    if List.len(bytes) != 9 then
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
                        data_bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
