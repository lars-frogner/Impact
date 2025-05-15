# Hash: bcd0f43fa7c1326f2a26ea6a78feebd71558a497b73a5419d3c46ac6851ab5b6
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::gpu::rendering::postprocessing::capturing::tone_mapping::ToneMappingMethod
# Type category: Inline
# Commit: d505d37
module [
    ToneMappingMethod,
    write_bytes,
    from_bytes,
]


## The method to use for tone mapping.
ToneMappingMethod : [
    None,
    ACES,
    KhronosPBRNeutral,
]

## Serializes a value of [ToneMappingMethod] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToneMappingMethod -> List U8
write_bytes = |bytes, value|
    when value is
        None ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        ACES ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        KhronosPBRNeutral ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [ToneMappingMethod] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToneMappingMethod _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(None)
            [1, ..] -> Ok(ACES)
            [2, ..] -> Ok(KhronosPBRNeutral)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
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
