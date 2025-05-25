# Hash: ad3ed4a56a0f0daed48277aa13630ce3ab2edaa3e6cbdeaecc939319a52b2705
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::window::input::mouse::MouseButtonEvent
# Type category: Inline
# Commit: 31f3514 (dirty)
module [
    MouseButtonEvent,
    write_bytes,
    from_bytes,
]

import Input.MouseButton
import Input.MouseButtonState

## A press or release of a mouse button.
MouseButtonEvent : {
    button : Input.MouseButton.MouseButton,
    state : Input.MouseButtonState.MouseButtonState,
}

## Serializes a value of [MouseButtonEvent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseButtonEvent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(2)
    |> Input.MouseButton.write_bytes(value.button)
    |> Input.MouseButtonState.write_bytes(value.state)

## Deserializes a value of [MouseButtonEvent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseButtonEvent _
from_bytes = |bytes|
    Ok(
        {
            button: bytes |> List.sublist({ start: 0, len: 1 }) |> Input.MouseButton.from_bytes?,
            state: bytes |> List.sublist({ start: 1, len: 1 }) |> Input.MouseButtonState.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 2 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
