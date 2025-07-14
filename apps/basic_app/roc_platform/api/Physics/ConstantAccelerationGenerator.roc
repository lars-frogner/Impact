# Hash: 30efbca7e2f75cad2269224e6e9d84b3e2c35f935546fdb430dd794710be5488
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::force::constant_acceleration::ConstantAccelerationGenerator
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    ConstantAccelerationGenerator,
    write_bytes,
    from_bytes,
]

import Comp.DynamicRigidBodyID
import Setup.ConstantAcceleration

## Generator for a constant world-space acceleration of the center of mass
## of a dynamic rigid body.
ConstantAccelerationGenerator : {
    ## The dynamic rigid body experiencing the acceleration.
    rigid_body_id : Comp.DynamicRigidBodyID.DynamicRigidBodyID,
    ## The acceleration of the body's center of mass in world space.
    acceleration : Setup.ConstantAcceleration.ConstantAcceleration,
}

## Serializes a value of [ConstantAccelerationGenerator] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAccelerationGenerator -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(32)
    |> Comp.DynamicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.ConstantAcceleration.write_bytes(value.acceleration)

## Deserializes a value of [ConstantAccelerationGenerator] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationGenerator _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.DynamicRigidBodyID.from_bytes?,
            acceleration: bytes |> List.sublist({ start: 8, len: 24 }) |> Setup.ConstantAcceleration.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 32 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
