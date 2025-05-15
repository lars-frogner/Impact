# Hash: 4237f043feb55b9d61dea64d028073d7fa167037daa4944b8b0af09ad38d72cd
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::NumpadKey
# Type category: Inline
# Commit: d505d37
module [
    NumpadKey,
    write_bytes,
    from_bytes,
]


NumpadKey : [
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
]

## Serializes a value of [NumpadKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NumpadKey -> List U8
write_bytes = |bytes, value|
    when value is
        Numpad0 ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Numpad1 ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Numpad2 ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        Numpad3 ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        Numpad4 ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        Numpad5 ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        Numpad6 ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        Numpad7 ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        Numpad8 ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        Numpad9 ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        NumpadAdd ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

        NumpadSubtract ->
            bytes
            |> List.reserve(1)
            |> List.append(11)

        NumpadMultiply ->
            bytes
            |> List.reserve(1)
            |> List.append(12)

        NumpadDivide ->
            bytes
            |> List.reserve(1)
            |> List.append(13)

        NumpadEnter ->
            bytes
            |> List.reserve(1)
            |> List.append(14)

        NumpadDecimal ->
            bytes
            |> List.reserve(1)
            |> List.append(15)

## Deserializes a value of [NumpadKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NumpadKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Numpad0)
            [1, ..] -> Ok(Numpad1)
            [2, ..] -> Ok(Numpad2)
            [3, ..] -> Ok(Numpad3)
            [4, ..] -> Ok(Numpad4)
            [5, ..] -> Ok(Numpad5)
            [6, ..] -> Ok(Numpad6)
            [7, ..] -> Ok(Numpad7)
            [8, ..] -> Ok(Numpad8)
            [9, ..] -> Ok(Numpad9)
            [10, ..] -> Ok(NumpadAdd)
            [11, ..] -> Ok(NumpadSubtract)
            [12, ..] -> Ok(NumpadMultiply)
            [13, ..] -> Ok(NumpadDivide)
            [14, ..] -> Ok(NumpadEnter)
            [15, ..] -> Ok(NumpadDecimal)
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
    test_roundtrip_for_variant(12, 1, 0)?
    test_roundtrip_for_variant(13, 1, 0)?
    test_roundtrip_for_variant(14, 1, 0)?
    test_roundtrip_for_variant(15, 1, 0)?
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
