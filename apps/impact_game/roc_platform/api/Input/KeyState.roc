# Hash: 981797c11b59769a
# Generated: 2026-01-14T16:43:28.245815688
# Rust type: impact::input::key::KeyState
# Type category: Inline
module [
    KeyState,
    write_bytes,
    from_bytes,
]

## The state of a key following a key event.
KeyState : [
    ## The key was pressed.
    Pressed,
    ## The key is being held down, emitting repeated events.
    Held,
    ## The key was released.
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

        Held ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Released ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [KeyState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KeyState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Pressed)
            [1, ..] -> Ok(Held)
            [2, ..] -> Ok(Released)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
