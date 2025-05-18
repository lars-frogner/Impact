# Hash: 6915a8f327416245c028d6e5663014ceec6a8a8a3c5c48a0c3f7c85d9255f760
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::window::input::key::NumberKey
# Type category: Inline
# Commit: c6462c2 (dirty)
module [
    NumberKey,
    write_bytes,
    from_bytes,
]

NumberKey : [
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
]

## Serializes a value of [NumberKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NumberKey -> List U8
write_bytes = |bytes, value|
    when value is
        Digit0 ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Digit1 ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Digit2 ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        Digit3 ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        Digit4 ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        Digit5 ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        Digit6 ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        Digit7 ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        Digit8 ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        Digit9 ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

## Deserializes a value of [NumberKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NumberKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Digit0)
            [1, ..] -> Ok(Digit1)
            [2, ..] -> Ok(Digit2)
            [3, ..] -> Ok(Digit3)
            [4, ..] -> Ok(Digit4)
            [5, ..] -> Ok(Digit5)
            [6, ..] -> Ok(Digit6)
            [7, ..] -> Ok(Digit7)
            [8, ..] -> Ok(Digit8)
            [9, ..] -> Ok(Digit9)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
