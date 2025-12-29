# Hash: 5a2d951becb2d5cf
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact::input::key::FunctionKey
# Type category: Inline
module [
    FunctionKey,
    write_bytes,
    from_bytes,
]

FunctionKey : [
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
]

## Serializes a value of [FunctionKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, FunctionKey -> List U8
write_bytes = |bytes, value|
    when value is
        F1 ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        F2 ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        F3 ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        F4 ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        F5 ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

        F6 ->
            bytes
            |> List.reserve(1)
            |> List.append(5)

        F7 ->
            bytes
            |> List.reserve(1)
            |> List.append(6)

        F8 ->
            bytes
            |> List.reserve(1)
            |> List.append(7)

        F9 ->
            bytes
            |> List.reserve(1)
            |> List.append(8)

        F10 ->
            bytes
            |> List.reserve(1)
            |> List.append(9)

        F11 ->
            bytes
            |> List.reserve(1)
            |> List.append(10)

        F12 ->
            bytes
            |> List.reserve(1)
            |> List.append(11)

## Deserializes a value of [FunctionKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result FunctionKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(F1)
            [1, ..] -> Ok(F2)
            [2, ..] -> Ok(F3)
            [3, ..] -> Ok(F4)
            [4, ..] -> Ok(F5)
            [5, ..] -> Ok(F6)
            [6, ..] -> Ok(F7)
            [7, ..] -> Ok(F8)
            [8, ..] -> Ok(F9)
            [9, ..] -> Ok(F10)
            [10, ..] -> Ok(F11)
            [11, ..] -> Ok(F12)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
