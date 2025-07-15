# Hash: 25308ad19efb940fbb72d21f48656ff0b7a5347a1037af77ec19925b2f68e886
# Generated: 2025-07-15T17:32:17+00:00
# Rust type: impact::window::input::key::SymbolKey
# Type category: Inline
# Commit: 1fbb6f6b (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
