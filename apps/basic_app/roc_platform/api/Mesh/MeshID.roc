# Hash: 83ae5edfea0e2ce5f6db355918fae4bd5cfba3793ac4b7670f0da1aac2c64c86
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::mesh::MeshID
# Type category: POD
# Commit: d505d37
module [
    MeshID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for specific meshes.
## Wraps a [`StringHash64`](impact_math::StringHash64).
MeshID : Hashing.StringHash64

## Serializes a value of [MeshID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MeshID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [MeshID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MeshID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 8 }) |> Hashing.from_bytes_string_hash_64?,
        ),
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
