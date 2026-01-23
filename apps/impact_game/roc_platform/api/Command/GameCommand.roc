# Hash: 6ec674ec42064cfa
# Generated: 2026-01-23T21:17:57.478883102
# Rust type: impact_game::command::GameCommand
# Type category: Inline
module [
    GameCommand,
    write_bytes,
    from_bytes,
]

import Game.PlayerMode

GameCommand : [
    SetPlayerMode Game.PlayerMode.PlayerMode,
]

## Serializes a value of [GameCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GameCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetPlayerMode(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Game.PlayerMode.write_bytes(val)

## Deserializes a value of [GameCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GameCommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetPlayerMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Game.PlayerMode.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
