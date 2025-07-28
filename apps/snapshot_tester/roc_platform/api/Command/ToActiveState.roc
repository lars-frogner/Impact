# Hash: 7f814adca479874a13e78291868bfe1d0fd21daee0c3cf6eb68f83475b09b0c3
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact::command::uils::ToActiveState
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    ToActiveState,
    write_bytes,
    from_bytes,
]

ToActiveState : [
    Enabled,
    Disabled,
    Opposite,
]

## Serializes a value of [ToActiveState] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ToActiveState -> List U8
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

        Opposite ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [ToActiveState] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ToActiveState _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Enabled)
            [1, ..] -> Ok(Disabled)
            [2, ..] -> Ok(Opposite)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
