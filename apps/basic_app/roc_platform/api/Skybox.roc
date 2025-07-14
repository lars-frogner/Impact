# Hash: ac181db5ed0d317074634a7dd0fb13f51888f60f7d511bf2baa1e2a2f5cb8adb
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_scene::skybox::Skybox
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    Skybox,
    new,
    write_bytes,
    from_bytes,
]

import Rendering.TextureID
import core.Builtin

## A skybox specified by a cubemap texture and a maximum luminance (the
## luminance that a texel value of unity should be mapped to).
Skybox : {
    cubemap_texture_id : Rendering.TextureID.TextureID,
    max_luminance : F32,
}

## Creates a new skybox with the given cubemap texture and maximum
## luminance.
new : Rendering.TextureID.TextureID, F32 -> Skybox
new = |cubemap_texture_id, max_luminance|
    { cubemap_texture_id, max_luminance }

## Serializes a value of [Skybox] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, Skybox -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(8)
    |> Rendering.TextureID.write_bytes(value.cubemap_texture_id)
    |> Builtin.write_bytes_f32(value.max_luminance)

## Deserializes a value of [Skybox] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result Skybox _
from_bytes = |bytes|
    Ok(
        {
            cubemap_texture_id: bytes |> List.sublist({ start: 0, len: 4 }) |> Rendering.TextureID.from_bytes?,
            max_luminance: bytes |> List.sublist({ start: 4, len: 4 }) |> Builtin.from_bytes_f32?,
        },
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
