# Hash: 440cefc1f747d99747d97e35bd87f2cedc924785d42a6d5d2ba1368054edabcd
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::NumberKey
# Type category: Inline
# Commit: d505d37
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
