# Hash: 563298acd65b3eb2d0687614a209edddc959ae9ed30ba4d848a33001f28ef28a
# Generated: 2025-09-19T14:54:30+00:00
# Rust type: impact::input::key::KeyState
# Type category: Inline
# Commit: fc08276f (dirty)
module [
    KeyState,
    write_bytes,
    from_bytes,
]

## Whether a key is pressed or released.
KeyState : [
    Pressed,
    Released,
]

## Serializes a value of [KeyState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, KeyState -> List U8
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

## Deserializes a value of [KeyState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KeyState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Pressed)
            [1, ..] -> Ok(Released)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
