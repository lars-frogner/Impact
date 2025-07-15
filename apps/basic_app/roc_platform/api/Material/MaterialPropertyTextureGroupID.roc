# Hash: a11f31d4c9ce708f64871b7491e3bd7af31f27f62c6d6fa648d670b196d03330
# Generated: 2025-07-15T17:32:17+00:00
# Rust type: impact_material::MaterialPropertyTextureGroupID
# Type category: POD
# Commit: 1fbb6f6b (dirty)
module [
    MaterialPropertyTextureGroupID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for group of textures used for material properties. Wraps a
## [`StringHash64`](impact_math::StringHash64).
MaterialPropertyTextureGroupID : Hashing.StringHash64

## Serializes a value of [MaterialPropertyTextureGroupID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MaterialPropertyTextureGroupID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [MaterialPropertyTextureGroupID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MaterialPropertyTextureGroupID _
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
