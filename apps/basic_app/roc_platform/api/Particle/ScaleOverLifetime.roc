# Hash: 9f85d92aa87fbccd
# Generated: 2026-08-27T20:43:58.32619579
# Rust type: impact_particle::ScaleOverLifetime
# Type category: POD
module [
    ScaleOverLifetime,
    linear,
    linear_shrink,
    constant,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Piecewise linear scale as a function of fractional time relative to the
## lifetime of the particle (so the full lifetime spans `t = [0.0, 1.0)`). The
## scale is considered `s0` before `t0` and `s3` after `t3`. The fractional
## times are assumed non-negative and monotonic.
ScaleOverLifetime : {
    t0 : F32,
    t1 : F32,
    t2 : F32,
    t3 : F32,
    s0 : F32,
    s1 : F32,
    s2 : F32,
    s3 : F32,
}

linear : F32, F32 -> ScaleOverLifetime
linear = |start_scale, end_scale|
    {
        t0: 0.0,
        t1: 1.0,
        t2: 1.0,
        t3: 1.0,
        s0: start_scale,
        s1: end_scale,
        s2: end_scale,
        s3: end_scale,
    }

linear_shrink : {} -> ScaleOverLifetime
linear_shrink = |{}|
    linear(1.0, 0.0)

constant : F32 -> ScaleOverLifetime
constant = |scale|
    linear(scale, scale)

## Serializes a value of [ScaleOverLifetime] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ScaleOverLifetime -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_f32(value.t0)
    |> Builtin.write_bytes_f32(value.t1)
    |> Builtin.write_bytes_f32(value.t2)
    |> Builtin.write_bytes_f32(value.t3)
    |> Builtin.write_bytes_f32(value.s0)
    |> Builtin.write_bytes_f32(value.s1)
    |> Builtin.write_bytes_f32(value.s2)
    |> Builtin.write_bytes_f32(value.s3)

## Deserializes a value of [ScaleOverLifetime] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ScaleOverLifetime _
from_bytes = |bytes|
    Ok(
        {
            t0: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            t1: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            t2: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            t3: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            s0: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
            s1: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
            s2: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_f32?,
            s3: bytes |> List.sublist({ start: 28, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
