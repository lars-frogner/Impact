# Hash: d1c95adedd8709ede89563e9cea5dc9b5ff21c4b980b2cf5f942b35f1ff4a3cd
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact::command::rendering::RenderingCommand
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    RenderingCommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState
import Command.ToExposure
import Command.ToRenderAttachmentQuantity
import Command.ToToneMappingMethod

RenderingCommand : [
    SetAmbientOcclusion Command.ToActiveState.ToActiveState,
    SetTemporalAntiAliasing Command.ToActiveState.ToActiveState,
    SetBloom Command.ToActiveState.ToActiveState,
    SetToneMappingMethod Command.ToToneMappingMethod.ToToneMappingMethod,
    SetExposure Command.ToExposure.ToExposure,
    SetRenderAttachmentVisualization Command.ToActiveState.ToActiveState,
    SetVisualizedRenderAttachmentQuantity Command.ToRenderAttachmentQuantity.ToRenderAttachmentQuantity,
    SetShadowMapping Command.ToActiveState.ToActiveState,
    SetWireframeMode Command.ToActiveState.ToActiveState,
    SetRenderPassTimings Command.ToActiveState.ToActiveState,
]

## Serializes a value of [RenderingCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RenderingCommand -> List U8
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

        SetShadowMapping(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(7)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetWireframeMode(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(8)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

        SetRenderPassTimings(val) ->
            bytes
            |> List.reserve(6)
            |> List.append(9)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 4))

## Deserializes a value of [RenderingCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RenderingCommand _
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

            [7, .. as data_bytes] ->
                Ok(
                    SetShadowMapping(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [8, .. as data_bytes] ->
                Ok(
                    SetWireframeMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [9, .. as data_bytes] ->
                Ok(
                    SetRenderPassTimings(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
