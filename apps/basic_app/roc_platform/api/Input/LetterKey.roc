# Hash: 4b9488dc4bc1b5443d5f10a9e6b92083c7943fb2c2cdf3a94e109a449750c1b1
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::LetterKey
# Type category: Inline
# Commit: d505d37
module [
    LetterKey,
    write_bytes,
    from_bytes,
]


LetterKey : [
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
]

## Serializes a value of [LetterKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LetterKey -> List U8
write_bytes = |bytes, value|
    when value is
        KeyA ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        KeyB ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        KeyC ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        KeyD ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        KeyE ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        KeyF ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        KeyG ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        KeyH ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        KeyI ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        KeyJ ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        KeyK ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

        KeyL ->
            bytes
            |> List.reserve(1)
            |> List.append(11)

        KeyM ->
            bytes
            |> List.reserve(1)
            |> List.append(12)

        KeyN ->
            bytes
            |> List.reserve(1)
            |> List.append(13)

        KeyO ->
            bytes
            |> List.reserve(1)
            |> List.append(14)

        KeyP ->
            bytes
            |> List.reserve(1)
            |> List.append(15)

        KeyQ ->
            bytes
            |> List.reserve(1)
            |> List.append(16)

        KeyR ->
            bytes
            |> List.reserve(1)
            |> List.append(17)

        KeyS ->
            bytes
            |> List.reserve(1)
            |> List.append(18)

        KeyT ->
            bytes
            |> List.reserve(1)
            |> List.append(19)

        KeyU ->
            bytes
            |> List.reserve(1)
            |> List.append(20)

        KeyV ->
            bytes
            |> List.reserve(1)
            |> List.append(21)

        KeyW ->
            bytes
            |> List.reserve(1)
            |> List.append(22)

        KeyX ->
            bytes
            |> List.reserve(1)
            |> List.append(23)

        KeyY ->
            bytes
            |> List.reserve(1)
            |> List.append(24)

        KeyZ ->
            bytes
            |> List.reserve(1)
            |> List.append(25)

## Deserializes a value of [LetterKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LetterKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(KeyA)
            [1, ..] -> Ok(KeyB)
            [2, ..] -> Ok(KeyC)
            [3, ..] -> Ok(KeyD)
            [4, ..] -> Ok(KeyE)
            [5, ..] -> Ok(KeyF)
            [6, ..] -> Ok(KeyG)
            [7, ..] -> Ok(KeyH)
            [8, ..] -> Ok(KeyI)
            [9, ..] -> Ok(KeyJ)
            [10, ..] -> Ok(KeyK)
            [11, ..] -> Ok(KeyL)
            [12, ..] -> Ok(KeyM)
            [13, ..] -> Ok(KeyN)
            [14, ..] -> Ok(KeyO)
            [15, ..] -> Ok(KeyP)
            [16, ..] -> Ok(KeyQ)
            [17, ..] -> Ok(KeyR)
            [18, ..] -> Ok(KeyS)
            [19, ..] -> Ok(KeyT)
            [20, ..] -> Ok(KeyU)
            [21, ..] -> Ok(KeyV)
            [22, ..] -> Ok(KeyW)
            [23, ..] -> Ok(KeyX)
            [24, ..] -> Ok(KeyY)
            [25, ..] -> Ok(KeyZ)
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
    test_roundtrip_for_variant(16, 1, 0)?
    test_roundtrip_for_variant(17, 1, 0)?
    test_roundtrip_for_variant(18, 1, 0)?
    test_roundtrip_for_variant(19, 1, 0)?
    test_roundtrip_for_variant(20, 1, 0)?
    test_roundtrip_for_variant(21, 1, 0)?
    test_roundtrip_for_variant(22, 1, 0)?
    test_roundtrip_for_variant(23, 1, 0)?
    test_roundtrip_for_variant(24, 1, 0)?
    test_roundtrip_for_variant(25, 1, 0)?
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
