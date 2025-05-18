# Hash: af1b408ccebea2a541a46f23f78249bd8848125d4100753914e7bc708387c55e
# Generated: 2025-05-18T21:33:59+00:00
# Rust type: impact::ui::command::ToInteractionMode
# Type category: Inline
# Commit: c6462c2 (dirty)
module [
    ToInteractionMode,
    write_bytes,
    from_bytes,
]

ToInteractionMode : [
    Control,
    Cursor,
    Opposite,
]

## Serializes a value of [ToInteractionMode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToInteractionMode -> List U8
write_bytes = |bytes, value|
    when value is
        Control ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Cursor ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Opposite ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [ToInteractionMode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToInteractionMode _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Control)
            [1, ..] -> Ok(Cursor)
            [2, ..] -> Ok(Opposite)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
