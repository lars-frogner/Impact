# Hash: 7a6b8c44ec95c205002e05d47d7affadc0e73c64cf216ff80be68afe43dd3482
# Generated: 2025-09-24T18:06:45+00:00
# Rust type: impact::command::scene::SceneCommand
# Type category: Inline
# Commit: ea3946bf (dirty)
module [
    SceneCommand,
    write_bytes,
    from_bytes,
]

import Command.ActiveState
import Entity
import Physics.UniformMedium
import Skybox

SceneCommand : [
    SetSkybox Skybox.Skybox,
    SetMedium Physics.UniformMedium.UniformMedium,
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
            |> List.reserve(33)
            |> List.append(0)
            |> Skybox.write_bytes(val)
            |> List.concat(List.repeat(0, 16))

        SetMedium(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(1)
            |> Physics.UniformMedium.write_bytes(val)

        SetSceneEntityActiveState { entity_id, state } ->
            bytes
            |> List.reserve(33)
            |> List.append(2)
            |> Entity.write_bytes_id(entity_id)
            |> Command.ActiveState.write_bytes(state)
            |> List.concat(List.repeat(0, 23))

## Deserializes a value of [SceneCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result SceneCommand _
from_bytes = |bytes|
    if List.len(bytes) != 33 then
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
                    SetMedium(
                        data_bytes |> List.sublist({ start: 0, len: 32 }) |> Physics.UniformMedium.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetSceneEntityActiveState     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        state: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.ActiveState.from_bytes?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
