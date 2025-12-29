# Hash: ccda81fd0436c5e5
# Generated: 2025-12-29T23:55:22.755341756
# Rust type: impact::command::uils::ActiveState
# Type category: Inline
module [
    ActiveState,
    write_bytes,
    from_bytes,
]

ActiveState : [
    Enabled,
    Disabled,
]

## Serializes a value of [ActiveState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ActiveState -> List U8
write_bytes = |bytes, value|
    when value is
        Enabled ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Disabled ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [ActiveState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ActiveState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Enabled)
            [1, ..] -> Ok(Disabled)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
