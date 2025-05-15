# Hash: e325e7cffe786c6c3baaa70b74abdb06583ee3d7c35fbb557b00d7f64527c159
# Generated: 2025-05-14T18:52:22+00:00
# Rust type: impact::scene::command::SceneCommand
# Type category: Inline
# Commit: d505d37
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
            entity : Entity.Id,
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

        SetSceneEntityActiveState { entity, state } ->
            bytes
            |> List.reserve(10)
            |> List.append(1)
            |> Entity.write_bytes_id(entity)
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
                        entity: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        state: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.ActiveState.from_bytes?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            _ -> Err(InvalidDiscriminant)

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    test_roundtrip_for_variant(0, 9, 1)?
    test_roundtrip_for_variant(1, 10, 0)?
    Ok({})

test_roundtrip_for_variant : U8, U64, U64 -> Result {} _
test_roundtrip_for_variant = |discriminant, variant_size, padding_size|
    bytes = 
        List.range({ start: At discriminant, end: Length variant_size })
        |> List.concat(List.repeat(0, padding_size))
        |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
