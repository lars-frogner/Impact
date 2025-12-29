# Hash: b79e709b029a4033
# Generated: 2025-12-29T23:56:08.53639192
# Rust type: impact_rendering::postprocessing::capturing::dynamic_range_compression::ToneMappingMethod
# Type category: Inline
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
