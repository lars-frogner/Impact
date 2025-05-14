# Hash: 39a1c1bca6a6fb231ab99130f6966f7128072f2b67af70d5bcd977277fa7c99b
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::texture::attachment::RenderAttachmentQuantity
# Type category: Inline
# Commit: d505d37
module [
    RenderAttachmentQuantity,
    write_bytes,
    from_bytes,
]


## A quantity that can be rendered to a dedicated render attachment texture.
RenderAttachmentQuantity : [
    DepthStencil,
    LinearDepth,
    NormalVector,
    MotionVector,
    MaterialColor,
    MaterialProperties,
    Luminance,
    LuminanceAux,
    LuminanceHistory,
    PreviousLuminanceHistory,
    Occlusion,
]

## Serializes a value of [RenderAttachmentQuantity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RenderAttachmentQuantity -> List U8
write_bytes = |bytes, value|
    when value is
        DepthStencil ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        LinearDepth ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        NormalVector ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        MotionVector ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        MaterialColor ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        MaterialProperties ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        Luminance ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        LuminanceAux ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        LuminanceHistory ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        PreviousLuminanceHistory ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        Occlusion ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

## Deserializes a value of [RenderAttachmentQuantity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RenderAttachmentQuantity _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(DepthStencil)
            [1, ..] -> Ok(LinearDepth)
            [2, ..] -> Ok(NormalVector)
            [3, ..] -> Ok(MotionVector)
            [4, ..] -> Ok(MaterialColor)
            [5, ..] -> Ok(MaterialProperties)
            [6, ..] -> Ok(Luminance)
            [7, ..] -> Ok(LuminanceAux)
            [8, ..] -> Ok(LuminanceHistory)
            [9, ..] -> Ok(PreviousLuminanceHistory)
            [10, ..] -> Ok(Occlusion)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
    test_roundtrip_for_variant(3, 1, 0)?
    test_roundtrip_for_variant(4, 1, 0)?
    test_roundtrip_for_variant(5, 1, 0)?
    test_roundtrip_for_variant(6, 1, 0)?
    test_roundtrip_for_variant(7, 1, 0)?
    test_roundtrip_for_variant(8, 1, 0)?
    test_roundtrip_for_variant(9, 1, 0)?
    test_roundtrip_for_variant(10, 1, 0)?
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
