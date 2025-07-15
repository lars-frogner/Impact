# Hash: 02fbfdc24b9e60ec6bef62823e8756c01d00451b72327023e3c8dfde3e7c43b4
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact::gpu::rendering::command::RenderingCommand
# Type category: Inline
# Commit: 189570ab (dirty)
module [
    RenderingCommand,
    write_bytes,
    from_bytes,
]

import Command.PostprocessingCommand
import Command.ToActiveState

RenderingCommand : [
    Postprocessing Command.PostprocessingCommand.PostprocessingCommand,
    SetShadowMapping Command.ToActiveState.ToActiveState,
    SetWireframeMode Command.ToActiveState.ToActiveState,
    SetRenderPassTimings Command.ToActiveState.ToActiveState,
]

## Serializes a value of [RenderingCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, RenderingCommand -> List U8
write_bytes = |bytes, value|
    when value is
        Postprocessing(val) ->
            bytes
            |> List.reserve(7)
            |> List.append(0)
            |> Command.PostprocessingCommand.write_bytes(val)

        SetShadowMapping(val) ->
            bytes
            |> List.reserve(7)
            |> List.append(1)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 5))

        SetWireframeMode(val) ->
            bytes
            |> List.reserve(7)
            |> List.append(2)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 5))

        SetRenderPassTimings(val) ->
            bytes
            |> List.reserve(7)
            |> List.append(3)
            |> Command.ToActiveState.write_bytes(val)
            |> List.concat(List.repeat(0, 5))

## Deserializes a value of [RenderingCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result RenderingCommand _
from_bytes = |bytes|
    if List.len(bytes) != 7 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Postprocessing(
                        data_bytes |> List.sublist({ start: 0, len: 6 }) |> Command.PostprocessingCommand.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetShadowMapping(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetWireframeMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    SetRenderPassTimings(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
