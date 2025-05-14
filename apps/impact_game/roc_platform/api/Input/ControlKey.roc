# Hash: 26cbd7824c567c8b0471b61f915a976740c38d9eb2d6bf8bae4edfea9ea6a2a7
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::ControlKey
# Type category: Inline
# Commit: d505d37
module [
    ControlKey,
    write_bytes,
    from_bytes,
]


ControlKey : [
    Escape,
    Backspace,
    Delete,
]

## Serializes a value of [ControlKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ControlKey -> List U8
write_bytes = |bytes, value|
    when value is
        Escape ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Backspace ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Delete ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [ControlKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ControlKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Escape)
            [1, ..] -> Ok(Backspace)
            [2, ..] -> Ok(Delete)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
    test_roundtrip_for_variant(2, 1, 0)?
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
