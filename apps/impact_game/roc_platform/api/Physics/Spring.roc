# Hash: 20efc1a58720fcb668dcf47a87654933ac61fbb419c5aab6ebf494c7b4dcb78b
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::rigid_body::forces::spring::Spring
# Type category: POD
# Commit: d505d37
module [
    Spring,
    write_bytes,
    from_bytes,
]

import core.Builtin

## A spring or elastic band.
Spring : {
    ## The spring constant representing the stiffness of the spring.
    stiffness : F64,
    ## The spring damping coefficient.
    damping : F64,
    ## The length for which the spring is in equilibrium.
    rest_length : F64,
    ## The length below which the spring force is always zero.
    slack_length : F64,
}

## Serializes a value of [Spring] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Spring -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Builtin.write_bytes_f64(value.stiffness)
    |> Builtin.write_bytes_f64(value.damping)
    |> Builtin.write_bytes_f64(value.rest_length)
    |> Builtin.write_bytes_f64(value.slack_length)

## Deserializes a value of [Spring] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Spring _
from_bytes = |bytes|
    Ok(
        {
            stiffness: bytes |> List.sublist({ start: 0, len: 8 }) |> Builtin.from_bytes_f64?,
            damping: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
            rest_length: bytes |> List.sublist({ start: 16, len: 8 }) |> Builtin.from_bytes_f64?,
            slack_length: bytes |> List.sublist({ start: 24, len: 8 }) |> Builtin.from_bytes_f64?,
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
