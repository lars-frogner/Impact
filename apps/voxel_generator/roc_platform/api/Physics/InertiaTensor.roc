# Hash: 151d5c597e19758f
# Generated: 2026-01-16T09:36:44.178916505
# Rust type: impact_physics::inertia::InertiaTensorP
# Type category: POD
module [
    InertiaTensor,
    write_bytes,
    from_bytes,
]

import core.Matrix3

## The inertia tensor of a physical body. This the "compact" version.
##
## This type is primarily intended for compact storage inside other types and
## collections. For computations, prefer the SIMD-friendly 16-byte aligned
## [`InertiaTensor`].
InertiaTensor : {
    matrix : Matrix3.Matrix3,
    inverse_matrix : Matrix3.Matrix3,
}

## Serializes a value of [InertiaTensor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InertiaTensor -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(72)
    |> Matrix3.write_bytes(value.matrix)
    |> Matrix3.write_bytes(value.inverse_matrix)

## Deserializes a value of [InertiaTensor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InertiaTensor _
from_bytes = |bytes|
    Ok(
        {
            matrix: bytes |> List.sublist({ start: 0, len: 36 }) |> Matrix3.from_bytes?,
            inverse_matrix: bytes |> List.sublist({ start: 36, len: 36 }) |> Matrix3.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 72 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
