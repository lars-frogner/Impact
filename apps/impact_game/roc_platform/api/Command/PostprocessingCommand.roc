# Hash: 97e4b12548ae138087fea82d2939706dc8b368510e63d42ed2d8c21eeda441fc
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::postprocessing::command::PostprocessingCommand
# Type category: Inline
# Commit: d505d37
module [
    PostprocessingCommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState
import Command.ToExposure
import Command.ToRenderAttachmentQuantity
import Command.ToToneMappingMethod

PostprocessingCommand : [
    SetAmbientOcclusion Command.ToActiveState.ToActiveState,
    SetTemporalAntiAliasing Command.ToActiveState.ToActiveState,
    SetBloom Command.ToActiveState.ToActiveState,
    SetToneMappingMethod Command.ToToneMappingMethod.ToToneMappingMethod,
    SetExposure Command.ToExposure.ToExposure,
    SetRenderAttachmentVisualization Command.ToActiveState.ToActiveState,
    SetVisualizedRenderAttachmentQuantity Command.ToRenderAttachmentQuantity.ToRenderAttachmentQuantity,
]

## Serializes a value of [PostprocessingCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PostprocessingCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetAmbientOcclusion(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(0)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetTemporalAntiAliasing(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(1)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetBloom(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(2)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetToneMappingMethod(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(3)
            |> Command.ToToneMappingMethod.write_bytes(val)
            |> List.concat(List.repeat(0, 3))

        SetExposure(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(4)
            |> Command.ToExposure.write_bytes(val)

        SetRenderAttachmentVisualization(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(5)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetVisualizedRenderAttachmentQuantity(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(6)
            |> Command.ToRenderAttachmentQuantity.write_bytes(val)
            |> List.concat(List.repeat(0, 3))

## Deserializes a value of [PostprocessingCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PostprocessingCommand _
from_bytes = |bytes|
    if List.len(bytes) != 6 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetAmbientOcclusion(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetTemporalAntiAliasing(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetBloom(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    SetToneMappingMethod(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.ToToneMappingMethod.from_bytes?,
                    ),
                )

            [4, .. as data_bytes] ->
                Ok(
                    SetExposure(
                        data_bytes |> List.sublist({ start: 0, len: 5 }) |> Command.ToExposure.from_bytes?,
                    ),
                )

            [5, .. as data_bytes] ->
                Ok(
                    SetRenderAttachmentVisualization(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [6, .. as data_bytes] ->
                Ok(
                    SetVisualizedRenderAttachmentQuantity(
                        data_bytes |> List.sublist({ start: 0, len: 2 }) |> Command.ToRenderAttachmentQuantity.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 2, 4)?
    test_roundtrip_for_variant(1, 2, 4)?
    test_roundtrip_for_variant(2, 2, 4)?
    test_roundtrip_for_variant(3, 3, 3)?
    test_roundtrip_for_variant(4, 6, 0)?
    test_roundtrip_for_variant(5, 2, 4)?
    test_roundtrip_for_variant(6, 3, 3)?
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
