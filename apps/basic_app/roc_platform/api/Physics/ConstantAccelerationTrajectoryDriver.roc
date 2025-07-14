# Hash: 3166646218bb8c629cd0a588f191c294e4db35028bf14fd895fe2395197d499f
# Generated: 2025-07-13T19:49:53+00:00
# Rust type: impact_physics::driven_motion::constant_acceleration::ConstantAccelerationTrajectoryDriver
# Type category: POD
# Commit: b1b4dfd8 (dirty)
module [
    ConstantAccelerationTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.ConstantAccelerationTrajectory

## Driver for imposing a constant acceleration trajectory on a kinematic
## rigid body.
ConstantAccelerationTrajectoryDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The constant acceleration trajectory imposed on the body.
    trajectory : Setup.ConstantAccelerationTrajectory.ConstantAccelerationTrajectory,
}

## Serializes a value of [ConstantAccelerationTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, ConstantAccelerationTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(88)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.ConstantAccelerationTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [ConstantAccelerationTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result ConstantAccelerationTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 80 }) |> Setup.ConstantAccelerationTrajectory.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 88 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
