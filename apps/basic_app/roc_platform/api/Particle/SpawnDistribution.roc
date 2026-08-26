# Hash: aaef24ee6dd8f834
# Generated: 2026-08-26T10:07:46.893830377
# Rust type: impact_particle::SpawnDistribution
# Type category: POD
module [
    SpawnDistribution,
    same,
    uniform,
    uniform_in_cos,
    write_bytes,
    from_bytes,
]

import core.Builtin
import core.Radians

SpawnDistribution : {
    min_value : F32,
    max_value : F32,
}

same : F32 -> SpawnDistribution
same = |value|
    { min_value: value, max_value: value }

uniform : F32, F32 -> SpawnDistribution
uniform = |min_value, max_value|
    { min_value, max_value: Num.max(min_value, max_value) }

uniform_in_cos : Radians.Radians, Radians.Radians -> SpawnDistribution
uniform_in_cos = |min_angle, max_angle|
    {
        min_value: Num.cos(Num.max(Radians.radians(min_angle), Num.min(Num.pi, Radians.radians(max_angle)))),
        max_value: Num.cos(Num.max(0, Num.min(Num.pi, Radians.radians(min_angle))))
    }

## Serializes a value of [SpawnDistribution] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SpawnDistribution -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Builtin.write_bytes_f32(value.min_value)
    |> Builtin.write_bytes_f32(value.max_value)

## Deserializes a value of [SpawnDistribution] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SpawnDistribution _
from_bytes = |bytes|
    Ok(
        {
            min_value: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            max_value: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 8 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
