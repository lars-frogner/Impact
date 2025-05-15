# Hash: 6d2005a64466e0c79fcd99d5b50ab9d8150a674949fe1ce5b493ec385b5bbae6
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::window::input::key::LockKey
# Type category: Inline
# Commit: d505d37
module [
    LockKey,
    write_bytes,
    from_bytes,
]


LockKey : [
    CapsLock,
    NumLock,
]

## Serializes a value of [LockKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, LockKey -> List U8
write_bytes = |bytes, value|
    when value is
        CapsLock ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        NumLock ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [LockKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result LockKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(CapsLock)
            [1, ..] -> Ok(NumLock)
            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 1, 0)?
    test_roundtrip_for_variant(1, 1, 0)?
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
