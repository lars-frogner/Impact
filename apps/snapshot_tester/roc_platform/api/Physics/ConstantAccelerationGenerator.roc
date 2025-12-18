# Hash: d85af0183d4067e884baecf0310e4234ed9271d7380c75e7794092b811e8a128
# Generated: 2025-12-17T23:58:42+00:00
# Rust type: impact_physics::force::constant_acceleration::ConstantAccelerationGenerator
# Type category: POD
# Commit: 7d41822d (dirty)
module [
    ConstantAccelerationGenerator,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Setup.ConstantAcceleration
import core.Builtin

## Generator for a constant world-space acceleration of the center of mass
## of a dynamic rigid body.
ConstantAccelerationGenerator : {
    ## The dynamic rigid body experiencing the acceleration.
    rigid_body_id : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The acceleration of the body's center of mass in world space.
    acceleration : Setup.ConstantAcceleration.ConstantAcceleration,
    padding : F32,
}

## Serializes a value of [ConstantAccelerationGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAccelerationGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(24)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.ConstantAcceleration.write_bytes(value.acceleration)
    |> Builtin.write_bytes_f32(value.padding)

## Deserializes a value of [ConstantAccelerationGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationGenerator _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            acceleration: bytes |> List.sublist({ start: 8, len: 12 }) |> Setup.ConstantAcceleration.from_bytes?,
            padding: bytes |> List.sublist({ start: 20, len: 4 }) |> Builtin.from_bytes_f32?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 24 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
