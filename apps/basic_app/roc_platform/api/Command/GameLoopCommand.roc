# Hash: bd5e01b05b2e8d446fe2f8a2dd0ba4d47b9c58b9bc8ac6a1cacce7a679a62420
# Generated: 2025-07-27T14:52:58+00:00
# Rust type: impact::command::game_loop::GameLoopCommand
# Type category: Inline
# Commit: 397d36d3 (dirty)
module [
    GameLoopCommand,
    write_bytes,
    from_bytes,
]

import Command.ToActiveState

GameLoopCommand : [
    SetGameLoop Command.ToActiveState.ToActiveState,
    PauseAfterSingleIteration,
]

## Serializes a value of [GameLoopCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GameLoopCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetGameLoop(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Command.ToActiveState.write_bytes(val)

        PauseAfterSingleIteration ->
            bytes
            |> List.reserve(2)
            |> List.append(1)
            |> List.concat(List.repeat(0, 1))

## Deserializes a value of [GameLoopCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GameLoopCommand _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetGameLoop(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Command.ToActiveState.from_bytes?,
                    ),
                )

            [1, ..] -> Ok(PauseAfterSingleIteration)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
