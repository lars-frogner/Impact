# Hash: c845b3719c6decd0cdbb7a0956e6388041f62be32f80a2f57b5620888a16cdf6
# Generated: 2025-12-17T23:58:42+00:00
# Rust type: impact_physics::driven_motion::circular::CircularTrajectoryDriver
# Type category: POD
# Commit: 7d41822d (dirty)
module [
    CircularTrajectoryDriver,
    write_bytes,
    from_bytes,
]

import Comp.KinematicRigidBodyID
import Setup.CircularTrajectory

## Driver for imposing a circular trajectory with constant speed on a kinematic
## rigid body.
CircularTrajectoryDriver : {
    ## The kinematic rigid body being driven.
    rigid_body_id : Comp.KinematicRigidBodyID.KinematicRigidBodyID,
    ## The circular trajectory imposed on the body.
    trajectory : Setup.CircularTrajectory.CircularTrajectory,
}

## Serializes a value of [CircularTrajectoryDriver] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, CircularTrajectoryDriver -> List U8
write_bytes = |bytes, value|
    bytes
    |> List.reserve(48)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.CircularTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [CircularTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 40 }) |> Setup.CircularTrajectory.from_bytes?,
        },
    )

test_roundtrip : {} -> Result {} _
test_roundtrip = |{}|
    bytes = List.range({ start: At 0, end: Length 48 }) |> List.map(|b| Num.to_u8(b))
    decoded = from_bytes(bytes)?
    encoded = write_bytes([], decoded)
    if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then
        Ok({})
    else
        Err(NotEqual(encoded, bytes))

expect
    result = test_roundtrip({})
    result |> Result.is_ok
