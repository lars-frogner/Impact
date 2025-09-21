# Hash: c400552531da66c4707709cf6c58ff5a355c3993bf82bc191eef78e6d09af393
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact::window::input::mouse::MouseButtonState
# Type category: Inline
# Commit: 397d36d3 (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
