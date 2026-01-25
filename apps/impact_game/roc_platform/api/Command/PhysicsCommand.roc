# Hash: b20b7ca92e826e82
# Generated: 2026-01-24T10:14:23.759286812
# Rust type: impact::command::physics::PhysicsCommand
# Type category: Inline
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Command.LocalForceUpdateMode
import Entity
import Physics.AlignmentDirection
import core.Builtin
import core.Point3
import core.Vector3

PhysicsCommand : [
    SetGravitationalConstant F32,
    UpdateLocalForce {
            entity_id : Entity.Id,
            mode : Command.LocalForceUpdateMode.LocalForceUpdateMode,
            force : Vector3.Vector3,
        },
    SetAlignmentTorqueDirection {
            entity_id : Entity.Id,
            direction : Physics.AlignmentDirection.AlignmentDirection,
        },
    ApplyImpulse {
            entity_id : Entity.Id,
            impulse : Vector3.Vector3,
            relative_position : Point3.Point3,
        },
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetGravitationalConstant(val) ->
            bytes
            |> List.reserve(33)
            |> List.append(0)
            |> Builtin.write_bytes_f32(val)
            |> List.concat(List.repeat(0, 28))

        UpdateLocalForce { entity_id, mode, force } ->
            bytes
            |> List.reserve(33)
            |> List.append(1)
            |> Entity.write_bytes_id(entity_id)
            |> Command.LocalForceUpdateMode.write_bytes(mode)
            |> Vector3.write_bytes(force)
            |> List.concat(List.repeat(0, 11))

        SetAlignmentTorqueDirection { entity_id, direction } ->
            bytes
            |> List.reserve(33)
            |> List.append(2)
            |> Entity.write_bytes_id(entity_id)
            |> Physics.AlignmentDirection.write_bytes(direction)
            |> List.concat(List.repeat(0, 11))

        ApplyImpulse { entity_id, impulse, relative_position } ->
            bytes
            |> List.reserve(33)
            |> List.append(3)
            |> Entity.write_bytes_id(entity_id)
            |> Vector3.write_bytes(impulse)
            |> Point3.write_bytes(relative_position)

## Deserializes a value of [PhysicsCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PhysicsCommand _
from_bytes = |bytes|
    if List.len(bytes) != 33 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetGravitationalConstant(
                        data_bytes |> List.sublist({ start: 0, len: 4 }) |> Builtin.from_bytes_f32?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    UpdateLocalForce     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        mode: data_bytes |> List.sublist({ start: 8, len: 1 }) |> Command.LocalForceUpdateMode.from_bytes?,
                        force: data_bytes |> List.sublist({ start: 9, len: 12 }) |> Vector3.from_bytes?,
                    },
                )


            [2, .. as data_bytes] ->
                Ok(
                    SetAlignmentTorqueDirection     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        direction: data_bytes |> List.sublist({ start: 8, len: 13 }) |> Physics.AlignmentDirection.from_bytes?,
                    },
                )


            [3, .. as data_bytes] ->
                Ok(
                    ApplyImpulse     {
                        entity_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Entity.from_bytes_id?,
                        impulse: data_bytes |> List.sublist({ start: 8, len: 12 }) |> Vector3.from_bytes?,
                        relative_position: data_bytes |> List.sublist({ start: 20, len: 12 }) |> Point3.from_bytes?,
                    },
                )


            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
