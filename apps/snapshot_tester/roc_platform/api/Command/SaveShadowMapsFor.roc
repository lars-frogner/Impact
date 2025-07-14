# Hash: 722c0380bbc5418d7a94b8fcb8620c73962115a1dc74da76793288eb745f4c2f
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact::gpu::rendering::screen_capture::command::SaveShadowMapsFor
# Type category: Inline
# Commit: b1b4dfd8 (dirty)
module [
    SaveShadowMapsFor,
    write_bytes,
    from_bytes,
]

SaveShadowMapsFor : [
    OmnidirectionalLight,
    UnidirectionalLight,
]

## Serializes a value of [SaveShadowMapsFor] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SaveShadowMapsFor -> List U8
write_bytes = |bytes, value|
    when value is
        OmnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        UnidirectionalLight ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [SaveShadowMapsFor] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SaveShadowMapsFor _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(OmnidirectionalLight)
            [1, ..] -> Ok(UnidirectionalLight)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
