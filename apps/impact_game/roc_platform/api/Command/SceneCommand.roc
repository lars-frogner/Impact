# Hash: ec736be43d48522d
# Generated: 2026-01-21T20:15:20.637171438
# Rust type: impact::command::scene::SceneCommand
# Type category: Inline
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
    SetActiveCamera {
            entity_id : Entity.Id,
        },
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
        SetActiveCamera { entity_id } ->
            bytes
            |> List.reserve(17)
            |> List.append(0)
            |> Entity.write_bytes_id(entity_id)
            |> List.concat(List.repeat(0, 8))

        SetSkybox(val) ->
            bytes
            |> List.reserve(17)
            |> List.append(1)
            |> Skybox.write_bytes(val)

        SetMedium(val) ->
            bytes
            |> List.reserve(17)
            |> List.append(2)
            |> Physics.UniformMedium.write_bytes(val)

        SetSceneEntityActiveState { entity_id, state } ->
            bytes
            |> List.reserve(17)
            |> List.append(3)
            |> Entity.write_bytes_id(entity_id)
            |> Command.ActiveState.write_bytes(state)
            |> List.concat(List.repeat(0, 7))

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
                    SetActiveCamera     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                    },
                )

            [1, .. as data_bytes] ->
                Ok(
                    SetSkybox(
                        data_bytes |> List.sublist({ start: 0, len: 16 }) |> Skybox.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetMedium(
                        data_bytes |> List.sublist({ start: 0, len: 16 }) |> Physics.UniformMedium.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    SetSceneEntityActiveState     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        state: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.ActiveState.from_bytes?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
