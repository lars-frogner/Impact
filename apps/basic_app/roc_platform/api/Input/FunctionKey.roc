# Hash: 4f24e9718320c43d25a989df6e4ee1bf356185ba507fb32fd4d89ad1b67aaed5
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::FunctionKey
# Type category: Inline
# Commit: d505d37
module [
    FunctionKey,
    write_bytes,
    from_bytes,
]


FunctionKey : [
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
]

## Serializes a value of [FunctionKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FunctionKey -> List U8
write_bytes = |bytes, value|
    when value is
        F1 ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        F2 ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        F3 ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        F4 ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        F5 ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        F6 ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        F7 ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        F8 ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        F9 ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        F10 ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        F11 ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

        F12 ->
            bytes
            |> List.reserve(1)
            |> List.append(11)

## Deserializes a value of [FunctionKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FunctionKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(F1)
            [1, ..] -> Ok(F2)
            [2, ..] -> Ok(F3)
            [3, ..] -> Ok(F4)
            [4, ..] -> Ok(F5)
            [5, ..] -> Ok(F6)
            [6, ..] -> Ok(F7)
            [7, ..] -> Ok(F8)
            [8, ..] -> Ok(F9)
            [9, ..] -> Ok(F10)
            [10, ..] -> Ok(F11)
            [11, ..] -> Ok(F12)
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
    test_roundtrip_for_variant(11, 1, 0)?
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
