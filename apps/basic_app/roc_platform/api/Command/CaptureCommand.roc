# Hash: 9db1b9848c368a15e2c229567c3eed0c1cb060072516617e6b70b52383a3d372
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::gpu::rendering::screen_capture::command::CaptureCommand
# Type category: Inline
# Commit: c6462c2 (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
