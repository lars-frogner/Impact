# Hash: 7e9519ca30a040e14b1bfaa9f10f9fb3ef44ff125a2e75816c8091fc22934e3b
# Generated: 2025-09-20T20:10:37+00:00
# Rust type: impact::input::mouse::CursorPosition
# Type category: Inline
# Commit: 5fdd98b9 (dirty)
module [
    CursorPosition,
    write_bytes,
    from_bytes,
]

import core.Builtin

## The position of the cursor within the window, relative to the center. The
## coordinates are normalized to the ranges `[-1, 1]` for `y` and `[-a, a]` for
## `x`, where `a = w/h` is the aspect ratio of the window. The lower left
## corner of the window is at `(-a, -1)`.
CursorPosition : {
    x : F64,
    y : F64,
}

## Serializes a value of [CursorPosition] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CursorPosition -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f64(value.x)
    |> Builtin.write_bytes_f64(value.y)

## Deserializes a value of [CursorPosition] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CursorPosition _
from_bytes = |bytes|
    Ok(
        {
            x: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            y: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
