# Hash: f94f0ece1762a46a7aa2cc2c548a6bd47638b12346c5b323eb079019db9b9522
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::WhitespaceKey
# Type category: Inline
# Commit: d505d37
module [
    WhitespaceKey,
    write_bytes,
    from_bytes,
]


WhitespaceKey : [
    Space,
    Tab,
    Enter,
]

## Serializes a value of [WhitespaceKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, WhitespaceKey -> List U8
write_bytes = |bytes, value|
    when value is
        Space ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Tab ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Enter ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [WhitespaceKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result WhitespaceKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Space)
            [1, ..] -> Ok(Tab)
            [2, ..] -> Ok(Enter)
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
