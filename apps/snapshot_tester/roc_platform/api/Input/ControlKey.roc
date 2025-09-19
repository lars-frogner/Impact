# Hash: 3f9fa523717532c390fcd6b06d6a1af14690788d762e2e6aa620501d3c4b89fa
# Generated: 2025-09-19T14:54:30+00:00
# Rust type: impact::input::key::ControlKey
# Type category: Inline
# Commit: fc08276f (dirty)
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
            [discr, ..] -> Err(InvalidDiscriminant(discr))
