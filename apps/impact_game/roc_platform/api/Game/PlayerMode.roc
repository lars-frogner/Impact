# Hash: 7df0787e8fe292a5
# Generated: 2026-01-23T22:12:11.221480608
# Rust type: impact_game::PlayerMode
# Type category: Inline
module [
    PlayerMode,
    write_bytes,
    from_bytes,
]

PlayerMode : [
    Dynamic,
    FreeCamera,
    OverviewCamera,
]

## Serializes a value of [PlayerMode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PlayerMode -> List U8
write_bytes = |bytes, value|
    when value is
        Dynamic ->
            bytes
            |> List.reserve(1)
            |> List.append(0)

        FreeCamera ->
            bytes
            |> List.reserve(1)
            |> List.append(1)

        OverviewCamera ->
            bytes
            |> List.reserve(1)
            |> List.append(2)

## Deserializes a value of [PlayerMode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PlayerMode _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Dynamic)
            [1, ..] -> Ok(FreeCamera)
            [2, ..] -> Ok(OverviewCamera)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
