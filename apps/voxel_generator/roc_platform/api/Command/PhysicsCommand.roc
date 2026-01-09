# Hash: a4bfc161d2ef6127
# Generated: 2026-01-09T10:50:30.908715526
# Rust type: impact::command::physics::PhysicsCommand
# Type category: Inline
module [
    PhysicsCommand,
    write_bytes,
    from_bytes,
]

import Comp.LocalForceGeneratorID
import core.Vector3

PhysicsCommand : [
    SetLocalForce {
            generator_id : Comp.LocalForceGeneratorID.LocalForceGeneratorID,
            force : Vector3.Vector3,
        },
]

## Serializes a value of [PhysicsCommand] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, PhysicsCommand -> List U8
write_bytes = |bytes, value|
    when value is
        SetLocalForce { generator_id, force } ->
            bytes
            |> List.reserve(21)
            |> List.append(0)
            |> Comp.LocalForceGeneratorID.write_bytes(generator_id)
            |> Vector3.write_bytes(force)

## Deserializes a value of [PhysicsCommand] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result PhysicsCommand _
from_bytes = |bytes|
    if List.len(bytes) != 21 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    SetLocalForce     {
                        generator_id: data_bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.LocalForceGeneratorID.from_bytes?,
                        force: data_bytes |> List.sublist({ start: 8, len: 12 }) |> Vector3.from_bytes?,
                    },
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
