# Hash: d8c6676bb7524c39
# Generated: 2025-12-29T23:54:14.852607239
# Rust type: impact::input::key::NavigationKey
# Type category: Inline
module [
    NavigationKey,
    write_bytes,
    from_bytes,
]

NavigationKey : [
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
]

## Serializes a value of [NavigationKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, NavigationKey -> List U8
write_bytes = |bytes, value|
    when value is
        Insert ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Home ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        End ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

        PageUp ->
            bytes
            |> List.reserve(1)
            |> List.append(3)

        PageDown ->
            bytes
            |> List.reserve(1)
            |> List.append(4)

## Deserializes a value of [NavigationKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result NavigationKey _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Insert)
            [1, ..] -> Ok(Home)
            [2, ..] -> Ok(End)
            [3, ..] -> Ok(PageUp)
            [4, ..] -> Ok(PageDown)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
