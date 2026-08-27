# Hash: 591c1876c1ad2caa
# Generated: 2026-08-27T20:46:48.704899638
# Rust type: impact_particle::ColorOverLifetime
# Type category: POD
module [
    ColorOverLifetime,
    linear,
    linear_alpha,
    linear_fade,
    constant,
    write_bytes,
    from_bytes,
]

import core.Builtin
import core.Vector3
import core.Vector4

## Piecewise linear RGBA base color as a function of fractional time relative
## to the lifetime of the particle (so the full lifetime spans `t = [0.0,
## 1.0)`). The color is considered `c0` before `t0` and `c3` after `t3`. The
## fractional times are assumed non-negative and monotonic.
ColorOverLifetime : {
    t0 : F32,
    t1 : F32,
    t2 : F32,
    t3 : F32,
    c0 : Vector4.Vector4,
    c1 : Vector4.Vector4,
    c2 : Vector4.Vector4,
    c3 : Vector4.Vector4,
}

linear : Vector4.Vector4, Vector4.Vector4 -> ColorOverLifetime
linear = |start_color, end_color|
    {
        t0: 0.0,
        t1: 1.0,
        t2: 1.0,
        t3: 1.0,
        c0: start_color,
        c1: end_color,
        c2: end_color,
        c3: end_color,
    }

linear_alpha : Vector3.Vector3, F32, F32 -> ColorOverLifetime
linear_alpha = |color, start_alpha, end_alpha|
    start_color = (color.0, color.1, color.2, start_alpha)
    end_color = (color.0, color.1, color.2, end_alpha)
    linear(start_color, end_color)

linear_fade : Vector3.Vector3 -> ColorOverLifetime
linear_fade = |color|
    linear_alpha(color, 1.0, 0.0)

constant : Vector4.Vector4 -> ColorOverLifetime
constant = |color|
    linear(color, color)

## Serializes a value of [ColorOverLifetime] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ColorOverLifetime -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Builtin.write_bytes_f32(value.t0)
    |> Builtin.write_bytes_f32(value.t1)
    |> Builtin.write_bytes_f32(value.t2)
    |> Builtin.write_bytes_f32(value.t3)
    |> Vector4.write_bytes(value.c0)
    |> Vector4.write_bytes(value.c1)
    |> Vector4.write_bytes(value.c2)
    |> Vector4.write_bytes(value.c3)

## Deserializes a value of [ColorOverLifetime] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ColorOverLifetime _
from_bytes = |bytes|
    Ok(
        {
            t0: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            t1: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            t2: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            t3: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            c0: bytes |> List.sublist({ start: 16, len: 16 }) |> Vector4.from_bytes?,
            c1: bytes |> List.sublist({ start: 32, len: 16 }) |> Vector4.from_bytes?,
            c2: bytes |> List.sublist({ start: 48, len: 16 }) |> Vector4.from_bytes?,
            c3: bytes |> List.sublist({ start: 64, len: 16 }) |> Vector4.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 80 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
