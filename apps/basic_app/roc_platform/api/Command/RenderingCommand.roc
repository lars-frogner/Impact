# Hash: ddc2f21305b365d73ac4d13cd032c049534c2927a41685a4b869a3201311c91e
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::command::RenderingCommand
# Type category: Inline
# Commit: d505d37
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
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 7, 0)?
    test_roundtrip_for_variant(1, 2, 5)?
    test_roundtrip_for_variant(2, 2, 5)?
    test_roundtrip_for_variant(3, 2, 5)?
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
