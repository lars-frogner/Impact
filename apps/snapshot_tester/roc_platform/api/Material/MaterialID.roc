# Hash: e11ab88a2f6b28b5079edebd2bf134d1d387c6df75985a8521e220cfc17f03c4
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_material::MaterialID
# Type category: POD
# Commit: 189570ab (dirty)
module [
    MaterialID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for specific material types.
## Wraps a [`StringHash64`](impact_math::StringHash64).
MaterialID : Hashing.StringHash64

## Serializes a value of [MaterialID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MaterialID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [MaterialID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MaterialID _
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
