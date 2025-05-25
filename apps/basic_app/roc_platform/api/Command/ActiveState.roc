# Hash: 424d5dfeefa60e8e2bc68aa1b6793e51ae4385d3406c842f5211418e4423c60e
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::engine::command::ActiveState
# Type category: Inline
# Commit: 31f3514 (dirty)
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
