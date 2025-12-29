# Hash: ff1e3b3e9dbc471f
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact::input::key::KeyboardEvent
# Type category: Inline
module [
    KeyboardEvent,
    write_bytes,
    from_bytes,
]

import Input.KeyState
import Input.KeyboardKey

## A press or release of a keyboard key.
KeyboardEvent : {
    key : Input.KeyboardKey.KeyboardKey,
    state : Input.KeyState.KeyState,
}

## Serializes a value of [KeyboardEvent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, KeyboardEvent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(3)
    |> Input.KeyboardKey.write_bytes(value.key)
    |> Input.KeyState.write_bytes(value.state)

## Deserializes a value of [KeyboardEvent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KeyboardEvent _
from_bytes = |bytes|
    Ok(
        {
            key: bytes |> List.sublist({ start: 0, len: 2 }) |> Input.KeyboardKey.from_bytes?,
            state: bytes |> List.sublist({ start: 2, len: 1 }) |> Input.KeyState.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 3 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
