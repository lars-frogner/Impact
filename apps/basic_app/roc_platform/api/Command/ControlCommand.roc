# Hash: b79becd55a749392a18956e536ca8372acbcd2b03d5419d3bacf0e046a7180d3
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::control::command::ControlCommand
# Type category: Inline
# Commit: d505d37
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
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 3, 6)?
    test_roundtrip_for_variant(1, 1, 8)?
    test_roundtrip_for_variant(2, 9, 0)?
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
