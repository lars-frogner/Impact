# Hash: 829c1f4e834c74adb6d1b480dff935c3c0ad9221eedebacfe87a0efd65f9e829
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact::command::controller::ControllerCommand
# Type category: Inline
# Commit: 397d36d3 (dirty)
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
    SetMovementSpeed F64,
]

## Serializes a value of [ControllerCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControllerCommand -> List U8
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

## Deserializes a value of [ControllerCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControllerCommand _
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
