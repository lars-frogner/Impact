# Hash: 589d9df88500798879e45c15e89195cc358efdf46ec03530e9487a1f48e880cd
# Generated: 2025-09-20T21:03:28+00:00
# Rust type: impact::input::mouse::CursorDirection
# Type category: Inline
# Commit: 5fdd98b9 (dirty)
module [
    CursorDirection,
    write_bytes,
    from_bytes,
]

import core.Builtin

## The direction the cursor is pointing in camera space relative to the camera
## looking direction, expressed in radians along the horizontal and vertical
## axes of the window. The values are bounded by the horizontal and vertical
## field of view of the camera.
CursorDirection : {
    ang_x : F64,
    ang_y : F64,
}

## Serializes a value of [CursorDirection] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CursorDirection -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f64(value.ang_x)
    |> Builtin.write_bytes_f64(value.ang_y)

## Deserializes a value of [CursorDirection] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CursorDirection _
from_bytes = |bytes|
    Ok(
        {
            ang_x: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            ang_y: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
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
