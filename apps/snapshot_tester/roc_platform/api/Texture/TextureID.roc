# Hash: 39523d553680bab2d4206abcdb6eb7af1120077c0e24c197ca62c8e035715107
# Generated: 2025-08-01T06:54:20+00:00
# Rust type: impact_texture::TextureID
# Type category: POD
# Commit: 5cd592d6 (dirty)
module [
    TextureID,
    from_name,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for a texture.
TextureID : Hashing.StringHash64

## Creates a texture ID hashed from the given name.
from_name : Str -> TextureID
from_name = |name|
    Hashing.hash_str_64(name)

## Serializes a value of [TextureID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TextureID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Hashing.write_bytes_string_hash_64(value)

## Deserializes a value of [TextureID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TextureID _
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
