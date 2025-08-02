# Hash: 4077406d41dc73e319b28a8de3571fed336ccc37cde00c01748ccbb24d30fa6e
# Generated: 2025-08-01T06:54:20+00:00
# Rust type: impact_scene::skybox::Skybox
# Type category: POD
# Commit: 5cd592d6 (dirty)
module [
    Skybox,
    new,
    write_bytes,
    from_bytes,
]

import Texture.TextureID
import core.Builtin

## A skybox specified by a cubemap texture and a maximum luminance (the
## luminance that a texel value of unity should be mapped to).
Skybox : {
    cubemap_texture_id : Texture.TextureID.TextureID,
    max_luminance : F64,
}

## Creates a new skybox with the given cubemap texture and maximum
## luminance.
new : Texture.TextureID.TextureID, F64 -> Skybox
new = |cubemap_texture_id, max_luminance|
    { cubemap_texture_id, max_luminance }

## Serializes a value of [Skybox] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Skybox -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(16)
    |> Texture.TextureID.write_bytes(value.cubemap_texture_id)
    |> Builtin.write_bytes_f64(value.max_luminance)

## Deserializes a value of [Skybox] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Skybox _
from_bytes = |bytes|
    Ok(
        {
            cubemap_texture_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Texture.TextureID.from_bytes?,
            max_luminance: bytes |> List.sublist({ start: 8, len: 8 }) |> Builtin.from_bytes_f64?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 16 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
