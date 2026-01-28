# Hash: 3d8b2139b521426d
# Generated: 2026-01-25T13:02:32.838920337
# Rust type: impact_game::InteractionMode
# Type category: Inline
module [
    InteractionMode,
    write_bytes,
    from_bytes,
]

InteractionMode : [
    Player,
    FreeCamera,
    OverviewCamera,
]

## Serializes a value of [InteractionMode] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, InteractionMode -> List U8
write_bytes = |bytes, value|
    when value is
        Player ->
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

## Deserializes a value of [InteractionMode] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result InteractionMode _
from_bytes = |bytes|
    if List.len(bytes) != 1 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(Player)
            [1, ..] -> Ok(FreeCamera)
            [2, ..] -> Ok(OverviewCamera)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
