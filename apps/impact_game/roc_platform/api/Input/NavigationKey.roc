# Hash: b3e6f694f047c22139eae4db0e66fc89cf10fd965bba2549f7892b9302b5b07d
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::NavigationKey
# Type category: Inline
# Commit: d505d37
module [
    NavigationKey,
    write_bytes,
    from_bytes,
]


NavigationKey : [
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
]

## Serializes a value of [NavigationKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NavigationKey -> List U8
write_bytes = |bytes, value|
    when value is
        Insert ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Home ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        End ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        PageUp ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        PageDown ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

## Deserializes a value of [NavigationKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NavigationKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Insert)
            [1, ..] -> Ok(Home)
            [2, ..] -> Ok(End)
            [3, ..] -> Ok(PageUp)
            [4, ..] -> Ok(PageDown)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
    test_roundtrip_for_variant(3, 1, 0)?
    test_roundtrip_for_variant(4, 1, 0)?
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
