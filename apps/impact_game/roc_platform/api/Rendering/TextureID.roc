# Hash: 38c0d24f21ca83834c8690962179480ce5cfca4f933457be0efa71b12c7423d5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::texture::TextureID
# Type category: POD
# Commit: d505d37
module [
    TextureID,
    write_bytes,
    from_bytes,
]

import core.Hashing

## Identifier for specific textures.
## Wraps a [`StringHash32`](impact_math::StringHash32).
TextureID : Hashing.StringHash32

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
