# Hash: 7f7eee57d91b2a804b1eb39f7a9ab0ce80794d26b6a5e0e40db1fed634808fb8
# Generated: 2025-08-01T06:51:20+00:00
# Rust type: impact::command::scene::SceneCommand
# Type category: Inline
# Commit: 5cd592d6
module [
    SceneCommand,
    write_bytes,
    from_bytes,
]

import Command.ActiveState
import Entity
import Skybox

SceneCommand : [
    SetSkybox Skybox.Skybox,
    SetSceneEntityActiveState {
            entity_id : Entity.Id,
            state : Command.ActiveState.ActiveState,
        },
    Clear,
]

## Serializes a value of [SceneCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, SceneCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetSkybox(val) ->
            bytes
            |> List.reserve(17)
            |> List.append(0)
            |> Skybox.write_bytes(val)

        SetSceneEntityActiveState { entity_id, state } ->
            bytes
            |> List.reserve(17)
            |> List.append(1)
            |> Entity.write_bytes_id(entity_id)
            |> Command.ActiveState.write_bytes(state)
            |> List.concat(List.repeat(0, 7))

        Clear ->
            bytes
            |> List.reserve(17)
            |> List.append(2)
            |> List.concat(List.repeat(0, 16))

## Deserializes a value of [SceneCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneCommand _
from_bytes = |bytes|
    if List.len(bytes) != 17 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetSkybox(
                        data_bytes |> List.sublist({ start: 0, len: 16 }) |> Skybox.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetSceneEntityActiveState     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        state: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.ActiveState.from_bytes?,
                    },
                )


            [2, ..] -> Ok(Clear)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
