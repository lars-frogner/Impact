# Hash: b23dfd8e2fb9c2c8379fdb7216b69963c6621b551adf9c26813b1b27f2ee2d8e
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::screen_capture::command::SaveShadowMapsFor
# Type category: Inline
# Commit: d505d37
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
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
