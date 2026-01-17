# Hash: 37e532f8bd970041
# Generated: 2026-01-17T13:07:38.150179169
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
import core.Builtin

SceneCommand : [
    SetSkybox Skybox.Skybox,
    SetMedium Physics.UniformMedium.UniformMedium,
    SetSceneEntityActiveState {
            entity_id : Entity.Id,
            state : Command.ActiveState.ActiveState,
        },
    SetMaxOmnidirectionalLightReach F32,
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

        SetMedium(val) ->
            bytes
            |> List.reserve(17)
            |> List.append(1)
            |> Physics.UniformMedium.write_bytes(val)

        SetSceneEntityActiveState { entity_id, state } ->
            bytes
            |> List.reserve(17)
            |> List.append(2)
            |> Entity.write_bytes_id(entity_id)
            |> Command.ActiveState.write_bytes(state)
            |> List.concat(List.repeat(0, 7))

        SetMaxOmnidirectionalLightReach(val) ->
            bytes
            |> List.reserve(17)
            |> List.append(3)
            |> Builtin.write_bytes_f32(val)
            |> List.concat(List.repeat(0, 12))

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
                    SetMedium(
                        data_bytes |> List.sublist({ start: 0, len: 16 }) |> Physics.UniformMedium.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    SetSceneEntityActiveState     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        state: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.ActiveState.from_bytes?,
                    },
                )


            [3, .. as data_bytes] ->
                Ok(
                    SetMaxOmnidirectionalLightReach(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
