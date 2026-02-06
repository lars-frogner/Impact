# Hash: 01cb35d9c8c3cc65
# Generated: 2026-02-06T19:52:17.213390491
# Rust type: impact_game::lookup::GameLookupTarget
# Type category: Inline
module [
    GameLookupTarget,
    lookup!,
    write_bytes,
    from_bytes,
]

import Entity
import Lookup

GameLookupTarget : [
    InventoryMass,
    SphereAbsorbedVoxelMass {
            entity_id : Entity.Id,
        },
    CapsuleAbsorbedVoxelMass {
            entity_id : Entity.Id,
        },
    LauncherLaunchSpeed,
]

lookup! : GameLookupTarget, _ => _
lookup! = |self, decode|
    [] |> write_bytes(self) |> Lookup.lookup!(decode)

## Serializes a value of [GameLookupTarget] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, GameLookupTarget -> List U8
write_bytes = |bytes, value|
    when value is
        InventoryMass ->
            bytes
            |> List.reserve(9)
            |> List.append(0)
            |> List.concat(List.repeat(0, 8))

        SphereAbsorbedVoxelMass { entity_id } ->
            bytes
            |> List.reserve(9)
            |> List.append(1)
            |> Entity.write_bytes_id(entity_id)

        CapsuleAbsorbedVoxelMass { entity_id } ->
            bytes
            |> List.reserve(9)
            |> List.append(2)
            |> Entity.write_bytes_id(entity_id)

        LauncherLaunchSpeed ->
            bytes
            |> List.reserve(9)
            |> List.append(3)
            |> List.concat(List.repeat(0, 8))

## Deserializes a value of [GameLookupTarget] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result GameLookupTarget _
from_bytes = |bytes|
    if List.len(bytes) != 9 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, ..] -> Ok(InventoryMass)
            [1, .. as data_bytes] ->
                Ok(
                    SphereAbsorbedVoxelMass     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                    },
                )


            [2, .. as data_bytes] ->
                Ok(
                    CapsuleAbsorbedVoxelMass     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                    },
                )


            [3, ..] -> Ok(LauncherLaunchSpeed)
            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
