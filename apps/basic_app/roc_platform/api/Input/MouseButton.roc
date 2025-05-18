# Hash: 2a3acd34988e8d48bb96c7049b57bb3a7f9eb014ac3e85d408239054f9937d5b
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::window::input::mouse::MouseButton
# Type category: Inline
# Commit: c6462c2 (dirty)
module [
    MouseButton,
    write_bytes,
    from_bytes,
]

## A button on a mouse.
MouseButton : [
    Left,
    Right,
    Middle,
]

## Serializes a value of [MouseButton] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, MouseButton -> List U8
write_bytes = |bytes, value|
    when value is
        Left ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Right ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Middle ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [MouseButton] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result MouseButton _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Left)
            [1, ..] -> Ok(Right)
            [2, ..] -> Ok(Middle)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
