# Hash: 9455dba21722a5d2d25673feddec736b04da35e55fb766f1ce8845645fb2d3e0
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::ModifierKey
# Type category: Inline
# Commit: d505d37
module [
    ModifierKey,
    write_bytes,
    from_bytes,
]


ModifierKey : [
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
]

## Serializes a value of [ModifierKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ModifierKey -> List U8
write_bytes = |bytes, value|
    when value is
        ShiftLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        ShiftRight ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        ControlLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        ControlRight ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        AltLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        AltRight ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        SuperLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        SuperRight ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

## Deserializes a value of [ModifierKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ModifierKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(ShiftLeft)
            [1, ..] -> Ok(ShiftRight)
            [2, ..] -> Ok(ControlLeft)
            [3, ..] -> Ok(ControlRight)
            [4, ..] -> Ok(AltLeft)
            [5, ..] -> Ok(AltRight)
            [6, ..] -> Ok(SuperLeft)
            [7, ..] -> Ok(SuperRight)
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
