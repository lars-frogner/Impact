# Hash: c0bd91671649af8f
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact_physics::force::detailed_drag::DragLoadMapID
# Type category: POD
module [
    DragLoadMapID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a [`DragLoadMap`].
## Wraps a [`StringHash32`](impact_math::StringHash32).
DragLoadMapID : Hashing.StringHash32

## Serializes a value of [DragLoadMapID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, DragLoadMapID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Hashing.write_bytes_string_hash_32(value)

## Deserializes a value of [DragLoadMapID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result DragLoadMapID _
from_bytes = |bytes|
    Ok(
        (
            bytes |> List.sublist({ start: 0, len: 4 }) |> Hashing.from_bytes_string_hash_32?,
        ),
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 4 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
