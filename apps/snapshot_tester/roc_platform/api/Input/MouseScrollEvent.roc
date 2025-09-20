# Hash: 45260ff37994c4951a7350e0be03b5d42fa305a11199e2a5392b27507e1b5ee3
# Generated: 2025-09-20T18:50:26+00:00
# Rust type: impact::input::mouse::MouseScrollEvent
# Type category: Inline
# Commit: 9eb6f040 (dirty)
module [
    MouseScrollEvent,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A delta movement of the mouse wheel, expressed in pixels.
MouseScrollEvent : {
    delta_x : F64,
    delta_y : F64,
}

## Serializes a value of [MouseScrollEvent] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseScrollEvent -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f64(value.delta_x)
    |> Builtin.write_bytes_f64(value.delta_y)

## Deserializes a value of [MouseScrollEvent] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseScrollEvent _
from_bytes = |bytes|
    Ok(
        {
            delta_x: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            delta_y: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
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
