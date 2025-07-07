# Hash: 43dd4e28d823a1f17555809393255e56773ee81635571179917dfe9e494615cb
# Generated: 2025-07-06T18:04:01+00:00
# Rust type: impact::gpu::rendering::postprocessing::command::ToRenderAttachmentQuantity
# Type category: Inline
# Commit: ce2d27b (dirty)
module [
    ToRenderAttachmentQuantity,
    write_bytes,
    from_bytes,
]

import Rendering.RenderAttachmentQuantity

ToRenderAttachmentQuantity : [
    Next,
    Previous,
    Specific Rendering.RenderAttachmentQuantity.RenderAttachmentQuantity,
]

## Serializes a value of [ToRenderAttachmentQuantity] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToRenderAttachmentQuantity -> List U8
write_bytes = |bytes, value|
    when value is
        Next ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> List.concat(List.repeat(0, 1))

        Previous ->
            bytes
            |> List.reserve(2)
            |> List.append(1)
            |> List.concat(List.repeat(0, 1))

        Specific(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(2)
            |> Rendering.RenderAttachmentQuantity.write_bytes(val)

## Deserializes a value of [ToRenderAttachmentQuantity] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToRenderAttachmentQuantity _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Next)
            [1, ..] -> Ok(Previous)
            [2, .. as data_bytes] ->
                Ok(
                    Specific(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Rendering.RenderAttachmentQuantity.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
