# Hash: 93cf77e5490074a1398be60badfae98f3b6f988bf24588b0492d9cdab836e311
# Generated: 2025-09-19T14:54:30+00:00
# Rust type: impact::input::key::WhitespaceKey
# Type category: Inline
# Commit: fc08276f (dirty)
module [
    WhitespaceKey,
    write_bytes,
    from_bytes,
]

WhitespaceKey : [
    Space,
    Tab,
    Enter,
]

## Serializes a value of [WhitespaceKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, WhitespaceKey -> List U8
write_bytes = |bytes, value|
    when value is
        Space ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Tab ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        Enter ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [WhitespaceKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result WhitespaceKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Space)
            [1, ..] -> Ok(Tab)
            [2, ..] -> Ok(Enter)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
