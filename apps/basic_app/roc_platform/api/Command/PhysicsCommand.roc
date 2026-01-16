# Hash: f6cb590ecc551871
# Generated: 2026-01-16T08:38:40.825032648
# Rust type: impact::command::physics::PhysicsCommand
# Type category: Inline
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.LocalForceUpdateMode
import Comp.AlignmentTorqueGeneratorID
import Comp.LocalForceGeneratorID
import Physics.AlignmentDirection
import core.Vector3

PhysicsCommand : [
    UpdateLocalForce {
            generator_id : Comp.LocalForceGeneratorID.LocalForceGeneratorID,
            mode : Command.LocalForceUpdateMode.LocalForceUpdateMode,
            force : Vector3.Vector3,
        },
    SetAlignmentTorqueDirection {
            generator_id : Comp.AlignmentTorqueGeneratorID.AlignmentTorqueGeneratorID,
            direction : Physics.AlignmentDirection.AlignmentDirection,
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

        SetAlignmentTorqueDirection { generator_id, direction } ->
            bytes
            |> List.reserve(22)
            |> List.append(1)
            |> Comp.AlignmentTorqueGeneratorID.write_bytes(generator_id)
            |> Physics.AlignmentDirection.write_bytes(direction)

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

            [1, .. as data_bytes] ->
                Ok(
                    SetAlignmentTorqueDirection     {
                        generator_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.AlignmentTorqueGeneratorID.from_bytes?,
                        direction: data_bytes |> List.sublist({ start: 8, len: 13 }) |> Physics.AlignmentDirection.from_bytes?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
