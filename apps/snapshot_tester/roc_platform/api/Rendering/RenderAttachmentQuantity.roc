# Hash: 31b3c16a3f448cbaf33e1e2911e7997a097c00a81c78216cdae0e7a0b53ae7f3
# Generated: 2025-07-13T20:18:37+00:00
# Rust type: impact_rendering::attachment::RenderAttachmentQuantity
# Type category: Inline
# Commit: b1b4dfd8 (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
