# Hash: 94fdbc74441f50e5829848811957c7f76ce21c37bc392455c6cc7dccbd0bc4d8
# Generated: 2025-12-17T23:58:02+00:00
# Rust type: impact_physics::force::spring_force::Spring
# Type category: POD
# Commit: 7d41822d (dirty)
module [
    Spring,
    new,
    standard,
    elastic_band,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A spring or elastic band.
Spring : {
    ## The spring constant representing the stiffness of the spring.
    stiffness : F32,
    ## The spring damping coefficient.
    damping : F32,
    ## The length for which the spring is in equilibrium.
    rest_length : F32,
    ## The length below which the spring force is always zero.
    slack_length : F32,
}

## Creates a new spring.
new : F32, F32, F32, F32 -> Spring
new = |stiffness, damping, rest_length, slack_length|
    {
        stiffness,
        damping,
        rest_length,
        slack_length,
    }

## Creates a standard spring (no slack).
standard : F32, F32, F32 -> Spring
standard = |stiffness, damping, rest_length|
    new(stiffness, damping, rest_length, 0)

## Creates an elastic band that is slack below a given length.
elastic_band : F32, F32, F32 -> Spring
elastic_band = |stiffness, damping, slack_length|
    new(stiffness, damping, slack_length, slack_length)

## Serializes a value of [Spring] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Spring -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Builtin.write_bytes_f32(value.stiffness)
    |> Builtin.write_bytes_f32(value.damping)
    |> Builtin.write_bytes_f32(value.rest_length)
    |> Builtin.write_bytes_f32(value.slack_length)

## Deserializes a value of [Spring] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Spring _
from_bytes = |bytes|
    Ok(
        {
            stiffness: bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
            damping: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
            rest_length: bytes |> List.sublist({ start: 8, len: 4 }) |> Builtin.from_bytes_f32?,
            slack_length: bytes |> List.sublist({ start: 12, len: 4 }) |> Builtin.from_bytes_f32?,
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
