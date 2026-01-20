# Hash: 6976f9646a268681
# Generated: 2026-01-18T10:23:28.079823297
# Rust type: impact_game::PlayerMode
# Type category: Inline
module [
    PlayerMode,
    write_bytes,
    from_bytes,
]

PlayerMode : [
    Active,
    Overview,
]

## Serializes a value of [PlayerMode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlayerMode -> List U8
write_bytes = |bytes, value|
    when value is
        Active ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        Overview ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

## Deserializes a value of [PlayerMode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlayerMode _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Active)
            [1, ..] -> Ok(Overview)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
