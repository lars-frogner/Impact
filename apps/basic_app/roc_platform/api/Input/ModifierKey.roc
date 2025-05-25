# Hash: 291dd7f24924cf310ad0681a1030253c8c4fcb9788ce1671313105e4384e61fe
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::window::input::key::ModifierKey
# Type category: Inline
# Commit: 31f3514 (dirty)
module [
    ModifierKey,
    write_bytes,
    from_bytes,
]

ModifierKey : [
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
]

## Serializes a value of [ModifierKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ModifierKey -> List U8
write_bytes = |bytes, value|
    when value is
        ShiftLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        ShiftRight ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        ControlLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        ControlRight ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        AltLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        AltRight ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        SuperLeft ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        SuperRight ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

## Deserializes a value of [ModifierKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ModifierKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(ShiftLeft)
            [1, ..] -> Ok(ShiftRight)
            [2, ..] -> Ok(ControlLeft)
            [3, ..] -> Ok(ControlRight)
            [4, ..] -> Ok(AltLeft)
            [5, ..] -> Ok(AltRight)
            [6, ..] -> Ok(SuperLeft)
            [7, ..] -> Ok(SuperRight)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
