# Hash: 42aaed1c3b516b4c682b86c842ac8889737c4b0d1ce0dc994668099fd6cfde19
# Generated: 2025-05-15T13:42:14+00:00
# Rust type: impact::window::input::mouse::MouseButtonState
# Type category: Inline
# Commit: 1e7723b (dirty)
module [
    MouseButtonState,
    write_bytes,
    from_bytes,
]


## Whether a mouse button is pressed or released.
MouseButtonState : [
    Pressed,
    Released,
]

## Serializes a value of [MouseButtonState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseButtonState -> List U8
write_bytes = |bytes, value|
    when value is
        Pressed ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Released ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [MouseButtonState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseButtonState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Pressed)
            [1, ..] -> Ok(Released)
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
