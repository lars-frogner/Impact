# Hash: e1e98944a8ab1ea1f846b176c45c5faadeea66232a4128a3daba526d484baf2a
# Generated: 2025-07-27T14:53:54+00:00
# Rust type: impact::command::scene::SceneCommand
# Type category: Inline
# Commit: 397d36d3 (dirty)
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
            |> List.reserve(10)
            |> List.append(0)
            |> Skybox.write_bytes(val)
            |> List.concat(List.repeat(0, 1))

        SetSceneEntityActiveState { entity_id, state } ->
            bytes
            |> List.reserve(10)
            |> List.append(1)
            |> Entity.write_bytes_id(entity_id)
            |> Command.ActiveState.write_bytes(state)

        Clear ->
            bytes
            |> List.reserve(10)
            |> List.append(2)
            |> List.concat(List.repeat(0, 9))

## Deserializes a value of [SceneCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneCommand _
from_bytes = |bytes|
    if List.len(bytes) != 10 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetSkybox(
                        data_bytes |> List.sublist({ start: 0, len: 8 }) |> Skybox.from_bytes?,
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
