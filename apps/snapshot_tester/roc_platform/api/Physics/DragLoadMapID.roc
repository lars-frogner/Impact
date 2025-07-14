# Hash: 054526910b3565649f4d5f5ebeca2ae1ba7ddc008f79af2064e201539721949f
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact_physics::force::detailed_drag::DragLoadMapID
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    DragLoadMapID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a [`DragLoadMap`].
## Wraps a [`StringHash64`](impact_math::StringHash64).
DragLoadMapID : Hashing.StringHash64

## Serializes a value of [DragLoadMapID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DragLoadMapID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [DragLoadMapID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DragLoadMapID _
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
