# Hash: b486f17a300779b9
# Generated: 2026-02-01T20:43:19.603855004
# Rust type: impact_game::command::GameCommand
# Type category: Inline
module [
    GameCommand,
    write_bytes,
    from_bytes,
]

import Game.InteractionMode
import core.Builtin

GameCommand : [
    SetInteractionMode Game.InteractionMode.InteractionMode,
    AddMassToInventory F32,
    SetLauncherLaunchSpeed F32,
]

## Serializes a value of [GameCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GameCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetInteractionMode(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(0)
            |> Game.InteractionMode.write_bytes(val)
            |> List.concat(List.repeat(0, 3))

        AddMassToInventory(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(1)
            |> Builtin.write_bytes_f32(val)

        SetLauncherLaunchSpeed(val) ->
            bytes
            |> List.reserve(5)
            |> List.append(2)
            |> Builtin.write_bytes_f32(val)

## Deserializes a value of [GameCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GameCommand _
from_bytes = |bytes|
    if List.len(bytes) != 5 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetInteractionMode(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Game.InteractionMode.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    AddMassToInventory(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetLauncherLaunchSpeed(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
