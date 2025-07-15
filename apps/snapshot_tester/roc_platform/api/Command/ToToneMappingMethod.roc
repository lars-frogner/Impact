# Hash: 06bcafb60dbc37492bc0fc252157b2a624c49fb09df13cb31703f6830878e18b
# Generated: 2025-07-15T17:32:43+00:00
# Rust type: impact::gpu::rendering::postprocessing::command::ToToneMappingMethod
# Type category: Inline
# Commit: 1fbb6f6b (dirty)
module [
    ToToneMappingMethod,
    write_bytes,
    from_bytes,
]

import Rendering.ToneMappingMethod

ToToneMappingMethod : [
    Next,
    Specific Rendering.ToneMappingMethod.ToneMappingMethod,
]

## Serializes a value of [ToToneMappingMethod] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToToneMappingMethod -> List U8
write_bytes = |bytes, value|
    when value is
        Next ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> List.concat(List.repeat(0, 1))

        Specific(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(1)
            |> Rendering.ToneMappingMethod.write_bytes(val)

## Deserializes a value of [ToToneMappingMethod] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToToneMappingMethod _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Next)
            [1, .. as data_bytes] ->
                Ok(
                    Specific(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Rendering.ToneMappingMethod.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
