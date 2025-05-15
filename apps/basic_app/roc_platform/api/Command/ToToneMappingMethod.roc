# Hash: fa545c87b70e4366ecdb733b48bc17aff00526ed3edbb941ab42a09fd09f2596
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::postprocessing::command::ToToneMappingMethod
# Type category: Inline
# Commit: d505d37
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
