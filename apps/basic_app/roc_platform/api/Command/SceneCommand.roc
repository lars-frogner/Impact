# Hash: d32b91bc9a20e66458ecc6894a5b849d46994f32a9811bcf7216bca44da64002
# Generated: 2025-05-23T18:55:01+00:00
# Rust type: impact::scene::command::SceneCommand
# Type category: Inline
# Commit: 31f3514 (dirty)
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


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
