# Hash: 6473dc76b9f9a72d
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact::input::mouse::MouseScrollEvent
# Type category: Inline
module [
    MouseScrollEvent,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A delta movement of the mouse wheel, expressed in pixels scaled by the
## global scroll sensitivity factor.
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
