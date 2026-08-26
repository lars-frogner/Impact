# Hash: 834f56856e9cddd9
# Generated: 2026-08-26T20:06:03.250940595
# Rust type: impact_particle::RateOverTime
# Type category: POD
module [
    RateOverTime,
    constant,
    burst,
    write_bytes,
    from_bytes,
]

import core.Builtin

## Piecewise linear rate of particle emission as a function of time. The rate
## is considered zero before `t0` and `r3` after `t3`. The times are assumed
## monotonic.
RateOverTime : {
    t0 : F32,
    t1 : F32,
    t2 : F32,
    t3 : F32,
    r0 : F32,
    r1 : F32,
    r2 : F32,
    r3 : F32,
}

constant : U32, F32, F32 -> RateOverTime
constant = |particle_count, delay, duration|
    clamped_duration = Num.max(duration, 1e-6)
    rate = Num.to_f32(particle_count) / clamped_duration
    start = Num.max(delay, 0.0)
    end = start + clamped_duration
    {
        t0: start,
        t1: end,
        t2: end,
        t3: end,
        r0: rate,
        r1: rate,
        r2: 0.0,
        r3: 0.0,
    }

burst : U32, F32 -> RateOverTime
burst = |particle_count, delay|
    constant(particle_count, delay, 0.0)

## Serializes a value of [RateOverTime] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RateOverTime -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_f32(value.t0)
    |> Builtin.write_bytes_f32(value.t1)
    |> Builtin.write_bytes_f32(value.t2)
    |> Builtin.write_bytes_f32(value.t3)
    |> Builtin.write_bytes_f32(value.r0)
    |> Builtin.write_bytes_f32(value.r1)
    |> Builtin.write_bytes_f32(value.r2)
    |> Builtin.write_bytes_f32(value.r3)

## Deserializes a value of [RateOverTime] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RateOverTime _
from_bytes = |bytes|
    Ok(
        {
            t0: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            t1: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            t2: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            t3: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
            r0: bytes |> List.sublist({ start: 16, len: 4 }) |> Builtin.from_bytes_f32?,
            r1: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
            r2: bytes |> List.sublist({ start: 24, len: 4 }) |> Builtin.from_bytes_f32?,
            r3: bytes |> List.sublist({ start: 28, len: 4 }) |> Builtin.from_bytes_f32?,
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
