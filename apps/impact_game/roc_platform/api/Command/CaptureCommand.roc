# Hash: d51b635eaf48f235b505b25bd78f0c55426bbd3f9c4d5ebb164ec6e2a4bfcd4f
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::screen_capture::command::CaptureCommand
# Type category: Inline
# Commit: d505d37
module [
    CaptureCommand,
    write_bytes,
    from_bytes,
]

import Command.SaveShadowMapsFor

CaptureCommand : [
    SaveScreenshot,
    SaveShadowMaps Command.SaveShadowMapsFor.SaveShadowMapsFor,
]

## Serializes a value of [CaptureCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CaptureCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SaveScreenshot ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> List.concat(List.repeat(0, 1))

        SaveShadowMaps(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(1)
            |> Command.SaveShadowMapsFor.write_bytes(val)

## Deserializes a value of [CaptureCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CaptureCommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(SaveScreenshot)
            [1, .. as data_bytes] ->
                Ok(
                    SaveShadowMaps(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.SaveShadowMapsFor.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 1)?
    test_roundtrip_for_variant(1, 2, 0)?
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
