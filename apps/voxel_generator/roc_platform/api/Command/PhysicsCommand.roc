# Hash: 445e905dcc872978
# Generated: 2026-01-14T12:38:46.465732148
# Rust type: impact::command::physics::PhysicsCommand
# Type category: Inline
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.LocalForceUpdateMode
import Comp.LocalForceGeneratorID
import core.Vector3

PhysicsCommand : [
    UpdateLocalForce {
            generator_id : Comp.LocalForceGeneratorID.LocalForceGeneratorID,
            mode : Command.LocalForceUpdateMode.LocalForceUpdateMode,
            force : Vector3.Vector3,
        },
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        UpdateLocalForce { generator_id, mode, force } ->
            bytes
            |> List.reserve(22)
            |> List.append(0)
            |> Comp.LocalForceGeneratorID.write_bytes(generator_id)
            |> Command.LocalForceUpdateMode.write_bytes(mode)
            |> Vector3.write_bytes(force)

## Deserializes a value of [PhysicsCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PhysicsCommand _
from_bytes = |bytes|
    if List.len(bytes) != 22 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    UpdateLocalForce     {
                        generator_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.LocalForceGeneratorID.from_bytes?,
                        mode: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.LocalForceUpdateMode.from_bytes?,
                        force: data_bytes |> List.sublist({ start: 9, len: 12 }) |> Vector3.from_bytes?,
                    },
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
