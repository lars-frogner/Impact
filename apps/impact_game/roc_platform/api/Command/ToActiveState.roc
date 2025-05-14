# Hash: f95709b3a7e1a1f040bfe2c0300c9fa84ffa958895895d50382cf999aa746c51
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::engine::command::ToActiveState
# Type category: Inline
# Commit: d505d37
module [
    ToActiveState,
    write_bytes,
    from_bytes,
]


ToActiveState : [
    Enabled,
    Disabled,
    Opposite,
]

## Serializes a value of [ToActiveState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToActiveState -> List U8
write_bytes = |bytes, value|
    when value is
        Enabled ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Disabled ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Opposite ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [ToActiveState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToActiveState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Enabled)
            [1, ..] -> Ok(Disabled)
            [2, ..] -> Ok(Opposite)
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
