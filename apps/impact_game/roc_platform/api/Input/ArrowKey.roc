# Hash: 71ce86a8ec4ed16a5b352608f5bfc463c13ed82b269f083ead6d4dd5b31d5568
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::ArrowKey
# Type category: Inline
# Commit: d505d37
module [
    ArrowKey,
    write_bytes,
    from_bytes,
]


ArrowKey : [
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
]

## Serializes a value of [ArrowKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ArrowKey -> List U8
write_bytes = |bytes, value|
    when value is
        ArrowUp ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        ArrowDown ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        ArrowLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        ArrowRight ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

## Deserializes a value of [ArrowKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ArrowKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(ArrowUp)
            [1, ..] -> Ok(ArrowDown)
            [2, ..] -> Ok(ArrowLeft)
            [3, ..] -> Ok(ArrowRight)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
    test_roundtrip_for_variant(3, 1, 0)?
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
