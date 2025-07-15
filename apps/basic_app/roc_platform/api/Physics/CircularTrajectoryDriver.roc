# Hash: 4942d9dfe218db8fa5bae82e988990443e455195af231e29d2e58861a9359e99
# Generated: 2025-07-15T10:43:03+00:00
# Rust type: impact_physics::driven_motion::circular::CircularTrajectoryDriver
# Type category: POD
# Commit: 189570ab (dirty)
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
    |> List.reserve(88)
    |> Comp.KinematicRigidBodyID.write_bytes(value.rigid_body_id)
    |> Setup.CircularTrajectory.write_bytes(value.trajectory)

## Deserializes a value of [CircularTrajectoryDriver] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result CircularTrajectoryDriver _
from_bytes = |bytes|
    Ok(
        {
            rigid_body_id: bytes |> List.sublist({ start: 0, len: 8 }) |> Comp.KinematicRigidBodyID.from_bytes?,
            trajectory: bytes |> List.sublist({ start: 8, len: 80 }) |> Setup.CircularTrajectory.from_bytes?,
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
