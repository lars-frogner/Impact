# Hash: 0499dcbe5efcac61ff4441cac2d885d06caa937ef9fcb9a1f493af90c4c5718a
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::physics::inertia::InertiaTensor
# Type category: POD
# Commit: d505d37
module [
    InertiaTensor,
    write_bytes,
    from_bytes,
]

import core.Matrix3

## The inertia tensor of a physical body.
InertiaTensor : {
    matrix : Matrix3.Matrix3 Binary64,
    inverse_matrix : Matrix3.Matrix3 Binary64,
}

## Serializes a value of [InertiaTensor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InertiaTensor -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(144)
    |> Matrix3.write_bytes_64(value.matrix)
    |> Matrix3.write_bytes_64(value.inverse_matrix)

## Deserializes a value of [InertiaTensor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InertiaTensor _
from_bytes = |bytes|
    Ok(
        {
            matrix: bytes |> List.sublist({ start: 0, len: 72 }) |> Matrix3.from_bytes_64?,
            inverse_matrix: bytes |> List.sublist({ start: 72, len: 72 }) |> Matrix3.from_bytes_64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 144 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
