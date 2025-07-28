# Hash: 79278d5135c02d81901d3f26b554f7f0f933a56378fe5f6b35f1b3891d296208
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact_rendering::postprocessing::capturing::dynamic_range_compression::ToneMappingMethod
# Type category: Inline
# Commit: 397d36d3 (dirty)
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
