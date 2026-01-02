# Hash: 3173bf7aa58c85d8
# Generated: 2026-01-01T09:41:16.584947407
# Rust type: impact::input::key::ArrowKey
# Type category: Inline
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
