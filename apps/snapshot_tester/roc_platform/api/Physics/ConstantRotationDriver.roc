# Hash: 4a2d3015123dbce955897f5ebb7f59c2751d68e34d27faef96691709d78d3347
# Generated: 2025-07-15T11:05:49+00:00
# Rust type: impact_physics::driven_motion::constant_rotation::ConstantRotationDriver
# Type category: POD
# Commit: 189570ab (dirty)
module [
    ConstantRotationDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.ConstantRotation

## Driver for imposing constant rotation on a kinematic rigid body.
ConstantRotationDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The constant rotation imposed on the body.
    rotation : Setup.ConstantRotation.ConstantRotation,
}

## Serializes a value of [ConstantRotationDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantRotationDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(80)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.ConstantRotation.write_bytes(value.rotation)

## Deserializes a value of [ConstantRotationDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantRotationDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            rotation: bytes |> List.sublist({ start: 8, len: 72 }) |> Setup.ConstantRotation.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 80 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
