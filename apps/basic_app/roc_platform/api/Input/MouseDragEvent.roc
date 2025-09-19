# Hash: 1397c0f29d3e9190f6d7554b962018e1ea8663e3ec4b91cd91b49a091aedfdaf
# Generated: 2025-09-19T18:59:31+00:00
# Rust type: impact::input::mouse::MouseDragEvent
# Type category: Inline
# Commit: ff568180 (dirty)
module [
    MouseDragEvent,
    write_bytes,
    from_bytes,
]

import Input.MouseButtonSet
import core.Builtin

## A delta movement of the mouse, expressed in radians across the field of
## view, as well as the set of mouse buttons currently pressed.
MouseDragEvent : {
    delta_x : F64,
    delta_y : F64,
    pressed : Input.MouseButtonSet.MouseButtonSet,
}

## Serializes a value of [MouseDragEvent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseDragEvent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(17)
    |> Builtin.write_bytes_f64(value.delta_x)
    |> Builtin.write_bytes_f64(value.delta_y)
    |> Input.MouseButtonSet.write_bytes(value.pressed)

## Deserializes a value of [MouseDragEvent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseDragEvent _
from_bytes = |bytes|
    Ok(
        {
            delta_x: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            delta_y: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            pressed: bytes |> List.sublist({ start: 16, len: 1 }) |> Input.MouseButtonSet.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 17 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
