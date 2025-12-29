# Hash: f717181160caf065
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact::input::mouse::MouseDragEvent
# Type category: Inline
module [
    MouseDragEvent,
    write_bytes,
    from_bytes,
]

import Input.CursorDirection
import Input.MouseButtonSet
import core.Builtin

## A delta movement of the mouse, expressed in radians across the field of
## view. Positive `y`-delta is towards the top of the window. The current
## camera-space direction of the cursor as well as the set of mouse buttons
## currently pressed are included for context.
MouseDragEvent : {
    ang_delta_x : F64,
    ang_delta_y : F64,
    cursor : Input.CursorDirection.CursorDirection,
    pressed : Input.MouseButtonSet.MouseButtonSet,
}

## Serializes a value of [MouseDragEvent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseDragEvent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(33)
    |> Builtin.write_bytes_f64(value.ang_delta_x)
    |> Builtin.write_bytes_f64(value.ang_delta_y)
    |> Input.CursorDirection.write_bytes(value.cursor)
    |> Input.MouseButtonSet.write_bytes(value.pressed)

## Deserializes a value of [MouseDragEvent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseDragEvent _
from_bytes = |bytes|
    Ok(
        {
            ang_delta_x: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            ang_delta_y: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            cursor: bytes |> List.sublist({ start: 16, len: 16 }) |> Input.CursorDirection.from_bytes?,
            pressed: bytes |> List.sublist({ start: 32, len: 1 }) |> Input.MouseButtonSet.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 33 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
