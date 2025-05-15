# Hash: 4717501196bf801884ff05da909c91cc730c7528a7d23e77c789bdb8c9f14ee7
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::SymbolKey
# Type category: Inline
# Commit: d505d37
module [
    SymbolKey,
    write_bytes,
    from_bytes,
]


SymbolKey : [
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Backquote,
]

## Serializes a value of [SymbolKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SymbolKey -> List U8
write_bytes = |bytes, value|
    when value is
        Minus ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Equal ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        BracketLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        BracketRight ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        Backslash ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        Semicolon ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        Quote ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        Comma ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        Period ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        Slash ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        Backquote ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

## Deserializes a value of [SymbolKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SymbolKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Minus)
            [1, ..] -> Ok(Equal)
            [2, ..] -> Ok(BracketLeft)
            [3, ..] -> Ok(BracketRight)
            [4, ..] -> Ok(Backslash)
            [5, ..] -> Ok(Semicolon)
            [6, ..] -> Ok(Quote)
            [7, ..] -> Ok(Comma)
            [8, ..] -> Ok(Period)
            [9, ..] -> Ok(Slash)
            [10, ..] -> Ok(Backquote)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
    test_roundtrip_for_variant(3, 1, 0)?
    test_roundtrip_for_variant(4, 1, 0)?
    test_roundtrip_for_variant(5, 1, 0)?
    test_roundtrip_for_variant(6, 1, 0)?
    test_roundtrip_for_variant(7, 1, 0)?
    test_roundtrip_for_variant(8, 1, 0)?
    test_roundtrip_for_variant(9, 1, 0)?
    test_roundtrip_for_variant(10, 1, 0)?
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
