# Hash: 31e717876e794ea4d67e44a75a0a40bb37ede53bb6f1b5bc44cffc12be8befa9
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact_gpu::texture::TextureID
# Type category: POD
# Commit: 397d36d3 (dirty)
module [
    TextureID,
    from_name,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for specific textures.
## Wraps a [`StringHash32`](impact_math::StringHash32).
TextureID : Hashing.StringHash32

## Creates a texture ID hashed from the given name.
from_name : Str -> TextureID
from_name = |name|
    Hashing.hash_str_32(name)

## Serializes a value of [TextureID] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, TextureID -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(4)
    |> Hashing.write_bytes_string_hash_32(value)

## Deserializes a value of [TextureID] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result TextureID _
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
